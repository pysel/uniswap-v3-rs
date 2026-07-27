use std::future::Future;

use tokio::sync::watch;
use uniswap_sdk_core::prelude::Token;

mod binance;
mod errors;
mod stable;

pub use binance::BinancePriceSource;
pub use errors::PriceSourceError;
pub use stable::StablePriceSource;

pub trait PriceSource: Clone + Send + Sync {
    fn price(
        &self,
        token: Token,
    ) -> impl Future<Output = Result<watch::Receiver<f64>, PriceSourceError>> + Send;
}
