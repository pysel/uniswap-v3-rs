use tokio::sync::watch;

use crate::strategies::Position;

mod errors;
mod hedge_status;
pub mod hyperliquid;

pub use errors::HedgerError;
pub use hedge_status::{HedgeSide, HedgeStatus};
pub use hyperliquid::{HyperliquidHedger, HyperliquidHedgerBuilder};

/// Strategy-agnostic hedge runner.
///
/// Implementations consume a strategy's position watch channel and publish the
/// latest [`HedgeStatus`] on their own watch channel. Concrete hedgers hold a
/// [`crate::client::UniswapV3Client`] (or equivalent) to read on-chain position
/// state when reacting to updates.
pub trait Hedger: Send + 'static {
    fn hedge(
        self,
        position: watch::Receiver<Option<Position>>,
    ) -> Result<watch::Receiver<HedgeStatus>, HedgerError>;
}
