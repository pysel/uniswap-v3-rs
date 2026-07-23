use std::time::SystemTime;

use alloy::primitives::{Address, U256, aliases::U24};
use uniswap_sdk_core::prelude::BaseCurrency;

use crate::errors::UniswapV3Error;
use crate::objects::Pool;

pub use crate::objects::CreatePositionParams;

const DEFAULT_DEADLINE_FROM_NOW_SECS: u64 = 60 * 60 * 24 * 30; // 30 days from now
const DEFAULT_AMOUNT0_MIN_BPS_OF_DESIRED: u64 = 500; // 5% of desired amount
const DEFAULT_AMOUNT1_MIN_BPS_OF_DESIRED: u64 = 500; // 5% of desired amount1

impl CreatePositionParams {
    pub fn builder(pool: &Pool) -> CreatePositionParamsBuilder {
        CreatePositionParamsBuilder {
            pool: pool.clone(),
            tick_lower: None,
            tick_upper: None,
            amount0_desired: None,
            amount1_desired: None,
            amount0_min: None,
            amount1_min: None,
            recipient: None,
            deadline: None,
        }
    }

    pub fn new(
        pool: &Pool,
        tick_lower: i32,
        tick_upper: i32,
        amount0_desired: U256,
        amount1_desired: U256,
        amount0_min: U256,
        amount1_min: U256,
        recipient: Address,
        deadline: U256,
    ) -> Result<Self, UniswapV3Error> {
        let (tick_lower, tick_upper) = pool.validate_ticks(tick_lower, tick_upper)?;

        Ok(Self {
            token0: pool.token0().address(),
            token1: pool.token1().address(),
            fee: U24::from(pool.fee()),
            tickLower: tick_lower,
            tickUpper: tick_upper,
            amount0Desired: amount0_desired,
            amount1Desired: amount1_desired,
            amount0Min: amount0_min,
            amount1Min: amount1_min,
            recipient,
            deadline,
        })
    }
}

pub struct CreatePositionParamsBuilder {
    pool: Pool,
    tick_lower: Option<i32>,
    tick_upper: Option<i32>,
    amount0_desired: Option<U256>,
    amount1_desired: Option<U256>,
    amount0_min: Option<U256>,
    amount1_min: Option<U256>,
    recipient: Option<Address>,
    deadline: Option<U256>,
}

impl CreatePositionParamsBuilder {
    pub fn tick_lower(mut self, tick_lower: i32) -> Self {
        self.tick_lower = Some(tick_lower);
        self
    }

    pub fn tick_upper(mut self, tick_upper: i32) -> Self {
        self.tick_upper = Some(tick_upper);
        self
    }

    pub fn amount0_desired(mut self, amount0_desired: U256) -> Self {
        self.amount0_desired = Some(amount0_desired);
        self
    }

    pub fn amount1_desired(mut self, amount1_desired: U256) -> Self {
        self.amount1_desired = Some(amount1_desired);
        self
    }

    pub fn amount0_min(mut self, amount0_min: U256) -> Self {
        self.amount0_min = Some(amount0_min);
        self
    }

    pub fn amount1_min(mut self, amount1_min: U256) -> Self {
        self.amount1_min = Some(amount1_min);
        self
    }

    pub fn recipient(mut self, recipient: Address) -> Self {
        self.recipient = Some(recipient);
        self
    }

    pub fn deadline(mut self, deadline: U256) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn then_default(mut self) -> Self {
        let bps_denominator = U256::from(10_000);
        if let (None, Some(amount0_desired)) = (self.amount0_min, self.amount0_desired) {
            let amount0_min_bps_of_desired = U256::from(DEFAULT_AMOUNT0_MIN_BPS_OF_DESIRED);
            self.amount0_min = Some(amount0_desired * amount0_min_bps_of_desired / bps_denominator);
        }

        if let (None, Some(amount1_desired)) = (self.amount1_min, self.amount1_desired) {
            let amount1_min_bps_of_desired = U256::from(DEFAULT_AMOUNT1_MIN_BPS_OF_DESIRED);
            self.amount1_min = Some(amount1_desired * amount1_min_bps_of_desired / bps_denominator);
        }

        if self.deadline.is_none() {
            if let Ok(now) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                let deadline = now.as_secs() + DEFAULT_DEADLINE_FROM_NOW_SECS;
                self.deadline = Some(U256::from(deadline));
            }
        }

        self
    }

    pub fn build(self) -> Result<CreatePositionParams, UniswapV3Error> {
        let tick_lower = self
            .tick_lower
            .ok_or_else(|| UniswapV3Error::Invalid("TICK_LOWER".to_string()))?;
        let tick_upper = self
            .tick_upper
            .ok_or_else(|| UniswapV3Error::Invalid("TICK_UPPER".to_string()))?;
        let amount0_desired = self
            .amount0_desired
            .ok_or_else(|| UniswapV3Error::Invalid("AMOUNT0_DESIRED".to_string()))?;
        let amount1_desired = self
            .amount1_desired
            .ok_or_else(|| UniswapV3Error::Invalid("AMOUNT1_DESIRED".to_string()))?;
        let amount0_min = self
            .amount0_min
            .ok_or_else(|| UniswapV3Error::Invalid("AMOUNT0_MIN".to_string()))?;
        let amount1_min = self
            .amount1_min
            .ok_or_else(|| UniswapV3Error::Invalid("AMOUNT1_MIN".to_string()))?;
        let recipient = self
            .recipient
            .ok_or_else(|| UniswapV3Error::Invalid("RECIPIENT".to_string()))?;
        let deadline = self
            .deadline
            .ok_or_else(|| UniswapV3Error::Invalid("DEADLINE".to_string()))?;

        let (tick_lower, tick_upper) = self.pool.validate_ticks(tick_lower, tick_upper)?;

        Ok(CreatePositionParams {
            token0: self.pool.token0().address(),
            token1: self.pool.token1().address(),
            fee: U24::from(self.pool.fee()),
            tickLower: tick_lower,
            tickUpper: tick_upper,
            amount0Desired: amount0_desired,
            amount1Desired: amount1_desired,
            amount0Min: amount0_min,
            amount1Min: amount1_min,
            recipient,
            deadline,
        })
    }
}
