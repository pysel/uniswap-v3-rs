use std::future::Future;

use tokio::sync::mpsc;
use uniswap_sdk_core::prelude::Token;

mod binance;
mod errors;

pub use binance::BinancePriceSource;
pub use errors::PriceSourceError;

pub trait PriceSource: Send + Sync {
    fn price(
        &self,
        token: Token,
    ) -> impl Future<Output = Result<mpsc::UnboundedReceiver<f64>, PriceSourceError>> + Send;
}
