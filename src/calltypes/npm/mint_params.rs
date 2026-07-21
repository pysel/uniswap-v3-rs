use alloy::primitives::{
    Address, U256,
    aliases::{I24, U24},
};
use uniswap_sdk_core::prelude::{BaseCurrency, Error};

use crate::objects::Pool;

pub use crate::objects::MintParams;

impl MintParams {
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
    ) -> Result<Self, Error> {
        pool.validate_ticks(tick_lower, tick_upper)?;

        Ok(Self {
            token0: pool.token0().address(),
            token1: pool.token1().address(),
            fee: U24::from(pool.fee()),
            tickLower: I24::try_from(tick_lower).expect("tick bounds are validated"),
            tickUpper: I24::try_from(tick_upper).expect("tick bounds are validated"),
            amount0Desired: amount0_desired,
            amount1Desired: amount1_desired,
            amount0Min: amount0_min,
            amount1Min: amount1_min,
            recipient,
            deadline,
        })
    }
}
