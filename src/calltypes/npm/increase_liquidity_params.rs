use alloy::primitives::{TxHash, U256};

use crate::calltypes::TransactionFuture;
use crate::errors::UniswapV3Error;
use crate::objects::Position;

use super::default_deadline;

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
    pub fn builder(position: &Position) -> IncreaseLiquidityParamsBuilder {
        IncreaseLiquidityParamsBuilder {
            token_id: position.token_id(),
            amount0_desired: None,
            amount1_desired: None,
            amount0_min: None,
            amount1_min: None,
            deadline: None,
        }
    }

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

pub struct IncreaseLiquidityParamsBuilder {
    token_id: U256,
    amount0_desired: Option<U256>,
    amount1_desired: Option<U256>,
    amount0_min: Option<U256>,
    amount1_min: Option<U256>,
    deadline: Option<U256>,
}

impl IncreaseLiquidityParamsBuilder {
    #[must_use]
    pub fn amount0_desired(mut self, amount0_desired: U256) -> Self {
        self.amount0_desired = Some(amount0_desired);
        self
    }

    #[must_use]
    pub fn amount1_desired(mut self, amount1_desired: U256) -> Self {
        self.amount1_desired = Some(amount1_desired);
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

    pub fn build(self) -> Result<IncreaseLiquidityParams, UniswapV3Error> {
        Ok(IncreaseLiquidityParams {
            tokenId: self.token_id,
            amount0Desired: self.amount0_desired.ok_or_else(|| {
                UniswapV3Error::RequiredFieldMissing("AMOUNT0_DESIRED".to_string())
            })?,
            amount1Desired: self.amount1_desired.ok_or_else(|| {
                UniswapV3Error::RequiredFieldMissing("AMOUNT1_DESIRED".to_string())
            })?,
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
