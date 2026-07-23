use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum UniswapV3Error {
    #[error("failed to build a client: {0}")]
    BuildError(String),
    #[error("RPC call failed: {0}")]
    RpcError(String),
    #[error("invalid argument: {0}")]
    Invalid(String),
    #[error("tick not found within bps")]
    TickNotFoundWithinBps,
    #[error("invalid pool: {0}")]
    InvalidPool(String),
    #[error("core error: {0}")]
    Core(#[from] uniswap_sdk_core::prelude::Error),
    #[error("math error: {0}")]
    Math(String),
}

impl From<uniswap_v3_math::error::UniswapV3MathError> for UniswapV3Error {
    fn from(error: uniswap_v3_math::error::UniswapV3MathError) -> Self {
        UniswapV3Error::Math(error.to_string())
    }
}
