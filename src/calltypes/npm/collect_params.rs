use alloy::primitives::{Address, TxHash, U256};

use crate::calltypes::TransactionFuture;
use crate::errors::UniswapV3Error;
use crate::objects::Position;

pub use crate::objects::CollectParams;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollectPositionResult {
    pub amount0: U256,
    pub amount1: U256,
}

pub struct CollectPositionResponse {
    pub tx_hash: TxHash,
    pub amounts: TransactionFuture<CollectPositionResult>,
}

impl CollectParams {
    #[must_use]
    pub fn builder(position: &Position) -> CollectParamsBuilder {
        CollectParamsBuilder {
            token_id: position.token_id(),
            recipient: None,
            amount0_max: None,
            amount1_max: None,
        }
    }

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

pub struct CollectParamsBuilder {
    token_id: U256,
    recipient: Option<Address>,
    amount0_max: Option<u128>,
    amount1_max: Option<u128>,
}

impl CollectParamsBuilder {
    #[must_use]
    pub fn recipient(mut self, recipient: Address) -> Self {
        self.recipient = Some(recipient);
        self
    }

    #[must_use]
    pub fn amount0_max(mut self, amount0_max: u128) -> Self {
        self.amount0_max = Some(amount0_max);
        self
    }

    #[must_use]
    pub fn amount1_max(mut self, amount1_max: u128) -> Self {
        self.amount1_max = Some(amount1_max);
        self
    }

    /// Sets missing max amounts to `u128::MAX` (collect all owed).
    #[must_use]
    pub fn then_default(mut self) -> Self {
        if self.amount0_max.is_none() {
            self.amount0_max = Some(u128::MAX);
        }
        if self.amount1_max.is_none() {
            self.amount1_max = Some(u128::MAX);
        }
        self
    }

    pub fn build(self) -> Result<CollectParams, UniswapV3Error> {
        Ok(CollectParams {
            tokenId: self.token_id,
            recipient: self
                .recipient
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("RECIPIENT".to_string()))?,
            amount0Max: self
                .amount0_max
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("AMOUNT0_MAX".to_string()))?,
            amount1Max: self
                .amount1_max
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("AMOUNT1_MAX".to_string()))?,
        })
    }
}
