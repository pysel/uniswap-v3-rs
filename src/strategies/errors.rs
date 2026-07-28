use alloy::primitives::{Address, U256};
use thiserror::Error;

use crate::{errors::UniswapV3Error, strategies::price_source::PriceSourceError};

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StrategyError {
    #[error(
        "invalid price: token USD prices must be finite and strictly positive, and token1 price must be non-zero"
    )]
    InvalidPrice,
    #[error("invalid strategy configuration: {0}")]
    InvalidConfig(String),
    #[error("signer is required")]
    SignerRequired,
    #[error("nonfungible position manager is required")]
    PositionManagerRequired,
    #[error("insufficient balance for token {token}: required {required}, available {available}")]
    InsufficientBalance {
        token: Address,
        required: U256,
        available: U256,
    },
    #[error("insufficient allowance for token {token}: required {required}, available {available}")]
    InsufficientAllowance {
        token: Address,
        required: U256,
        available: U256,
    },
    #[error("price source channel closed")]
    PriceSourceClosed,
    #[error(transparent)]
    PriceSource(#[from] PriceSourceError),
    #[error(transparent)]
    UniswapV3(#[from] UniswapV3Error),
}
