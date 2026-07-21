use alloy::primitives::U256;

use crate::objects::Position;

pub use crate::objects::DecreaseLiquidityParams;

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
