use alloy::primitives::{TxHash, U256};

use crate::calltypes::TransactionFuture;
use crate::objects::Position;

pub use crate::objects::DecreaseLiquidityParams;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecreaseLiquidityResult {
    pub amount0: U256,
    pub amount1: U256,
}

pub struct DecreaseLiquidityResponse {
    pub tx_hash: TxHash,
    pub amounts: TransactionFuture<DecreaseLiquidityResult>,
}

impl DecreaseLiquidityParams {
    #[must_use]
    pub fn new(
        position: &Position,
        liquidity: u128,
        amount0_min: U256,
        amount1_min: U256,
        deadline: U256,
    ) -> Self {
        Self::from_token_id(
            position.token_id(),
            liquidity,
            amount0_min,
            amount1_min,
            deadline,
        )
    }

    #[must_use]
    pub fn from_token_id(
        token_id: U256,
        liquidity: u128,
        amount0_min: U256,
        amount1_min: U256,
        deadline: U256,
    ) -> Self {
        Self {
            tokenId: token_id,
            liquidity,
            amount0Min: amount0_min,
            amount1Min: amount1_min,
            deadline,
        }
    }
}
