use alloy::primitives::Address;
use alloy_primitives::U256;
use tokio::{sync::watch, task::JoinHandle};
use tracing::info;
use uniswap_sdk_core::{entities::Token, prelude::BaseCurrency};

use crate::{
    calltypes::{BPS, ClosePositionParams, CreatePositionParams},
    client::UniswapV3Client,
    errors::UniswapV3Error,
    objects::{Pool, Position as NpmPosition, TokenExt},
    strategies::{
        Strategy,
        constant_window::position::Position,
        errors::StrategyError,
        price_source::PriceSource,
        utils::{apply_bps_above, apply_bps_below},
    },
};

pub struct ConstantWindowStrategy<T0, T1> {
    // Strategy parameters.

    // USD price source for pool token0.
    price_source_token0: T0,

    // USD price source for pool token1.
    price_source_token1: T1,

    // Lower tick in basis points below true price.
    length_below_mid: BPS,

    // Upper tick in basis points above true price.
    length_above_mid: BPS,

    // If mid price moves down below rebalance threshold, the position is closed and recreated.
    rebalance_below_threshold: BPS,

    // If mid price moves above rebalance threshold, the position is closed and recreated.
    rebalance_above_threshold: BPS,

    // Fraction of wallet token0 balance to use when opening a position, in (0, 1].
    max_token0_amount_as_portfolio_fraction: f64,

    // Fraction of wallet token1 balance to use when opening a position, in (0, 1].
    max_token1_amount_as_portfolio_fraction: f64,

    // Runtime bookkeeping for the active NPM position.
    position: Option<Position>,
}

