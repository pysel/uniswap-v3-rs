use alloy::primitives::Address;
use tokio::task::JoinHandle;

use crate::{
    calltypes::BPS,
    client::UniswapV3Client,
    errors::UniswapV3Error,
    strategies::{
        Strategy,
        errors::StrategyError,
        price_source::{BinancePriceSource, PriceSource},
    },
};

// pub enum ConstantWindowState {
//     Starting,

// }

pub struct ConstantWindowStrategy<T> {
    // A handle of a running strategy.
    handle: Option<JoinHandle<Result<(), StrategyError>>>,

    // Strategy parameters.
    length_below_mid: BPS,
    length_above_mid: BPS,
    rebalance_below_threshold: BPS,
    rebalance_above_threshold: BPS,
    price_source: T,
}

impl<T> ConstantWindowStrategy<T>
where
    T: PriceSource,
{
    #[must_use]
    pub fn new(
        length_below_mid: BPS,
        length_above_mid: BPS,
        rebalance_below_threshold: BPS,
        rebalance_above_threshold: BPS,
        price_source: T,
    ) -> Self {
        Self {
            length_below_mid,
            length_above_mid,
            rebalance_below_threshold,
            rebalance_above_threshold,
            price_source,
            handle: None,
        }
    }
}

impl<T> Strategy for ConstantWindowStrategy<T>
where
    T: PriceSource + 'static,
{
    fn run(&mut self, _client: UniswapV3Client, _pool: Address) -> Result<(), StrategyError> {
        if self.handle.is_some() {
            return Err(StrategyError::AlreadyRunning);
        }

        let _ = (
            self.length_below_mid,
            self.length_above_mid,
            self.rebalance_below_threshold,
            self.rebalance_above_threshold,
            &self.price_source,
        );

        self.handle = Some(tokio::spawn(async move { unimplemented!() }));
        Ok(())
    }

    fn abort(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

pub struct ConstantWindowStrategyBuilder<T = ()> {
    length_below_mid: Option<BPS>,
    length_above_mid: Option<BPS>,
    rebalance_below_threshold: Option<BPS>,
    rebalance_above_threshold: Option<BPS>,
    price_source: Option<T>,
}

impl ConstantWindowStrategy<()> {
    #[must_use]
    pub fn builder() -> ConstantWindowStrategyBuilder {
        ConstantWindowStrategyBuilder {
            length_below_mid: None,
            length_above_mid: None,
            rebalance_below_threshold: None,
            rebalance_above_threshold: None,
            price_source: None,
        }
    }
}

impl<T> ConstantWindowStrategyBuilder<T> {
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
}

impl ConstantWindowStrategyBuilder<()> {
    #[must_use]
    pub fn with_binance_price_source(self) -> ConstantWindowStrategyBuilder<BinancePriceSource> {
        ConstantWindowStrategyBuilder {
            length_below_mid: self.length_below_mid,
            length_above_mid: self.length_above_mid,
            rebalance_below_threshold: self.rebalance_below_threshold,
            rebalance_above_threshold: self.rebalance_above_threshold,
            price_source: Some(BinancePriceSource::new()),
        }
    }
}

impl<T> ConstantWindowStrategyBuilder<T>
where
    T: PriceSource,
{
    pub fn build(self) -> Result<ConstantWindowStrategy<T>, UniswapV3Error> {
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
        let price_source = self
            .price_source
            .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("PRICE_SOURCE".to_string()))?;

        Ok(ConstantWindowStrategy {
            length_below_mid,
            length_above_mid,
            rebalance_below_threshold,
            rebalance_above_threshold,
            price_source,
            handle: None,
        })
    }
}
