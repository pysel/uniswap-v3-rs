use alloy_primitives::U256;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum HedgerError {
    #[error("required field missing: {0}")]
    RequiredFieldMissing(String),
    #[error("invalid hedger configuration: {0}")]
    InvalidConfig(String),
    #[error("failed to initialize Hyperliquid client: {0}")]
    HyperliquidClient(String),
    #[error("out of margin: required {required}, available {available}")]
    OutOfMargin { required: U256, available: U256 },
    #[error("position watch channel closed")]
    PositionWatchClosed,
    #[error("hedge execution is not implemented for this venue yet")]
    NotImplemented,
}
