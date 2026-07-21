use alloy::primitives::{Address, U256};

use crate::objects::Position;

pub use crate::objects::CollectParams;

impl CollectParams {
    #[must_use]
    pub fn new(
        position: &Position,
        recipient: Address,
        amount0_max: u128,
        amount1_max: u128,
    ) -> Self {
        Self::from_token_id(position.token_id(), recipient, amount0_max, amount1_max)
    }

    #[must_use]
    pub fn collect_all(token_id: U256, recipient: Address) -> Self {
        Self::from_token_id(token_id, recipient, u128::MAX, u128::MAX)
    }

    #[must_use]
    pub fn collect_all_for_position(position: &Position, recipient: Address) -> Self {
        Self::collect_all(position.token_id(), recipient)
    }

    #[must_use]
    pub fn from_token_id(
        token_id: U256,
        recipient: Address,
        amount0_max: u128,
        amount1_max: u128,
    ) -> Self {
        Self {
            tokenId: token_id,
            recipient,
            amount0Max: amount0_max,
            amount1Max: amount1_max,
        }
    }
}
