use alloy::primitives::{Address, U256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosePositionParams {
    recipient: Address,
    amount0_min: U256,
    amount1_min: U256,
    deadline: U256,
}

impl ClosePositionParams {
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
