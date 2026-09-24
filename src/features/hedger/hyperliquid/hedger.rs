use std::time::Duration;

use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::U256;
use ethers_signers::Signer;
use hyperliquid_rust_sdk::{
    AssetPosition, BaseUrl, ExchangeClient, ExchangeDataStatus, ExchangeResponseStatus,
    FilledOrder, InfoClient, MarketCloseParams, MarketOrderParams, UserStateResponse,
};
use tokio::{sync::watch, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uniswap_sdk_core::prelude::{BaseCurrencyCore, Token};

use crate::{
    client::UniswapV3Client,
    hedger::{
        Hedge, HedgeSide, HedgeStatus, Hedger, HedgerError,
        utils::{f64_to_raw, parse_f64, raw_to_f64, usd_to_atomic},
    },
    objects::{Pool, TokenExt},
    strategies::Position,
};

const VENUE: &str = "hyperliquid";

/// Hyperliquid rejects orders whose notional value is below this USD amount.
const MIN_ORDER_NOTIONAL_USD: f64 = 10.0;

/// Default cap on market-order slippage, as a fraction of the mid price.
const DEFAULT_SLIPPAGE: f64 = 0.01;

/// Hyperliquid venue hedger.
///
/// Watches a strategy position channel, reads live Uniswap principal exposure,
/// and maintains short perps on Hyperliquid for each non-stablecoin pool token.
pub struct HyperliquidHedger {
    client: UniswapV3Client,
    exchange_client: ExchangeClient,
    info_client: InfoClient,
    position: watch::Receiver<Option<Position>>,
    status: watch::Sender<HedgeStatus>,
    max_leverage: f64,
    slippage: f64,
    rehedge_interval_seconds: u64,
    cancellation_token: CancellationToken,
}

impl HyperliquidHedger {
    #[must_use]
    pub fn builder() -> HyperliquidHedgerBuilder {
        HyperliquidHedgerBuilder {
            client: None,
            private_key: None,
            position: None,
            base_url: BaseUrl::Mainnet,
            max_leverage: None,
            slippage: None,
            rehedge_interval_seconds: None,
            cancellation_token: None,
        }
    }

    #[must_use]
    pub fn max_leverage(&self) -> f64 {
        self.max_leverage
    }

    #[must_use]
    pub fn slippage(&self) -> f64 {
        self.slippage
    }

    #[must_use]
    pub fn rehedge_interval_seconds(&self) -> u64 {
        self.rehedge_interval_seconds
    }

    fn validate(&self) -> Result<(), HedgerError> {
        if !self.max_leverage.is_finite() || self.max_leverage <= 0.0 {
            return Err(HedgerError::InvalidConfig(
                "max leverage must be finite and strictly positive".to_string(),
            ));
        }
        if self.rehedge_interval_seconds == 0 {
            return Err(HedgerError::InvalidConfig(
                "rehedge interval must be strictly positive".to_string(),
            ));
        }

        Ok(())
    }

    fn hyperliquid_symbol(token: &Token) -> Result<String, HedgerError> {
        let symbol = token
            .symbol()
            .ok_or(HedgerError::TokenSymbolMissing)?
            .to_uppercase();
        Ok(match symbol.as_str() {
            "WETH" => "ETH".to_string(),
            "WBTC" => "BTC".to_string(),
            other => other.to_string(),
        })
    }

    fn asset_supported(&self, asset: &str) -> bool {
        self.exchange_client
            .meta
            .universe
            .iter()
            .any(|item| item.name == asset)
    }

    fn required_leverage(
        target_notional_usd: f64,
        usable_collateral_usd: f64,
    ) -> Result<f64, HedgerError> {
        if !target_notional_usd.is_finite() || target_notional_usd < 0.0 {
            return Err(HedgerError::NumericConversion(format!(
                "invalid target notional: {target_notional_usd}"
            )));
        }
        if !usable_collateral_usd.is_finite() || usable_collateral_usd < 0.0 {
            return Err(HedgerError::NumericConversion(format!(
                "invalid usable collateral: {usable_collateral_usd}"
            )));
        }
        if usable_collateral_usd == 0.0 {
            return Ok(f64::INFINITY);
        }
        Ok(target_notional_usd / usable_collateral_usd)
    }

    fn leverage_setting(required_leverage: f64) -> u32 {
        required_leverage.ceil().max(1.0) as u32
    }

    /// Base-size drift below which a rebalance is skipped: the larger of the
    /// taker-fee band and the venue minimum order value converted to base size.
    fn rebalance_dead_band(target: f64, taker_rate: f64, mid: f64) -> f64 {
        (target * taker_rate).max(MIN_ORDER_NOTIONAL_USD / mid)
    }

    fn exchange_status(
        response: ExchangeResponseStatus,
    ) -> Result<ExchangeDataStatus, HedgerError> {
        let response = match response {
            ExchangeResponseStatus::Ok(response) => response,
            ExchangeResponseStatus::Err(error) => return Err(HedgerError::OrderFailed(error)),
        };
        let mut statuses = response
            .data
            .ok_or_else(|| HedgerError::OrderFailed("missing exchange response data".to_string()))?
            .statuses
            .into_iter();
        let status = statuses.next().ok_or_else(|| {
            HedgerError::OrderFailed("exchange response contains no statuses".to_string())
        })?;
        if statuses.next().is_some() {
            return Err(HedgerError::OrderFailed(
                "exchange response contains multiple statuses".to_string(),
            ));
        }
        match status {
            ExchangeDataStatus::Error(error) => Err(HedgerError::OrderFailed(error)),
            status => Ok(status),
        }
    }

    fn ensure_success(response: ExchangeResponseStatus) -> Result<(), HedgerError> {
        // update_leverage / similar actions return `{status:ok, response:{type:default}}`
        // with no `data` statuses payload.
        match response {
            ExchangeResponseStatus::Ok(response) => match response.data {
                None => Ok(()),
                Some(data) => {
                    let mut statuses = data.statuses.into_iter();
                    let status = statuses.next().ok_or_else(|| {
                        HedgerError::OrderFailed(
                            "exchange response contains no statuses".to_string(),
                        )
                    })?;
                    if statuses.next().is_some() {
                        return Err(HedgerError::OrderFailed(
                            "exchange response contains multiple statuses".to_string(),
                        ));
                    }
                    match status {
                        ExchangeDataStatus::Success => Ok(()),
                        ExchangeDataStatus::Error(error) => Err(HedgerError::OrderFailed(error)),
                        status => Err(HedgerError::OrderFailed(format!(
                            "expected success response, received {status:?}"
                        ))),
                    }
                }
            },
            ExchangeResponseStatus::Err(error) => Err(HedgerError::OrderFailed(error)),
        }
    }

    fn filled_order(response: ExchangeResponseStatus) -> Result<FilledOrder, HedgerError> {
        match Self::exchange_status(response)? {
            ExchangeDataStatus::Filled(fill) => Ok(fill),
            status => Err(HedgerError::OrderFailed(format!(
                "expected filled order, received {status:?}"
            ))),
        }
    }

    fn rounded_order_size(&self, asset: &str, size: f64) -> Result<(f64, f64), HedgerError> {
        let decimals = self
            .exchange_client
            .meta
            .universe
            .iter()
            .find(|item| item.name == asset)
            .ok_or_else(|| HedgerError::UnsupportedAsset(asset.to_string()))?
            .sz_decimals;
        let factor = 10f64.powi(decimals as i32);
        let rounded = (size * factor).round() / factor;
        Ok((rounded, 0.5 / factor))
    }

    fn validate_fill(
        &self,
        asset: &str,
        requested_size: f64,
        fill: &FilledOrder,
    ) -> Result<(), HedgerError> {
        let (requested_size, tolerance) = self.rounded_order_size(asset, requested_size)?;
        let filled_size = parse_f64(&fill.total_sz, "filled size")?;
        if requested_size <= 0.0 || filled_size <= 0.0 {
            return Err(HedgerError::OrderFailed(format!(
                "{asset} order filled zero size"
            )));
        }
        if filled_size + tolerance < requested_size {
            return Err(HedgerError::OrderFailed(format!(
                "{asset} order partially filled: requested {requested_size}, filled {filled_size}"
            )));
        }
        Ok(())
    }

    fn taker_fee(fill: &FilledOrder, taker_rate: f64) -> Result<U256, HedgerError> {
        let filled_size = parse_f64(&fill.total_sz, "filled size")?;
        let average_price = parse_f64(&fill.avg_px, "average fill price")?;
        usd_to_atomic(filled_size * average_price * taker_rate)
    }

    async fn user_state(&self) -> Result<UserStateResponse, HedgerError> {
        self.info_client
            .user_state(self.exchange_client.wallet.address())
            .await
            .map_err(|error| HedgerError::AccountData(error.to_string()))
    }

    async fn taker_rate(&self) -> Result<f64, HedgerError> {
        let fees = self
            .info_client
            .user_fees(self.exchange_client.wallet.address())
            .await
            .map_err(|error| HedgerError::AccountData(error.to_string()))?;
        parse_f64(&fees.user_cross_rate, "user_cross_rate")
    }

    async fn mid_price(&self, asset: &str) -> Result<f64, HedgerError> {
        let mids = self
            .info_client
            .all_mids()
            .await
            .map_err(|error| HedgerError::AccountData(error.to_string()))?;
        let mid = mids
            .get(asset)
            .ok_or_else(|| HedgerError::UnsupportedAsset(asset.to_string()))?;
        parse_f64(mid, "mid price")
    }

    fn asset_position<'a>(state: &'a UserStateResponse, asset: &str) -> Option<&'a AssetPosition> {
        state
            .asset_positions
            .iter()
            .find(|position| position.position.coin == asset)
    }

    fn signed_size(state: &UserStateResponse, asset: &str) -> Result<f64, HedgerError> {
        let Some(position) = Self::asset_position(state, asset) else {
            return Ok(0.0);
        };
        parse_f64(&position.position.szi, "position szi")
    }

    fn short_size(state: &UserStateResponse, asset: &str) -> Result<f64, HedgerError> {
        let szi = Self::signed_size(state, asset)?;
        if szi > 0.0 {
            return Err(HedgerError::UnexpectedLong(asset.to_string()));
        }
        Ok(szi.abs())
    }

    fn margin_used(state: &UserStateResponse, asset: &str) -> Result<f64, HedgerError> {
        let Some(position) = Self::asset_position(state, asset) else {
            return Ok(0.0);
        };
        parse_f64(&position.position.margin_used, "margin_used")
    }

    fn venue_max_leverage(state: &UserStateResponse, asset: &str) -> Option<u32> {
        Self::asset_position(state, asset).map(|position| position.position.max_leverage)
    }

    fn withdrawable(state: &UserStateResponse) -> Result<f64, HedgerError> {
        parse_f64(&state.withdrawable, "withdrawable")
    }

    fn resolve_asset(&self, token: &Token) -> Result<Option<String>, HedgerError> {
        if token.is_stablecoin() {
            return Ok(None);
        }
        let asset = Self::hyperliquid_symbol(token)?;
        if !self.asset_supported(&asset) {
            return Err(HedgerError::UnsupportedAsset(asset));
        }
        Ok(Some(asset))
    }

    /// Closes existing Hyperliquid perps for the current pair's volatile assets.
    pub async fn pre_run(&self, token0: &Token, token1: &Token) -> Result<(), HedgerError> {
        for token in [token0, token1] {
            let Some(asset) = self.resolve_asset(token)? else {
                continue;
            };
            let state = self.user_state().await?;
            let szi = Self::signed_size(&state, &asset)?;
            if szi.abs() > 0.0 {
                info!(%asset, szi, "pre_run closing existing hyperliquid position");
                self.close(token).await.map_err(|error| {
                    HedgerError::CleanupFailed(format!("pre_run close {asset}: {error}"))
                })?;
            }
        }
        Ok(())
    }

    /// Fully closes the Hyperliquid position for `token`, returning estimated taker fees.
    pub async fn close(&self, token: &Token) -> Result<U256, HedgerError> {
        let Some(asset) = self.resolve_asset(token)? else {
            return Ok(U256::ZERO);
        };
        self.close_asset(&asset).await
    }

    async fn close_asset(&self, asset: &str) -> Result<U256, HedgerError> {
        let state = self.user_state().await?;
        let szi = Self::signed_size(&state, asset)?;
        if szi.abs() == 0.0 {
            return Ok(U256::ZERO);
        }
        let taker_rate = self.taker_rate().await?;

        let response = self
            .exchange_client
            .market_close(MarketCloseParams {
                asset,
                sz: None,
                px: None,
                slippage: Some(self.slippage),
                cloid: None,
                wallet: None,
            })
            .await
            .map_err(|error| HedgerError::CleanupFailed(error.to_string()))?;
        let fill = Self::filled_order(response)
            .and_then(|fill| {
                self.validate_fill(asset, szi.abs(), &fill)?;
                Ok(fill)
            })
            .map_err(|error| HedgerError::CleanupFailed(format!("close {asset}: {error}")))?;

        let state = self.user_state().await?;
        let remaining = Self::signed_size(&state, asset)?;
        if remaining.abs() > 0.0 {
            return Err(HedgerError::CleanupFailed(format!(
                "asset {asset} still open with szi {remaining}"
            )));
        }
        Self::taker_fee(&fill, taker_rate)
            .map_err(|error| HedgerError::CleanupFailed(format!("close {asset}: {error}")))
    }

    /// Opens, increases, or reduces a short hedge for `token` to match `target_raw_size`.
    ///
    /// Stablecoins and zero targets without an open short return [`None`]. Drift
    /// within the rebalance dead band (the larger of the taker-fee band and the
    /// venue minimum order value) leaves the current leg unchanged.
    pub async fn hedge(
        &self,
        token: &Token,
        target_raw_size: U256,
        prior_leg: Option<&Hedge>,
    ) -> Result<Option<Hedge>, HedgerError> {
        let Some(asset) = self.resolve_asset(token)? else {
            return Ok(None);
        };

        let target = raw_to_f64(target_raw_size, token.decimals())?;
        let (target, _) = self.rounded_order_size(&asset, target)?;
        let taker_rate = self.taker_rate().await?;
        let state = self.user_state().await?;
        let actual = Self::short_size(&state, &asset)?;
        let prior_fees = prior_leg.map(|hedge| hedge.fees_paid).unwrap_or(U256::ZERO);

        // Full closes bypass the dead band so a vanished target never strands a
        // dust short below the venue minimum order value.
        if target == 0.0 {
            if actual > 0.0 {
                self.close(token).await?;
            }
            return Ok(None);
        }

        let mid = self.mid_price(&asset).await?;
        let delta = target - actual;
        if delta.abs() <= Self::rebalance_dead_band(target, taker_rate, mid) {
            return if actual == 0.0 {
                Ok(None)
            } else {
                Ok(Some(Self::hedge_from_state(
                    token, &asset, &state, prior_fees,
                )?))
            };
        }

        let fee = if delta > 0.0 {
            self.increase_short(&asset, target, delta, mid, &state)
                .await?
        } else {
            self.reduce_short(&asset, (-delta).min(actual)).await?
        };
        let state = self.user_state().await?;
        let actual = Self::short_size(&state, &asset)?;
        if actual == 0.0 {
            return Err(HedgerError::OrderFailed(format!(
                "{asset} position is flat after hedging a non-zero target"
            )));
        }
        Ok(Some(Self::hedge_from_state(
            token,
            &asset,
            &state,
            prior_fees.saturating_add(fee),
        )?))
    }

    fn hedge_from_state(
        token: &Token,
        asset: &str,
        state: &UserStateResponse,
        fees_paid: U256,
    ) -> Result<Hedge, HedgerError> {
        let size_human = Self::short_size(state, asset)?;
        let margin_usd = Self::margin_used(state, asset)?;
        let size = f64_to_raw(token, size_human)?;
        Ok(Hedge::new(
            VENUE.to_string(),
            asset.to_string(),
            HedgeSide::Short,
            usd_to_atomic(margin_usd)?,
            size,
            fees_paid,
        ))
    }

    async fn increase_short(
        &self,
        asset: &str,
        target_human: f64,
        delta: f64,
        mid: f64,
        state: &UserStateResponse,
    ) -> Result<U256, HedgerError> {
        let target_notional = target_human * mid;
        let withdrawable = Self::withdrawable(state)?;
        let margin_used = Self::margin_used(state, asset)?;
        let usable = withdrawable + margin_used;
        let required = Self::required_leverage(target_notional, usable)?;

        if !required.is_finite() || required > self.max_leverage {
            let required_margin = if self.max_leverage > 0.0 {
                target_notional / self.max_leverage
            } else {
                target_notional
            };
            return Err(HedgerError::OutOfMargin {
                required: usd_to_atomic(required_margin)?,
                available: usd_to_atomic(usable)?,
            });
        }

        if let Some(venue_max) = Self::venue_max_leverage(state, asset)
            && required > f64::from(venue_max)
        {
            return Err(HedgerError::VenueLeverageExceeded {
                required: required.to_string(),
                max: venue_max,
            });
        }

        let leverage = Self::leverage_setting(required);
        let response = self
            .exchange_client
            .update_leverage(leverage, asset, true, None)
            .await
            .map_err(|error| {
                let message = error.to_string();
                if message.to_lowercase().contains("leverage") {
                    HedgerError::VenueLeverageExceeded {
                        required: required.to_string(),
                        max: leverage,
                    }
                } else {
                    HedgerError::OrderFailed(message)
                }
            })?;
        Self::ensure_success(response)?;

        let taker_rate = self.taker_rate().await?;
        let response = self
            .exchange_client
            .market_open(MarketOrderParams {
                asset,
                is_buy: false,
                sz: delta,
                px: None,
                slippage: Some(self.slippage),
                cloid: None,
                wallet: None,
            })
            .await
            .map_err(|error| HedgerError::OrderFailed(error.to_string()))?;
        let fill = Self::filled_order(response)?;
        self.validate_fill(asset, delta, &fill)?;
        Self::taker_fee(&fill, taker_rate)
    }

    async fn reduce_short(&self, asset: &str, delta: f64) -> Result<U256, HedgerError> {
        let taker_rate = self.taker_rate().await?;
        let response = self
            .exchange_client
            .market_close(MarketCloseParams {
                asset,
                sz: Some(delta),
                px: None,
                slippage: Some(self.slippage),
                cloid: None,
                wallet: None,
            })
            .await
            .map_err(|error| HedgerError::OrderFailed(error.to_string()))?;
        let fill = Self::filled_order(response)?;
        self.validate_fill(asset, delta, &fill)?;
        Self::taker_fee(&fill, taker_rate)
    }

    async fn load_exposure(
        &self,
        position: &Position,
    ) -> Result<(Token, Token, U256, U256), HedgerError> {
        if position.chain_id != self.client.chain_id() {
            return Err(HedgerError::Uniswap(format!(
                "position chain {} does not match client chain {}",
                position.chain_id,
                self.client.chain_id()
            )));
        }

        let pool = Pool::from_address(position.pool, &self.client)
            .await
            .map_err(|error| HedgerError::Uniswap(error.to_string()))?;

        let amounts = self
            .client
            .compute_current_token_amounts(position.pool, position.position_id)
            .await
            .map_err(|error| HedgerError::Uniswap(error.to_string()))?;

        Ok((
            pool.token0().clone(),
            pool.token1().clone(),
            amounts.amount0,
            amounts.amount1,
        ))
    }

    async fn cleanup(&self, status: &HedgeStatus) -> Result<(), HedgerError> {
        let mut errors = Vec::new();
        let mut closed_assets = Vec::new();
        for hedge in [&status.token0_hedge, &status.token1_hedge]
            .into_iter()
            .filter_map(|hedge| hedge.as_ref())
        {
            if closed_assets.contains(&hedge.asset) {
                continue;
            }
            match self.close_asset(&hedge.asset).await {
                Ok(_) => closed_assets.push(hedge.asset.clone()),
                Err(error) => errors.push(error.to_string()),
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(HedgerError::CleanupFailed(errors.join("; ")))
        }
    }

    async fn hedge_position(&self, position: &Position) -> HedgeStatus {
        let current = self.status.borrow().clone();
        let mut token0_hedge = current.token0_hedge.clone();
        let mut token1_hedge = current.token1_hedge.clone();
        let (token0, token1, amount0, amount1) = match self.load_exposure(position).await {
            Ok(exposure) => exposure,
            Err(error) => {
                return HedgeStatus::with_error(error, token0_hedge, token1_hedge);
            }
        };

        let token0_asset = match self.resolve_asset(&token0) {
            Ok(asset) => asset,
            Err(error) => {
                return HedgeStatus::with_error(error, token0_hedge, token1_hedge);
            }
        };
        let token1_asset = match self.resolve_asset(&token1) {
            Ok(asset) => asset,
            Err(error) => {
                return HedgeStatus::with_error(error, token0_hedge, token1_hedge);
            }
        };
        let pair_changed = token0_hedge
            .as_ref()
            .is_some_and(|hedge| Some(&hedge.asset) != token0_asset.as_ref())
            || token1_hedge
                .as_ref()
                .is_some_and(|hedge| Some(&hedge.asset) != token1_asset.as_ref());
        if pair_changed {
            return match self.cleanup(&current).await {
                Ok(()) => HedgeStatus::idle(),
                Err(error) => HedgeStatus::with_error(error, token0_hedge, token1_hedge),
            };
        }

        if current.is_idle()
            && let Err(error) = self.pre_run(&token0, &token1).await
        {
            return HedgeStatus::with_error(error, None, None);
        }
        token0_hedge = match self.hedge(&token0, amount0, token0_hedge.as_ref()).await {
            Ok(hedge) => hedge,
            Err(error) => {
                return HedgeStatus::with_error(error, token0_hedge, token1_hedge);
            }
        };
        token1_hedge = match self.hedge(&token1, amount1, token1_hedge.as_ref()).await {
            Ok(hedge) => hedge,
            Err(error) => {
                return HedgeStatus::with_error(error, token0_hedge, token1_hedge);
            }
        };

        HedgeStatus::hedged(token0_hedge, token1_hedge)
    }

    async fn run_loop(mut self) {
        let mut interval =
            tokio::time::interval(Duration::from_secs(self.rehedge_interval_seconds));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // Consume the immediate first tick so the wait path does not double-fire.
        interval.tick().await;

        loop {
            // A closed position watch is terminal absence, even if the last value remains Some.
            let current = *self.position.borrow();
            let current_status = self.status.borrow().clone();
            let next = match current {
                None => match self.cleanup(&current_status).await {
                    Ok(()) => HedgeStatus::idle(),
                    Err(error) => HedgeStatus::with_error(
                        error,
                        current_status.token0_hedge.clone(),
                        current_status.token1_hedge.clone(),
                    ),
                },
                Some(_) if current_status.has_error() => {
                    match self.cleanup(&current_status).await {
                        Ok(()) => HedgeStatus::idle(),
                        Err(error) => HedgeStatus::with_error(
                            error,
                            current_status.token0_hedge.clone(),
                            current_status.token1_hedge.clone(),
                        ),
                    }
                }
                Some(position) => self.hedge_position(&position).await,
            };

            if self.status.send(next.clone()).is_err() {
                warn!("hyperliquid hedge status channel closed");
                let _ = self.cleanup(&next).await;
                return;
            }

            if let Some(error) = &next.error {
                warn!(%error, "hyperliquid hedge cycle failed");
                continue; // if the status is error, we try to immediately recover (no waiting interval)
            }

            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    if let Err(error) = self.cleanup(&next).await {
                        warn!(%error, "hyperliquid hedge cleanup failed on cancel");
                        let _ = self.status.send(HedgeStatus::with_error(
                            HedgerError::StoppedCleanupFailed(error.to_string()),
                            next.token0_hedge.clone(),
                            next.token1_hedge.clone(),
                        ));
                        return;
                    }

                    let _ = self.status.send(HedgeStatus::with_error(
                        HedgerError::Stopped,
                        None,
                        None,
                    ));
                    return;
                }
                changed = self.position.changed() => {
                    if changed.is_err() {
                        if let Err(error) = self.cleanup(&next).await {
                            warn!(%error, "hyperliquid hedge cycle failed");
                            let _ = self.status.send(HedgeStatus::with_error(
                                HedgerError::StoppedCleanupFailed(error.to_string()),
                                next.token0_hedge.clone(),
                                next.token1_hedge.clone(),
                            ));
                            return;
                        }

                        let _ = self.status.send(HedgeStatus::with_error(
                            HedgerError::Stopped,
                            None,
                            None,
                        ));
                        return;
                    }
                }
                _ = interval.tick() => {}
            }
        }
    }
}

