mod errors;
mod hedge_status;
pub mod hyperliquid;
pub(crate) mod utils;

pub use errors::HedgerError;
pub use hedge_status::{Hedge, HedgeSide, HedgeStatus};
pub use hyperliquid::{BaseUrl, HyperliquidHedger, HyperliquidHedgerBuilder};

/// Strategy-agnostic hedge runner.
///
/// Implementations own a strategy position watch channel and publish the latest
/// [`HedgeStatus`] on their own watch channel.
pub trait Hedger: Send + 'static {
    fn hedge(self) -> Result<tokio::sync::watch::Receiver<HedgeStatus>, HedgerError>;
}
