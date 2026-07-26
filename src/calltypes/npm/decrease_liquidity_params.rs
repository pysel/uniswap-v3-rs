use alloy::primitives::{TxHash, U256};

use crate::calltypes::TransactionFuture;
use crate::errors::UniswapV3Error;
use crate::objects::Position;

use super::default_deadline;

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
    pub fn builder(position: &Position) -> DecreaseLiquidityParamsBuilder {
        DecreaseLiquidityParamsBuilder {
            token_id: position.token_id(),
            liquidity: None,
            amount0_min: None,
            amount1_min: None,
            deadline: None,
        }
    }

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

pub struct DecreaseLiquidityParamsBuilder {
    token_id: U256,
    liquidity: Option<u128>,
    amount0_min: Option<U256>,
    amount1_min: Option<U256>,
    deadline: Option<U256>,
}

impl DecreaseLiquidityParamsBuilder {
    #[must_use]
    pub fn liquidity(mut self, liquidity: u128) -> Self {
        self.liquidity = Some(liquidity);
        self
    }

    #[must_use]
    pub fn amount0_min(mut self, amount0_min: U256) -> Self {
        self.amount0_min = Some(amount0_min);
        self
    }

    #[must_use]
    pub fn amount1_min(mut self, amount1_min: U256) -> Self {
        self.amount1_min = Some(amount1_min);
        self
    }

    #[must_use]
    pub fn deadline(mut self, deadline: U256) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Sets missing mins to `0` and missing deadline to ~30 days from now.
    #[must_use]
    pub fn then_default(mut self) -> Self {
        if self.amount0_min.is_none() {
            self.amount0_min = Some(U256::ZERO);
        }
        if self.amount1_min.is_none() {
            self.amount1_min = Some(U256::ZERO);
        }
        if self.deadline.is_none() {
            self.deadline = default_deadline();
        }
        self
    }

    pub fn build(self) -> Result<DecreaseLiquidityParams, UniswapV3Error> {
        Ok(DecreaseLiquidityParams {
            tokenId: self.token_id,
            liquidity: self
                .liquidity
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("LIQUIDITY".to_string()))?,
            amount0Min: self
                .amount0_min
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("AMOUNT0_MIN".to_string()))?,
            amount1Min: self
                .amount1_min
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("AMOUNT1_MIN".to_string()))?,
            deadline: self
                .deadline
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("DEADLINE".to_string()))?,
        })
    }
}
