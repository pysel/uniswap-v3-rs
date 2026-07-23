use alloy::primitives::{TxHash, U256};

use crate::calltypes::TransactionFuture;
use crate::objects::Position;

pub use crate::objects::IncreaseLiquidityParams;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IncreaseLiquidityResult {
    pub liquidity: u128,
    pub amount0: U256,
    pub amount1: U256,
}

pub struct IncreaseLiquidityResponse {
    pub tx_hash: TxHash,
    pub result: TransactionFuture<IncreaseLiquidityResult>,
}

impl IncreaseLiquidityParams {
    #[must_use]
    pub fn new(
        position: &Position,
        amount0_desired: U256,
        amount1_desired: U256,
        amount0_min: U256,
        amount1_min: U256,
        deadline: U256,
    ) -> Self {
        Self {
            tokenId: position.token_id(),
            amount0Desired: amount0_desired,
            amount1Desired: amount1_desired,
            amount0Min: amount0_min,
            amount1Min: amount1_min,
            deadline,
        }
    }
}
