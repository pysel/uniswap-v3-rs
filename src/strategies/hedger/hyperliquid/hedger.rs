use std::time::Duration;

use alloy::signers::local::PrivateKeySigner;
use hyperliquid_rust_sdk::{ExchangeClient, InfoClient};
use tokio::sync::watch;

use crate::{
    client::UniswapV3Client,
    strategies::{
        Position,
        hedger::{HedgeStatus, Hedger, HedgerError},
    },
};

/// Hyperliquid venue hedger.
///
/// This scaffold stores a [`UniswapV3Client`] (to read on-chain NPM position
/// state), initialized Hyperliquid clients, and leverage configuration, then
/// runs a position-watch loop. It does **not** place hedge orders yet: an
/// active strategy position is reported as [`HedgeStatus::Error`] with
/// [`HedgerError::NotImplemented`].
pub struct HyperliquidHedger {
    client: UniswapV3Client,
    exchange_client: ExchangeClient,
    info_client: InfoClient,
    max_leverage: f64,
    rehedge_interval_seconds: u64,
}

impl HyperliquidHedger {
    #[must_use]
    pub fn builder() -> HyperliquidHedgerBuilder {
        HyperliquidHedgerBuilder {
            client: None,
            private_key: None,
            max_leverage: None,
            rehedge_interval_seconds: None,
        }
    }

    #[must_use]
    pub fn max_leverage(&self) -> f64 {
        self.max_leverage
    }

    #[must_use]
    pub fn rehedge_interval_seconds(&self) -> u64 {
        self.rehedge_interval_seconds
    }

    async fn run_loop(
        self,
        mut position: watch::Receiver<Option<Position>>,
        hedge_tx: watch::Sender<HedgeStatus>,
    ) {
        // The initialized clients are owned by this task and will be used when
        // position-state reads and hedge execution are implemented.
        let _ = (&self.client, &self.exchange_client, &self.info_client);

        let mut interval =
            tokio::time::interval(Duration::from_secs(self.rehedge_interval_seconds));
        // The first tick completes immediately; subsequent ticks pace rechecks.
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            let next = match position.borrow().as_ref() {
                None => HedgeStatus::NoHedge,
                Some(_) => HedgeStatus::Error(HedgerError::NotImplemented),
            };

            if hedge_tx.send(next).is_err() {
                // All hedge-status receivers were dropped.
                return;
            }

            tokio::select! {
                changed = position.changed() => {
                    if changed.is_err() {
                        let _ = hedge_tx.send(HedgeStatus::Error(HedgerError::PositionWatchClosed));
                        return;
                    }
                }
                _ = interval.tick() => {
                    // Periodic recheck of the current position / hedge.
                }
            }
        }
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
}

impl Hedger for HyperliquidHedger {
    fn hedge(
        self,
        position: watch::Receiver<Option<Position>>,
    ) -> Result<watch::Receiver<HedgeStatus>, HedgerError> {
        self.validate()?;

        let (hedge_tx, hedge_rx) = watch::channel(HedgeStatus::NoHedge);

        // Spawns the loop in a new task.
        tokio::spawn(async move {
            self.run_loop(position, hedge_tx).await;
        });

        Ok(hedge_rx)
    }
}

pub struct HyperliquidHedgerBuilder {
    client: Option<UniswapV3Client>,
    private_key: Option<PrivateKeySigner>,
    max_leverage: Option<f64>,
    rehedge_interval_seconds: Option<u64>,
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
    pub fn max_leverage(mut self, max_leverage: f64) -> Self {
        self.max_leverage = Some(max_leverage);
        self
    }

    #[must_use]
    pub fn rehedge_interval_seconds(mut self, rehedge_interval_seconds: u64) -> Self {
        self.rehedge_interval_seconds = Some(rehedge_interval_seconds);
        self
    }