impl<T0, T1> ConstantWindowStrategy<T0, T1>
where
    T0: PriceSource,
    T1: PriceSource,
{
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        length_below_mid: BPS,
        length_above_mid: BPS,
        rebalance_below_threshold: BPS,
        rebalance_above_threshold: BPS,
        max_token0_amount_as_portfolio_fraction: f64,
        max_token1_amount_as_portfolio_fraction: f64,
        price_source_token0: T0,
        price_source_token1: T1,
    ) -> Self {
        Self {
            length_below_mid,
            length_above_mid,
            rebalance_below_threshold,
            rebalance_above_threshold,
            max_token0_amount_as_portfolio_fraction,
            max_token1_amount_as_portfolio_fraction,
            price_source_token0,
            price_source_token1,
            position: None,
        }
    }

    /// `balance * fraction`, with `fraction` in `(0, 1]`.
    pub(super) fn amount_from_portfolio_fraction(
        token: &Token,
        balance: U256,
        fraction: f64,
    ) -> Result<U256, StrategyError> {
        if !fraction.is_finite() || fraction <= 0.0 || fraction > 1.0 {
            return Err(StrategyError::InvalidConfig(
                "portfolio fraction must be finite and in (0, 1]".to_string(),
            ));
        }
        if balance.is_zero() || fraction == 1.0 {
            return Ok(balance);
        }

        // Scale with 1e18 fixed-point so typical ERC-20 balances stay exact enough.
        const SCALE: u128 = 1_000_000_000_000_000_000;
        let scaled = (fraction * SCALE as f64).round();
        if !scaled.is_finite() || scaled <= 0.0 || scaled > SCALE as f64 {
            return Err(StrategyError::InvalidConfig(
                "portfolio fraction could not be scaled".to_string(),
            ));
        }

        let amount =balance * U256::from(scaled as u128) / U256::from(SCALE);

        if amount.is_zero() {
            return Err(StrategyError::InsufficientBalance {
                token: token.address(),
                required: U256::from(1),
                available: balance,
            });
        }

        Ok(amount)
    }

    /// Reads the latest token0/token1 USD quotes and returns Uniswap-style mid:
    /// `token1` per `token0` as `price0_usd / price1_usd` (human units).
    pub fn price(
        price0: &watch::Receiver<f64>,
        price1: &watch::Receiver<f64>,
    ) -> Result<f64, StrategyError> {
        let price0_usd = *price0.borrow();
        let price1_usd = *price1.borrow();
        if !price0_usd.is_finite()
            || !price1_usd.is_finite()
            || price0_usd <= 0.0
            || price1_usd <= 0.0
        {
            return Err(StrategyError::InvalidPrice);
        }
        Ok(price0_usd / price1_usd)
    }

    /// Returns whether an NPM position belongs to the exact pool key.
    pub(super) fn matches_pool(position: &NpmPosition, pool: &Pool) -> bool {
        position.token0() == pool.token0().address()
            && position.token1() == pool.token1().address()
            && position.fee() == pool.fee()
    }

    /// Closes every owner NFT that matches the exact pool key.
    pub async fn pre_run(
        &mut self,
        client: &UniswapV3Client,
        pool: &Pool,
    ) -> Result<(), StrategyError> {
        let owner = client
            .signer_address()
            .ok_or(StrategyError::SignerRequired)?;
        let _ = client
            .position_manager()
            .ok_or(StrategyError::PositionManagerRequired)?;

        let positions = client.get_positions(owner).await?;
        for position in positions
            .into_iter()
            .filter(|position| Self::matches_pool(position, pool))
        {
            let params = ClosePositionParams::builder()
                .recipient(owner)
                .then_default()
                .build()?;
            let response = client.close_position(&position, params).await?;
            let _ = response.amounts.await?;
        }

        self.position = None;
        Ok(())
    }

    /// Validates configuration, live prices, balances, and NPM allowances.
    pub async fn validate(
        &self,
        client: &UniswapV3Client,
        pool: &Pool,
        price0: &watch::Receiver<f64>,
        price1: &watch::Receiver<f64>,
    ) -> Result<(), StrategyError> {
        if self.max_token0_amount_as_portfolio_fraction <= 0.0
            || self.max_token0_amount_as_portfolio_fraction > 1.0
        {
            return Err(StrategyError::InvalidConfig(
                "max_token0_amount_as_portfolio_fraction must be in (0, 1]".to_string(),
            ));
        }
        if self.max_token1_amount_as_portfolio_fraction <= 0.0
            || self.max_token1_amount_as_portfolio_fraction > 1.0
        {
            return Err(StrategyError::InvalidConfig(
                "max_token1_amount_as_portfolio_fraction must be in (0, 1]".to_string(),
            ));
        }

        if self.rebalance_below_threshold.get() >= self.length_below_mid.get() {
            return Err(StrategyError::InvalidConfig(
                "rebalance_below_threshold must be strictly less than length_below_mid".to_string(),
            ));
        }
        if self.rebalance_above_threshold.get() >= self.length_above_mid.get() {
            return Err(StrategyError::InvalidConfig(
                "rebalance_above_threshold must be strictly less than length_above_mid".to_string(),
            ));
        }

        let _ = Self::price(price0, price1)?;

        let owner = client
            .signer_address()
            .ok_or(StrategyError::SignerRequired)?;
        let npm = client
            .position_manager()
            .ok_or(StrategyError::PositionManagerRequired)?;

        let token0 = pool.token0();
        let token1 = pool.token1();

        let balance0 = token0.balance_of(client, owner).await?;
        let amount0max = Self::amount_from_portfolio_fraction(token0, balance0, self.max_token0_amount_as_portfolio_fraction)?;

        let balance1 = token1.balance_of(client, owner).await?;
        let amount1max = Self::amount_from_portfolio_fraction(token1, balance1, self.max_token1_amount_as_portfolio_fraction)?;

        let allowance0 = token0.allowance(client, owner, npm.address()).await?;
        if allowance0 < amount0max {
            return Err(StrategyError::InsufficientAllowance {
                token: token0.address(),
                required: amount0max,
                available: allowance0,
            });
        }

        let allowance1 = token1.allowance(client, owner, npm.address()).await?;
        if allowance1 < amount1max {
            return Err(StrategyError::InsufficientAllowance {
                token: token1.address(),
                required: amount1max,
                available: allowance1,
            });
        }

        Ok(())
    }

    /// Returns true when `current_price` is within inclusive rebalance thresholds of `open_price`.
    pub fn check_position_bounds(&self, open_price: f64, current_price: f64) -> bool {
        let lower = apply_bps_below(open_price, self.rebalance_below_threshold);
        let upper = apply_bps_above(open_price, self.rebalance_above_threshold);
        current_price >= lower && current_price <= upper
    }

    /// Derives spacing-aligned ticks centered on an external human mid price.
    pub(super) fn ticks_from_external_mid(
        pool: &Pool,
        mid: f64,
        length_below_mid: BPS,
        length_above_mid: BPS,
    ) -> Result<(i32, i32), StrategyError> {
        let lower_price = apply_bps_below(mid, length_below_mid);
        let upper_price = apply_bps_above(mid, length_above_mid);
        if lower_price <= 0.0 || upper_price <= lower_price {
            return Err(StrategyError::InvalidPrice);
        }

        let lower_tick = pool.human_price_to_tick(lower_price)?;
        let upper_tick = pool.human_price_to_tick(upper_price)?;

        pool.validate_ticks(lower_tick, upper_tick)?;
        Ok((lower_tick, upper_tick))
    }

    /// Opens a new position around the current external mid when none is tracked.
    pub async fn set_position(
        &mut self,
        client: &UniswapV3Client,
        pool: &Pool,
        price0: &watch::Receiver<f64>,
        price1: &watch::Receiver<f64>,
    ) -> Result<(), StrategyError> {
        let owner = client
            .signer_address()
            .ok_or(StrategyError::SignerRequired)?;
        let mid = Self::price(price0, price1)?;
        let (lower_tick, upper_tick) =
            Self::ticks_from_external_mid(pool, mid, self.length_below_mid, self.length_above_mid)?;

        let (balance0, balance1) = self.get_token_balances(client, pool).await?;
        let amount0 = Self::amount_from_portfolio_fraction(
            pool.token0(),
            balance0,
            self.max_token0_amount_as_portfolio_fraction,
        )?;

        let amount1 = Self::amount_from_portfolio_fraction(
            pool.token1(),
            balance1,
            self.max_token1_amount_as_portfolio_fraction,
        )?;

        let params = CreatePositionParams::builder(pool)
            .tick_lower(lower_tick)
            .tick_upper(upper_tick)
            .amount0_desired(amount0)
            .amount1_desired(amount1)
            .recipient(owner)
            .then_default()
            .build()?;

        let response = client.create_position(params, None).await?;
        let minted = response.position.await?;

        info!(
            position_id = %minted.token_id,
            open_price = mid,
            lower_tick,
            upper_tick,
            %amount0,
            %amount1,
            "constant window position opened"
        );

        self.position = Some(Position::new(mid, minted.token_id, lower_tick, upper_tick));
        Ok(())
    }

    /// Checks bounds for the tracked position; closes it when they fail.
    pub async fn check_position(
        &mut self,
        client: &UniswapV3Client,
        price0: &watch::Receiver<f64>,
        price1: &watch::Receiver<f64>,
    ) -> Result<(), StrategyError> {
        let Some(position) = self.position else {
            return Ok(());
        };

        let current = Self::price(price0, price1)?;
        if self.check_position_bounds(position.open_price, current) {
            return Ok(());
        }

        info!(
            position_id = %position.position_id,
            open_price = position.open_price,
            current_price = current,
            lower_tick = position.lower_tick,
            upper_tick = position.upper_tick,
            "constant window position rebalancing"
        );

        self.close_position(client).await
    }

    /// Closes the tracked position and clears bookkeeping after confirmation.
    pub async fn close_position(&mut self, client: &UniswapV3Client) -> Result<(), StrategyError> {
        let owner = client
            .signer_address()
            .ok_or(StrategyError::SignerRequired)?;
        let Some(position) = self.position else {
            return Ok(());
        };

        let npm_position = client.get_position(position.position_id).await?;
        let params = ClosePositionParams::builder()
            .recipient(owner)
            .then_default()
            .build()?;
        let response = client.close_position(&npm_position, params).await?;
        let _ = response.amounts.await?;

        info!(
            position_id = %position.position_id,
            open_price = position.open_price,
            lower_tick = position.lower_tick,
            upper_tick = position.upper_tick,
            "constant window position closed"
        );

        self.position = None;
        Ok(())
    }

    async fn run_loop(
        mut self,
        client: UniswapV3Client,
        pool_address: Address,
    ) -> Result<(), StrategyError> {
        let pool = Pool::from_address(pool_address, &client).await?;
        let mut price0 = self
            .price_source_token0
            .price(pool.token0().clone())
            .await?;
        let mut price1 = self
            .price_source_token1
            .price(pool.token1().clone())
            .await?;

        self.pre_run(&client, &pool).await?;
        self.validate(&client, &pool, &price0, &price1).await?;

        loop {
            if self.position.is_some() {
                self.check_position(&client, &price0, &price1).await?;
            } else {
                self.set_position(&client, &pool, &price0, &price1).await?;
            }

            tokio::select! {
                result = price0.changed() => {
                    result.map_err(|_| StrategyError::PriceSourceClosed)?;
                }
                result = price1.changed() => {
                    result.map_err(|_| StrategyError::PriceSourceClosed)?;
                }
            }
        }
    }

    async fn get_token_balances(
        &self,
        client: &UniswapV3Client,
        pool: &Pool,
    ) -> Result<(U256, U256), StrategyError> {
        let owner = client.signer_address().ok_or(StrategyError::SignerRequired)?;
        let token0 = pool.token0();
        let token1 = pool.token1();

        let balance0 = token0.balance_of(client, owner).await?;
        let balance1 = token1.balance_of(client, owner).await?;
        Ok((balance0, balance1))
    }
}

