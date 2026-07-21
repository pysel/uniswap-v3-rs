use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum UniswapV3Error {
    #[error("failed to build a client: {0}")]
    BuildError(String),
    #[error("RPC call failed: {0}")]
    RpcError(String),
    #[error("invalid argument: {0}")]
    Invalid(String),
    #[error("invalid pool: {0}")]
    InvalidPool(String),
    #[error("core error: {0}")]
    Core(#[from] uniswap_sdk_core::prelude::Error),
}