    pub async fn build(self) -> Result<HyperliquidHedger, HedgerError> {
        let client = self
            .client
            .ok_or_else(|| HedgerError::RequiredFieldMissing("CLIENT".to_string()))?;
        let private_key = self
            .private_key
            .ok_or_else(|| HedgerError::RequiredFieldMissing("PRIVATE_KEY".to_string()))?;
        let max_leverage = self
            .max_leverage
            .ok_or_else(|| HedgerError::RequiredFieldMissing("MAX_LEVERAGE".to_string()))?;
        let rehedge_interval_seconds = self.rehedge_interval_seconds.ok_or_else(|| {
            HedgerError::RequiredFieldMissing("REHEDGE_INTERVAL_SECONDS".to_string())
        })?;

        if !max_leverage.is_finite() || max_leverage <= 0.0 {
            return Err(HedgerError::InvalidConfig(
                "max leverage must be finite and strictly positive".to_string(),
            ));
        }
        if rehedge_interval_seconds == 0 {
            return Err(HedgerError::InvalidConfig(
                "rehedge interval must be strictly positive".to_string(),
            ));
        }

        // hyperliquid_rust_sdk 0.6 uses ethers internally. Need conversion
        let wallet = match private_key
            .to_bytes()
            .to_string()
            .parse() {
                Ok(wallet) => wallet,
                Err(_) => return Err(HedgerError::HyperliquidClient("Failed to parse private key as an ethers wallet".to_string())),
            };

        let exchange_client = ExchangeClient::new(
            None,
            wallet,
            None,
            None,
            None,
        )
        .await
        .map_err(|error| HedgerError::HyperliquidClient(error.to_string()))?;
    
        let info_client = InfoClient::new(None, None)
            .await
            .map_err(|error| HedgerError::HyperliquidClient(error.to_string()))?;

        Ok(HyperliquidHedger {
            client,
            exchange_client,
            info_client,
            max_leverage,
            rehedge_interval_seconds,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use alloy::signers::local::PrivateKeySigner;
    use alloy_primitives::{Address, U256};
    use tokio::sync::watch;

    use super::*;
    use crate::strategies::hedger::Hedger;

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

    async fn test_client() -> UniswapV3Client {
        let rpc_url = std::env::var("RPC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());
        UniswapV3Client::builder()
            .rpc_url(rpc_url)
            .build()
            .await
            .expect("UniswapV3Client for tests; start anvil or set RPC_URL")
    }

    #[test]
    fn builder_requires_client() {
        let rt = runtime();
        rt.block_on(async {
            let err = HyperliquidHedger::builder()
                .private_key(test_signer())
                .max_leverage(2.0)
                .rehedge_interval_seconds(30)
                .build()
                .await
                .err()
                .expect("client required");
            assert_eq!(
                err,
                HedgerError::RequiredFieldMissing("CLIENT".to_string())
            );
        });
    }

    #[test]
    fn builder_requires_private_key() {
        let rt = runtime();
        rt.block_on(async {
            let err = HyperliquidHedger::builder()
                .client(test_client().await)
                .max_leverage(2.0)
                .rehedge_interval_seconds(30)
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
    fn builder_requires_max_leverage() {
        let rt = runtime();
        rt.block_on(async {
            let err = HyperliquidHedger::builder()
                .client(test_client().await)
                .private_key(test_signer())
                .rehedge_interval_seconds(30)
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
    fn builder_requires_rehedge_interval_seconds() {
        let rt = runtime();
        rt.block_on(async {
            let err = HyperliquidHedger::builder()
                .client(test_client().await)
                .private_key(test_signer())
                .max_leverage(2.0)
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
    fn builder_rejects_non_positive_leverage() {
        let rt = runtime();
        rt.block_on(async {
            let client = test_client().await;
            for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
                let err = HyperliquidHedger::builder()
                    .client(client.clone())
                    .private_key(test_signer())
                    .max_leverage(bad)
                    .rehedge_interval_seconds(30)
                    .build()
                    .await
                    .err()
                    .expect("invalid leverage");
                assert!(matches!(err, HedgerError::InvalidConfig(_)));
            }
        });
    }

    #[test]
    fn builder_rejects_zero_rehedge_interval() {
        let rt = runtime();
        rt.block_on(async {
            let err = HyperliquidHedger::builder()
                .client(test_client().await)
                .private_key(test_signer())
                .max_leverage(2.0)
                .rehedge_interval_seconds(0)
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
                .max_leverage(3.5)
                .rehedge_interval_seconds(30)
                .build()
                .await
                .expect("valid builder");
            assert_eq!(hedger.max_leverage(), 3.5);
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
                .max_leverage(2.0)
                .rehedge_interval_seconds(30)
                .build()
                .await
                .expect("hedger");

            let hedge_rx = hedger.hedge(pos_rx).expect("hedge");

            // Seeded value is NoHedge; the loop also publishes NoHedge for None.
            assert_eq!(*hedge_rx.borrow(), HedgeStatus::NoHedge);

            // Wait briefly for the spawned loop to evaluate the current value.
            tokio::time::sleep(Duration::from_millis(20)).await;
            assert_eq!(*hedge_rx.borrow(), HedgeStatus::NoHedge);

            // Keep the receiver alive across the sleep so the task does not exit early.
            let _ = hedge_rx.borrow();
        });
    }

    #[test]
    #[ignore = "requires local Uniswap RPC and Hyperliquid API access"]
    fn hedge_publishes_not_implemented_for_active_position() {
        let rt = runtime();
        rt.block_on(async {
            let (pos_tx, pos_rx) = watch::channel(Some(sample_position()));
            let hedger = HyperliquidHedger::builder()
                .client(test_client().await)
                .private_key(test_signer())
                .max_leverage(2.0)
                .rehedge_interval_seconds(30)
                .build()
                .await
                .expect("hedger");

            let mut hedge_rx = hedger.hedge(pos_rx).expect("hedge");

            // Wait until the loop publishes NotImplemented (or already has).
            let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
            loop {
                if matches!(
                    &*hedge_rx.borrow(),
                    HedgeStatus::Error(HedgerError::NotImplemented)
                ) {
                    break;
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "timed out waiting for NotImplemented"
                );
                tokio::select! {
                    changed = hedge_rx.changed() => {
                        changed.expect("hedge channel open");
                    }
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {}
                }
            }

            // Transition back to no position.
            pos_tx.send(None).expect("position sender open");
            let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
            loop {
                if *hedge_rx.borrow() == HedgeStatus::NoHedge {
                    break;
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "timed out waiting for NoHedge"
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

    #[test]
    #[ignore = "requires local Uniswap RPC and Hyperliquid API access"]
    fn hedge_publishes_position_watch_closed_when_input_closes() {
        let rt = runtime();
        rt.block_on(async {
            let (pos_tx, pos_rx) = watch::channel(None);
            let hedger = HyperliquidHedger::builder()
                .client(test_client().await)
                .private_key(test_signer())
                .max_leverage(2.0)
                .rehedge_interval_seconds(30)
                .build()
                .await
                .expect("hedger");

            let mut hedge_rx = hedger.hedge(pos_rx).expect("hedge");
            drop(pos_tx);

            let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
            loop {
                if matches!(
                    &*hedge_rx.borrow(),
                    HedgeStatus::Error(HedgerError::PositionWatchClosed)
                ) {
                    break;
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "timed out waiting for PositionWatchClosed"
                );
                tokio::select! {
                    changed = hedge_rx.changed() => {
                        // Channel may still be open with the error value.
                        let _ = changed;
                    }
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {}
                }
            }
        });
    }
}