impl<T0, T1> Strategy for ConstantWindowStrategy<T0, T1>
where
    T0: PriceSource + 'static,
    T1: PriceSource + 'static,
{
    fn run(
        &mut self,
        client: UniswapV3Client,
        pool: Address,
    ) -> Result<JoinHandle<Result<(), StrategyError>>, StrategyError> {
        let worker = Self {
            price_source_token0: self.price_source_token0.clone(),
            price_source_token1: self.price_source_token1.clone(),
            length_below_mid: self.length_below_mid,
            length_above_mid: self.length_above_mid,
            rebalance_below_threshold: self.rebalance_below_threshold,
            rebalance_above_threshold: self.rebalance_above_threshold,
            max_token0_amount_as_portfolio_fraction: self.max_token0_amount_as_portfolio_fraction,
            max_token1_amount_as_portfolio_fraction: self.max_token1_amount_as_portfolio_fraction,
            position: self.position.take(),
        };

        Ok(tokio::spawn(
            async move { worker.run_loop(client, pool).await },
        ))
    }
}

pub struct ConstantWindowStrategyBuilder<T0 = (), T1 = ()> {
    length_below_mid: Option<BPS>,
    length_above_mid: Option<BPS>,
    rebalance_below_threshold: Option<BPS>,
    rebalance_above_threshold: Option<BPS>,
    max_token0_amount_as_portfolio_fraction: Option<f64>,
    max_token1_amount_as_portfolio_fraction: Option<f64>,
    price_source_token0: Option<T0>,
    price_source_token1: Option<T1>,
}

