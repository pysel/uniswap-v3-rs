use alloy::primitives::Address;

use crate::client::UniswapV3Client;

mod constant_window;
mod errors;
pub mod price_source;
pub(crate) mod utils;

pub use constant_window::{ConstantWindowStrategy, ConstantWindowStrategyBuilder};
pub use errors::StrategyError;
pub use price_source::{BinancePriceSource, PriceSource, PriceSourceError, StablePriceSource};

pub trait Strategy: Send + 'static {
    fn run(&mut self, client: UniswapV3Client, pool: Address) -> Result<(), StrategyError>;

    fn abort(&mut self);
}