impl Hedger for HyperliquidHedger {
    fn hedge(self) -> Result<(JoinHandle<()>, watch::Receiver<HedgeStatus>), HedgerError> {
        self.validate()?;
        let hedge_rx = self.status.subscribe();

        let handle = tokio::spawn(async move {
            self.run_loop().await;
        });

        Ok((handle, hedge_rx))
    }
}

pub struct HyperliquidHedgerBuilder {
    client: Option<UniswapV3Client>,
    private_key: Option<PrivateKeySigner>,
    position: Option<watch::Receiver<Option<Position>>>,
    base_url: BaseUrl,
    max_leverage: Option<f64>,
    slippage: Option<f64>,
    rehedge_interval_seconds: Option<u64>,
    cancellation_token: Option<CancellationToken>,
}

impl HyperliquidHedgerBuilder {
    #[must_use]
    pub fn client(mut self, client: UniswapV3Client) -> Self {
        self.client = Some(client);
        self
    }

    #[must_use]
    pub fn private_key(mut self, private_key: PrivateKeySigner) -> Self {
        self.private_key = Some(private_key);
        self
    }

    #[must_use]
    pub fn position(mut self, position: watch::Receiver<Option<Position>>) -> Self {
        self.position = Some(position);
        self
    }

    /// Hyperliquid API endpoint. Defaults to [`BaseUrl::Mainnet`].
    #[must_use]
    pub fn base_url(mut self, base_url: BaseUrl) -> Self {
        self.base_url = base_url;
        self
    }