impl ConstantWindowStrategy<(), ()> {
    #[must_use]
    pub fn builder() -> ConstantWindowStrategyBuilder {
        ConstantWindowStrategyBuilder {
            length_below_mid: None,
            length_above_mid: None,
            rebalance_below_threshold: None,
            rebalance_above_threshold: None,
            max_token0_amount_as_portfolio_fraction: None,
            max_token1_amount_as_portfolio_fraction: None,
            price_source_token0: None,
            price_source_token1: None,
        }
    }
}

impl<T0, T1> ConstantWindowStrategyBuilder<T0, T1> {
    #[must_use]
    pub fn length_below_mid(mut self, length_below_mid: BPS) -> Self {
        self.length_below_mid = Some(length_below_mid);
        self
    }

    #[must_use]
    pub fn length_above_mid(mut self, length_above_mid: BPS) -> Self {
        self.length_above_mid = Some(length_above_mid);
        self
    }

    #[must_use]
    pub fn rebalance_below_threshold(mut self, rebalance_below_threshold: BPS) -> Self {
        self.rebalance_below_threshold = Some(rebalance_below_threshold);
        self
    }

    #[must_use]
    pub fn rebalance_above_threshold(mut self, rebalance_above_threshold: BPS) -> Self {
        self.rebalance_above_threshold = Some(rebalance_above_threshold);
        self
    }

