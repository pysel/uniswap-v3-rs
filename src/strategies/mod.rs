use alloy::primitives::Address;
use tokio::task::JoinHandle;

use crate::client::UniswapV3Client;

mod constant_window;
mod errors;
pub mod price_source;
pub(crate) mod utils;

pub use constant_window::{ConstantWindowStrategy, ConstantWindowStrategyBuilder};
pub use errors::StrategyError;
pub use price_source::{BinancePriceSource, PriceSource, PriceSourceError, StablePriceSource};

pub trait Strategy: Send + 'static {
    /// Spawns the strategy task. The returned handle completes with that task's
    /// [`StrategyError`] when one occurs; callers can also `abort()` the handle.
    fn run(
        &mut self,
        client: UniswapV3Client,
        pool: Address,
    ) -> Result<JoinHandle<Result<(), StrategyError>>, StrategyError>;
}
