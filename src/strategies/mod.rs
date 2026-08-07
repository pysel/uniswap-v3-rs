use alloy::primitives::Address;
use tokio::{sync::watch, task::JoinHandle};

use crate::{client::UniswapV3Client};

mod constant_window;
mod errors;
pub mod price_source;
pub(crate) mod utils;
pub mod position;

pub use constant_window::{ConstantWindowStrategy, ConstantWindowStrategyBuilder};
pub use errors::StrategyError;
pub use price_source::{BinancePriceSource, PriceSource, PriceSourceError, StablePriceSource};
pub use position::Position;

pub type StrategyHandle = JoinHandle<Result<(), StrategyError>>;

pub trait Strategy: Send + 'static {
    /// Spawns the strategy task. The returned handle completes with that task's
    /// [`StrategyError`] when one occurs; callers can also `abort()` the handle.
    fn run(
        &mut self,
        client: UniswapV3Client,
        pool: Address,
    ) -> Result<(StrategyHandle, watch::Receiver<Option<Position>>), StrategyError>;
}