    #[must_use]
    pub fn max_token0_amount_as_portfolio_fraction(
        mut self,
        max_token0_amount_as_portfolio_fraction: f64,
    ) -> Self {
        self.max_token0_amount_as_portfolio_fraction =
            Some(max_token0_amount_as_portfolio_fraction);
        self
    }

    #[must_use]
    pub fn max_token1_amount_as_portfolio_fraction(
        mut self,
        max_token1_amount_as_portfolio_fraction: f64,
    ) -> Self {
        self.max_token1_amount_as_portfolio_fraction =
            Some(max_token1_amount_as_portfolio_fraction);
        self
    }

    #[must_use]
    pub fn price_source_token0<S>(
        self,
        price_source_token0: S,
    ) -> ConstantWindowStrategyBuilder<S, T1>
    where
        S: PriceSource,
    {
        ConstantWindowStrategyBuilder {
            length_below_mid: self.length_below_mid,
            length_above_mid: self.length_above_mid,
            rebalance_below_threshold: self.rebalance_below_threshold,
            rebalance_above_threshold: self.rebalance_above_threshold,
            max_token0_amount_as_portfolio_fraction: self.max_token0_amount_as_portfolio_fraction,
            max_token1_amount_as_portfolio_fraction: self.max_token1_amount_as_portfolio_fraction,
            price_source_token0: Some(price_source_token0),
            price_source_token1: self.price_source_token1,
        }
    }

    #[must_use]
    pub fn price_source_token1<S>(
        self,
        price_source_token1: S,
    ) -> ConstantWindowStrategyBuilder<T0, S>
    where
        S: PriceSource,
    {
        ConstantWindowStrategyBuilder {
            length_below_mid: self.length_below_mid,
            length_above_mid: self.length_above_mid,
            rebalance_below_threshold: self.rebalance_below_threshold,
            rebalance_above_threshold: self.rebalance_above_threshold,
            max_token0_amount_as_portfolio_fraction: self.max_token0_amount_as_portfolio_fraction,
            max_token1_amount_as_portfolio_fraction: self.max_token1_amount_as_portfolio_fraction,
            price_source_token0: self.price_source_token0,
            price_source_token1: Some(price_source_token1),
        }
    }
}

