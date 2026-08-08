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
    #[error("Uniswap position read failed: {0}")]
    Uniswap(String),
    #[error("token symbol is missing")]
    TokenSymbolMissing,
    #[error("unsupported Hyperliquid asset: {0}")]
    UnsupportedAsset(String),
    #[error("numeric conversion failed: {0}")]
    NumericConversion(String),
    #[error("Hyperliquid account data unavailable or invalid: {0}")]
    AccountData(String),
    /// Required and available margin are USD/USDC at 6-decimal atomic precision.
    #[error("out of margin: required {required}, available {available}")]
    OutOfMargin { required: U256, available: U256 },
    #[error("required leverage {required} exceeds venue max leverage {max}")]
    VenueLeverageExceeded { required: String, max: u32 },
    #[error("Hyperliquid order rejected or not filled: {0}")]
    OrderFailed(String),
    #[error("failed to close Hyperliquid hedge: {0}")]
    CleanupFailed(String),
    #[error("unexpected long Hyperliquid position for asset {0}")]
    UnexpectedLong(String),
    #[error("hedger stopped, positions cleaned up")]
    Stopped,
    #[error("hedger stopped, positions failed to cleanup, reason: {0}")]
    StoppedCleanupFailed(String),
}
