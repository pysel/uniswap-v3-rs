use alloy::primitives::{Address, TxHash, U256};

use crate::calltypes::TransactionFuture;
use crate::errors::UniswapV3Error;

use super::default_deadline;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosePositionParams {
    recipient: Address,
    amount0_min: U256,
    amount1_min: U256,
    deadline: U256,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosePositionResult {
    pub amount0: U256,
    pub amount1: U256,
}

pub struct ClosePositionResponse {
    pub tx_hash: TxHash,
    pub amounts: TransactionFuture<ClosePositionResult>,
}

impl ClosePositionParams {
    #[must_use]
    pub fn builder() -> ClosePositionParamsBuilder {
        ClosePositionParamsBuilder {
            recipient: None,
            amount0_min: None,
            amount1_min: None,
            deadline: None,
        }
    }

    #[must_use]
    pub const fn new(
        recipient: Address,
        amount0_min: U256,
        amount1_min: U256,
        deadline: U256,
    ) -> Self {
        Self {
            recipient,
            amount0_min,
            amount1_min,
            deadline,
        }
    }

    #[must_use]
    pub const fn recipient(&self) -> Address {
        self.recipient
    }

    #[must_use]
    pub const fn amount0_min(&self) -> U256 {
        self.amount0_min
    }

    #[must_use]
    pub const fn amount1_min(&self) -> U256 {
        self.amount1_min
    }

    #[must_use]
    pub const fn deadline(&self) -> U256 {
        self.deadline
    }
}

pub struct ClosePositionParamsBuilder {
    recipient: Option<Address>,
    amount0_min: Option<U256>,
    amount1_min: Option<U256>,
    deadline: Option<U256>,
}

impl ClosePositionParamsBuilder {
    #[must_use]
    pub fn recipient(mut self, recipient: Address) -> Self {
        self.recipient = Some(recipient);
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

    pub fn build(self) -> Result<ClosePositionParams, UniswapV3Error> {
        Ok(ClosePositionParams {
            recipient: self
                .recipient
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("RECIPIENT".to_string()))?,
            amount0_min: self
                .amount0_min
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("AMOUNT0_MIN".to_string()))?,
            amount1_min: self
                .amount1_min
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("AMOUNT1_MIN".to_string()))?,
            deadline: self
                .deadline
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("DEADLINE".to_string()))?,
        })
    }
}