impl<T0, T1> ConstantWindowStrategyBuilder<T0, T1>
where
    T0: PriceSource,
    T1: PriceSource,
{
    pub fn build(self) -> Result<ConstantWindowStrategy<T0, T1>, UniswapV3Error> {
        let length_below_mid = self
            .length_below_mid
            .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("LENGTH_BELOW_MID".to_string()))?;
        let length_above_mid = self
            .length_above_mid
            .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("LENGTH_ABOVE_MID".to_string()))?;
        let rebalance_below_threshold = self.rebalance_below_threshold.ok_or_else(|| {
            UniswapV3Error::RequiredFieldMissing("REBALANCE_BELOW_THRESHOLD".to_string())
        })?;
        let rebalance_above_threshold = self.rebalance_above_threshold.ok_or_else(|| {
            UniswapV3Error::RequiredFieldMissing("REBALANCE_ABOVE_THRESHOLD".to_string())
        })?;
        let max_token0_amount_as_portfolio_fraction = self
            .max_token0_amount_as_portfolio_fraction
            .ok_or_else(|| {
                UniswapV3Error::RequiredFieldMissing(
                    "MAX_TOKEN0_AMOUNT_AS_PORTFOLIO_FRACTION".to_string(),
                )
            })?;
        let max_token1_amount_as_portfolio_fraction = self
            .max_token1_amount_as_portfolio_fraction
            .ok_or_else(|| {
                UniswapV3Error::RequiredFieldMissing(
                    "MAX_TOKEN1_AMOUNT_AS_PORTFOLIO_FRACTION".to_string(),
                )
            })?;
        let price_source_token0 = self.price_source_token0.ok_or_else(|| {
            UniswapV3Error::RequiredFieldMissing("PRICE_SOURCE_TOKEN0".to_string())
        })?;
        let price_source_token1 = self.price_source_token1.ok_or_else(|| {
            UniswapV3Error::RequiredFieldMissing("PRICE_SOURCE_TOKEN1".to_string())
        })?;

        Ok(ConstantWindowStrategy::new(
            length_below_mid,
            length_above_mid,
            rebalance_below_threshold,
            rebalance_above_threshold,
            max_token0_amount_as_portfolio_fraction,
            max_token1_amount_as_portfolio_fraction,
            price_source_token0,
            price_source_token1,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;

    use alloy::primitives::{U256, address};
    use tokio::sync::watch;
    use uniswap_sdk_core::{prelude::Token, token};

    use super::*;
    use crate::{
        objects::{Factory, NonfungiblePositionManager, Pool},
        strategies::price_source::{PriceSource, PriceSourceError},
    };

    #[derive(Clone, Default)]
    struct DummyPriceSource;

    impl PriceSource for DummyPriceSource {
        #[allow(clippy::manual_async_fn)]
        fn price(
            &self,
            _token: Token,
        ) -> impl Future<Output = Result<watch::Receiver<f64>, PriceSourceError>> + Send {
            async move {
                let (tx, rx) = watch::channel(1.0);
                tokio::spawn(async move {
                    tx.closed().await;
                });
                Ok(rx)
            }
        }
    }

    fn strategy(
        length_below: u16,
        length_above: u16,
        rebalance_below: u16,
        rebalance_above: u16,
        fraction0: f64,
        fraction1: f64,
    ) -> ConstantWindowStrategy<DummyPriceSource, DummyPriceSource> {
        ConstantWindowStrategy::new(
            BPS::new(length_below),
            BPS::new(length_above),
            BPS::new(rebalance_below),
            BPS::new(rebalance_above),
            fraction0,
            fraction1,
            DummyPriceSource,
            DummyPriceSource,
        )
    }

    fn test_pool(token0_decimals: u8, token1_decimals: u8, spacing: i32) -> Pool {
        let factory = Factory::from_chain(1).expect("mainnet factory");
        let token0 = token!(
            1,
            "0000000000000000000000000000000000000001",
            token0_decimals,
            "T0",
            "token0"
        );
        let token1 = token!(
            1,
            "0000000000000000000000000000000000000002",
            token1_decimals,
            "T1",
            "token1"
        );
        Pool::new(factory, token0, token1, 500, spacing).expect("pool")
    }

    #[test]
    fn price_accepts_positive_finite_quotes() {
        let (_tx0, rx0) = watch::channel(2000.0);
        let (_tx1, rx1) = watch::channel(1.0);
        assert_eq!(
            ConstantWindowStrategy::<DummyPriceSource, DummyPriceSource>::price(&rx0, &rx1)
                .unwrap(),
            2000.0
        );
    }

    #[test]
    fn price_rejects_non_positive_or_non_finite_quotes() {
        let cases = [
            (f64::NAN, 1.0),
            (f64::INFINITY, 1.0),
            (0.0, 1.0),
            (-1.0, 1.0),
            (1.0, 0.0),
            (1.0, -1.0),
            (1.0, f64::NAN),
        ];
        for (p0, p1) in cases {
            let (_tx0, rx0) = watch::channel(p0);
            let (_tx1, rx1) = watch::channel(p1);
            assert_eq!(
                ConstantWindowStrategy::<DummyPriceSource, DummyPriceSource>::price(&rx0, &rx1),
                Err(StrategyError::InvalidPrice)
            );
        }
    }

    #[test]
    fn check_position_bounds_are_inclusive_and_asymmetric() {
        let strategy = strategy(200, 300, 100, 50, 1.0, 1.0);
        let open = 1000.0;

        assert!(strategy.check_position_bounds(open, 1000.0));
        assert!(strategy.check_position_bounds(open, 990.0)); // exactly -100 bps
        assert!(strategy.check_position_bounds(open, 1005.0)); // exactly +50 bps
        assert!(!strategy.check_position_bounds(open, 989.999));
        assert!(!strategy.check_position_bounds(open, 1005.001));
    }

    #[test]
    fn amount_from_portfolio_fraction_scales_balance() {
        let balance = U256::from(1_000_000u64);
        let token = token!(1, "0x0000000000000000000000000000000000000001", 18, "T0", "token0");
        assert_eq!(
            ConstantWindowStrategy::<DummyPriceSource, DummyPriceSource>::amount_from_portfolio_fraction(
                &token, balance, 1.0
            )
            .unwrap(),
            balance
        );
        assert_eq!(
            ConstantWindowStrategy::<DummyPriceSource, DummyPriceSource>::amount_from_portfolio_fraction(
                &token, balance, 0.5
            )
            .unwrap(),
            U256::from(500_000u64)
        );
        assert!(
            ConstantWindowStrategy::<DummyPriceSource, DummyPriceSource>::amount_from_portfolio_fraction(
                &token, balance, 0.0
            )
            .is_err()
        );
        assert!(
            ConstantWindowStrategy::<DummyPriceSource, DummyPriceSource>::amount_from_portfolio_fraction(
                &token, balance, 1.1
            )
            .is_err()
        );
    }

    #[test]
    fn ticks_from_external_mid_are_spacing_aligned_and_ordered() {
        let pool = test_pool(18, 18, 60);
        let (lower, upper) =
            ConstantWindowStrategy::<DummyPriceSource, DummyPriceSource>::ticks_from_external_mid(
                &pool,
                2000.0,
                BPS::new(100),
                BPS::new(200),
            )
            .unwrap();

        assert!(lower < upper);
        assert_eq!(lower % 60, 0);
        assert_eq!(upper % 60, 0);
    }

    #[test]
    fn ticks_from_external_mid_respect_decimal_differences() {
        let pool_eq = test_pool(18, 18, 10);
        let pool_diff = test_pool(18, 6, 10);

        let (lower_eq, upper_eq) =
            ConstantWindowStrategy::<DummyPriceSource, DummyPriceSource>::ticks_from_external_mid(
                &pool_eq,
                1.01,
                BPS::new(50),
                BPS::new(50),
            )
            .unwrap();
        let (lower_diff, upper_diff) =
            ConstantWindowStrategy::<DummyPriceSource, DummyPriceSource>::ticks_from_external_mid(
                &pool_diff,
                1.01,
                BPS::new(50),
                BPS::new(50),
            )
            .unwrap();

        // Same human mid with different decimals must not collapse to identical ticks.
        assert_ne!((lower_eq, upper_eq), (lower_diff, upper_diff));
        assert!(lower_diff < upper_diff);
        assert_eq!(lower_diff % 10, 0);
        assert_eq!(upper_diff % 10, 0);
    }

    #[test]
    fn matches_pool_requires_exact_token_and_fee_key() {
        let pool = test_pool(18, 6, 10);
        let manager = NonfungiblePositionManager::from_chain(1).expect("npm");

        let matching = NpmPosition::new_unchecked(
            manager,
            U256::from(1),
            pool.token0().address(),
            pool.token1().address(),
            pool.fee(),
            -10,
            10,
        );
        assert!(
            ConstantWindowStrategy::<DummyPriceSource, DummyPriceSource>::matches_pool(
                &matching, &pool
            )
        );

        let wrong_fee = NpmPosition::new_unchecked(
            manager,
            U256::from(2),
            pool.token0().address(),
            pool.token1().address(),
            pool.fee() + 1,
            -10,
            10,
        );
        assert!(
            !ConstantWindowStrategy::<DummyPriceSource, DummyPriceSource>::matches_pool(
                &wrong_fee, &pool
            )
        );

        let wrong_token = NpmPosition::new_unchecked(
            manager,
            U256::from(3),
            pool.token0().address(),
            address!("0x0000000000000000000000000000000000000099"),
            pool.fee(),
            -10,
            10,
        );
        assert!(
            !ConstantWindowStrategy::<DummyPriceSource, DummyPriceSource>::matches_pool(
                &wrong_token,
                &pool
            )
        );
    }

    #[test]
    fn floor_and_ceil_spacing_helpers() {
        let pool = test_pool(18, 18, 60);
        assert_eq!(pool.floor_to_spacing(61), 60);
        assert_eq!(pool.floor_to_spacing(60), 60);
        assert_eq!(pool.floor_to_spacing(-61), -120);
        assert_eq!(pool.ceil_to_spacing(61), 120);
        assert_eq!(pool.ceil_to_spacing(60), 60);
        assert_eq!(pool.ceil_to_spacing(-61), -60);
    }
}
