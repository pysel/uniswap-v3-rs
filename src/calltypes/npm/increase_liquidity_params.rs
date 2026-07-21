use alloy::primitives::U256;

use crate::objects::Position;

pub use crate::objects::IncreaseLiquidityParams;

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