    #[must_use]
    pub fn max_leverage(mut self, max_leverage: f64) -> Self {
        self.max_leverage = Some(max_leverage);
        self
    }

    /// Maximum market-order slippage as a fraction of the mid price
    /// (`0.01` = 1%). Orders are submitted as IOC limits priced at
    /// `mid * (1 - slippage)` for sells and `mid * (1 + slippage)` for buys.
    /// Must be strictly between 0 and 1. Defaults to `0.01`.
    #[must_use]
    pub fn slippage(mut self, slippage: f64) -> Self {
        self.slippage = Some(slippage);
        self
    }

    #[must_use]
    pub fn rehedge_interval_seconds(mut self, rehedge_interval_seconds: u64) -> Self {
        self.rehedge_interval_seconds = Some(rehedge_interval_seconds);
        self
    }

    #[must_use]
    pub fn cancellation_token(mut self, cancellation_token: CancellationToken) -> Self {
        self.cancellation_token = Some(cancellation_token);
        self
    }

    pub async fn build(self) -> Result<HyperliquidHedger, HedgerError> {
        let client = self
            .client
            .ok_or_else(|| HedgerError::RequiredFieldMissing("CLIENT".to_string()))?;
        let private_key = self
            .private_key
            .ok_or_else(|| HedgerError::RequiredFieldMissing("PRIVATE_KEY".to_string()))?;
        let position = self
            .position
            .ok_or_else(|| HedgerError::RequiredFieldMissing("POSITION".to_string()))?;
        let max_leverage = self
            .max_leverage
            .ok_or_else(|| HedgerError::RequiredFieldMissing("MAX_LEVERAGE".to_string()))?;
        let rehedge_interval_seconds = self.rehedge_interval_seconds.ok_or_else(|| {
            HedgerError::RequiredFieldMissing("REHEDGE_INTERVAL_SECONDS".to_string())
        })?;
        let cancellation_token = self
            .cancellation_token
            .ok_or_else(|| HedgerError::RequiredFieldMissing("CANCELLATION_TOKEN".to_string()))?;
        let base_url = self.base_url;
        let slippage = self.slippage.unwrap_or(DEFAULT_SLIPPAGE);

        if !max_leverage.is_finite() || max_leverage <= 0.0 {
            return Err(HedgerError::InvalidConfig(
                "max leverage must be finite and strictly positive".to_string(),
            ));
        }
        if !slippage.is_finite() || slippage <= 0.0 || slippage >= 1.0 {
            return Err(HedgerError::InvalidConfig(
                "slippage must be a fraction strictly between 0 and 1".to_string(),
            ));
        }
        if rehedge_interval_seconds == 0 {
            return Err(HedgerError::InvalidConfig(
                "rehedge interval must be strictly positive".to_string(),
            ));
        }

        // hyperliquid_rust_sdk 0.6 uses ethers wallets internally.
        let key_hex = format!("{:#x}", private_key.to_bytes());
        let wallet = key_hex.parse().map_err(|_| {
            HedgerError::HyperliquidClient(
                "failed to parse private key as an ethers wallet".to_string(),
            )
        })?;

        let exchange_client = ExchangeClient::new(None, wallet, Some(base_url), None, None)
            .await
            .map_err(|error| HedgerError::HyperliquidClient(error.to_string()))?;

        let info_client = InfoClient::new(None, Some(base_url))
            .await
            .map_err(|error| HedgerError::HyperliquidClient(error.to_string()))?;
        let (status, _) = watch::channel(HedgeStatus::idle());

        Ok(HyperliquidHedger {
            client,
            exchange_client,
            info_client,
            position,
            status,
            max_leverage,
            slippage,
            rehedge_interval_seconds,
            cancellation_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use alloy::signers::local::PrivateKeySigner;
    use alloy_primitives::{Address, U256};
    use tokio::sync::watch;
    use tokio_util::sync::CancellationToken;
    use uniswap_sdk_core::prelude::Token;

    use super::*;
    use crate::{hedger::Hedger, objects::TokenExt};

    fn test_signer() -> PrivateKeySigner {
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            .parse()
            .expect("valid test private key")
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
    }

    fn sample_position() -> Position {
        Position::new(1.0, U256::from(1u64), Address::ZERO, 1, -100, 100)
    }

    fn no_position() -> watch::Receiver<Option<Position>> {
        watch::channel(None).1
    }

    fn token(symbol: &str, decimals: u8) -> Token {
        Token::new(1, Address::ZERO, decimals, Some(symbol.into()), None, 0, 0)
    }

    async fn test_client() -> UniswapV3Client {
        let rpc_url =
            std::env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());
        UniswapV3Client::builder()
            .rpc_url(rpc_url)
            .build()
            .await
            .expect("UniswapV3Client for tests; start anvil or set RPC_URL")
    }

    #[test]
    fn hyperliquid_symbol_maps_wrapped_assets() {
        assert_eq!(
            HyperliquidHedger::hyperliquid_symbol(&token("WETH", 18)).unwrap(),
            "ETH"
        );
        assert_eq!(
            HyperliquidHedger::hyperliquid_symbol(&token("WBTC", 8)).unwrap(),
            "BTC"
        );
        assert_eq!(
            HyperliquidHedger::hyperliquid_symbol(&token("UNI", 18)).unwrap(),
            "UNI"
        );
        assert!(matches!(
            HyperliquidHedger::hyperliquid_symbol(&Token::new(
                1,
                Address::ZERO,
                18,
                None,
                None,
                0,
                0
            )),
            Err(HedgerError::TokenSymbolMissing)
        ));
    }

    #[test]
    fn raw_human_roundtrip() {
        let weth = token("WETH", 18);
        let raw = weth.from_amount(0.01);
        let human = raw_to_f64(raw, 18).unwrap();
        assert!((human - 0.01).abs() < 1e-12);
    }

    #[test]
    fn usd_atomic_conversion() {
        assert_eq!(usd_to_atomic(1.5).unwrap(), U256::from(1_500_000u64));
        assert_eq!(usd_to_atomic(0.0135).unwrap(), U256::from(13_500u64));
    }

    #[test]
    fn leverage_from_notional_and_collateral() {
        let required = HyperliquidHedger::required_leverage(0.01 * 3000.0, 0.005 * 3000.0).unwrap();
        assert!((required - 2.0).abs() < 1e-12);
        assert_eq!(HyperliquidHedger::leverage_setting(2.0), 2);
        assert_eq!(HyperliquidHedger::leverage_setting(2.1), 3);
        assert!(
            HyperliquidHedger::required_leverage(30.0, 0.0)
                .unwrap()
                .is_infinite()
        );
    }

    #[test]
    fn dead_band_takes_max_of_fee_and_min_notional_bands() {
        // Large target: fee band (1000 * 0.00045 = 0.45 base) exceeds $10 / mid.
        let band = HyperliquidHedger::rebalance_dead_band(1000.0, 0.00045, 3000.0);
        assert!((band - 0.45).abs() < 1e-12);
        // Small target: $10 minimum order value converted at mid dominates.
        let band = HyperliquidHedger::rebalance_dead_band(0.1, 0.00045, 3000.0);
        assert!((band - 10.0 / 3000.0).abs() < 1e-12);
    }

    #[test]
    fn fees_accumulate_in_atomic_usd() {
        let open_fee = usd_to_atomic(0.0135).unwrap();
        let adjust_fee = usd_to_atomic(0.002790).unwrap();
        assert_eq!(open_fee, U256::from(13_500u64));
        assert_eq!(
            open_fee.saturating_add(adjust_fee),
            U256::from(13_500u64) + U256::from(2_790u64)
        );
    }

    #[test]
    fn hedge_status_idle_hedged_and_error() {
        assert!(HedgeStatus::idle().is_idle());
        assert!(!HedgeStatus::idle().has_error());

        let hedged = HedgeStatus::hedged(None, None);
        assert!(hedged.is_idle());

        let errored = HedgeStatus::with_error(HedgerError::Stopped, None, None);
        assert!(errored.has_error());
        assert!(!errored.is_idle());
    }

    #[test]
    fn stablecoin_tokens_are_skipped_by_is_stablecoin() {
        assert!(token("USDC", 6).is_stablecoin());
        assert!(!token("WETH", 18).is_stablecoin());
    }

    #[test]
    fn builder_requires_client() {
        let rt = runtime();
        rt.block_on(async {
            let err = HyperliquidHedger::builder()
                .private_key(test_signer())
                .position(no_position())
                .max_leverage(2.0)
                .rehedge_interval_seconds(30)
                .cancellation_token(CancellationToken::new())
                .build()
                .await
                .err()
                .expect("client required");
            assert_eq!(err, HedgerError::RequiredFieldMissing("CLIENT".to_string()));
        });
    }

    #[test]
    fn builder_requires_position() {
        let rt = runtime();
        rt.block_on(async {
            let err = HyperliquidHedger::builder()
                .private_key(test_signer())
                .max_leverage(2.0)
                .rehedge_interval_seconds(30)
                .cancellation_token(CancellationToken::new())
                .build()
                .await
                .err()
                .expect("position required");
            // CLIENT is checked first when both client and position are missing.
            assert!(matches!(
                err,
                HedgerError::RequiredFieldMissing(field)
                    if field == "CLIENT" || field == "POSITION"
            ));
        });
    }

    #[test]
    #[ignore = "requires local Uniswap RPC"]
    fn builder_requires_private_key() {
        let rt = runtime();
        rt.block_on(async {
            let err = HyperliquidHedger::builder()
                .client(test_client().await)
                .max_leverage(2.0)
                .rehedge_interval_seconds(30)
                .cancellation_token(CancellationToken::new())
                .build()
                .await
                .err()
                .expect("private key required");
            assert_eq!(
                err,
                HedgerError::RequiredFieldMissing("PRIVATE_KEY".to_string())
            );
        });
    }

    #[test]
    #[ignore = "requires local Uniswap RPC"]
    fn builder_requires_max_leverage() {
        let rt = runtime();
        rt.block_on(async {
            let err = HyperliquidHedger::builder()
                .client(test_client().await)
                .private_key(test_signer())
                .position(no_position())
                .rehedge_interval_seconds(30)
                .cancellation_token(CancellationToken::new())
                .build()
                .await
                .err()
                .expect("leverage required");
            assert_eq!(
                err,
                HedgerError::RequiredFieldMissing("MAX_LEVERAGE".to_string())
            );
        });
    }

    #[test]
    #[ignore = "requires local Uniswap RPC"]
    fn builder_requires_rehedge_interval_seconds() {
        let rt = runtime();
        rt.block_on(async {
            let err = HyperliquidHedger::builder()
                .client(test_client().await)
                .private_key(test_signer())
                .position(no_position())
                .max_leverage(2.0)
                .cancellation_token(CancellationToken::new())
                .build()
                .await
                .err()
                .expect("rehedge interval required");
            assert_eq!(
                err,
                HedgerError::RequiredFieldMissing("REHEDGE_INTERVAL_SECONDS".to_string())
            );
        });
    }

    #[test]
    #[ignore = "requires local Uniswap RPC"]
    fn builder_rejects_non_positive_leverage() {
        let rt = runtime();
        rt.block_on(async {
            let client = test_client().await;
            for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
                let err = HyperliquidHedger::builder()
                    .client(client.clone())
                    .private_key(test_signer())
                    .position(no_position())
                    .max_leverage(bad)
                    .rehedge_interval_seconds(30)
                    .cancellation_token(CancellationToken::new())
                    .build()
                    .await
                    .err()
                    .expect("invalid leverage");
                assert!(matches!(err, HedgerError::InvalidConfig(_)));
            }
        });
    }

    #[test]
    #[ignore = "requires local Uniswap RPC"]
    fn builder_rejects_invalid_slippage() {
        let rt = runtime();
        rt.block_on(async {
            let client = test_client().await;
            for bad in [0.0, -0.01, 1.0, f64::NAN, f64::INFINITY] {
                let err = HyperliquidHedger::builder()
                    .client(client.clone())
                    .private_key(test_signer())
                    .position(no_position())
                    .max_leverage(2.0)
                    .slippage(bad)
                    .rehedge_interval_seconds(30)
                    .cancellation_token(CancellationToken::new())
                    .build()
                    .await
                    .err()
                    .expect("invalid slippage");
                assert!(matches!(err, HedgerError::InvalidConfig(_)));
            }
        });
    }

    #[test]
    #[ignore = "requires local Uniswap RPC"]
    fn builder_rejects_zero_rehedge_interval() {
        let rt = runtime();
        rt.block_on(async {
            let err = HyperliquidHedger::builder()
                .client(test_client().await)
                .private_key(test_signer())
                .position(no_position())
                .max_leverage(2.0)
                .rehedge_interval_seconds(0)
                .cancellation_token(CancellationToken::new())
                .build()
                .await
                .err()
                .expect("zero rehedge interval rejected");
            assert!(matches!(err, HedgerError::InvalidConfig(_)));
        });
    }

    #[test]
    #[ignore = "requires local Uniswap RPC and Hyperliquid API access"]
    fn builder_succeeds_with_valid_config() {
        let rt = runtime();
        rt.block_on(async {
            let hedger = HyperliquidHedger::builder()
                .client(test_client().await)
                .private_key(test_signer())
                .position(no_position())
                .max_leverage(3.5)
                .rehedge_interval_seconds(30)
                .cancellation_token(CancellationToken::new())
                .build()
                .await
                .expect("valid builder");
            assert_eq!(hedger.max_leverage(), 3.5);
            assert_eq!(hedger.slippage(), DEFAULT_SLIPPAGE);
            assert_eq!(hedger.rehedge_interval_seconds(), 30);
        });
    }

    #[test]
    #[ignore = "requires local Uniswap RPC and Hyperliquid API access"]
    fn hedge_publishes_no_hedge_when_position_is_none() {
        let rt = runtime();
        rt.block_on(async {
            let (_pos_tx, pos_rx) = watch::channel(None);
            let hedger = HyperliquidHedger::builder()
                .client(test_client().await)
                .private_key(test_signer())
                .position(pos_rx)
                .max_leverage(2.0)
                .rehedge_interval_seconds(30)
                .cancellation_token(CancellationToken::new())
                .build()
                .await
                .expect("hedger");

            let (_handle, hedge_rx) = hedger.hedge().expect("hedge");
            assert!(hedge_rx.borrow().is_idle());
            tokio::time::sleep(Duration::from_millis(20)).await;
            assert!(hedge_rx.borrow().is_idle());
            let _ = hedge_rx.borrow();
        });
    }

    #[test]
    #[ignore = "requires local Uniswap RPC and Hyperliquid API access"]
    fn hedge_lifecycle_with_active_position() {
        let rt = runtime();
        rt.block_on(async {
            let (pos_tx, pos_rx) = watch::channel(Some(sample_position()));
            let hedger = HyperliquidHedger::builder()
                .client(test_client().await)
                .private_key(test_signer())
                .position(pos_rx)
                .max_leverage(2.0)
                .rehedge_interval_seconds(30)
                .cancellation_token(CancellationToken::new())
                .build()
                .await
                .expect("hedger");

            let (_handle, mut hedge_rx) = hedger.hedge().expect("hedge");
            let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
            loop {
                let status = hedge_rx.borrow().clone();
                if !status.is_idle() {
                    break;
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "timed out waiting for hedge status"
                );
                tokio::select! {
                    changed = hedge_rx.changed() => {
                        changed.expect("hedge channel open");
                    }
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {}
                }
            }

            pos_tx.send(None).expect("position sender open");
            let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
            loop {
                let status = hedge_rx.borrow().clone();
                if status.is_idle() || matches!(status.error, Some(HedgerError::CleanupFailed(_))) {
                    break;
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "timed out waiting for cleanup"
                );
                tokio::select! {
                    changed = hedge_rx.changed() => {
                        changed.expect("hedge channel open");
                    }
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {}
                }
            }
        });
    }
}
