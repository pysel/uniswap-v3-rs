use alloy::primitives::{
    Address, U256,
    aliases::{U24, U160},
};
use uniswap_sdk_core::prelude::{BaseCurrency, Error};

use super::Path;

use crate::objects::{
    ExactInputParams, ExactInputSingleParams, ExactOutputParams, ExactOutputSingleParams,
};

impl ExactInputParams {
    pub fn new(
        path: &Path,
        recipient: Address,
        amount_in: U256,
        amount_out_minimum: U256,
    ) -> Result<Self, Error> {
        Ok(Self {
            path: path.bytes(false)?,
            recipient,
            amountIn: amount_in,
            amountOutMinimum: amount_out_minimum,
        })
    }
}

impl ExactInputSingleParams {
    pub fn new(
        path: &Path,
        recipient: Address,
        amount_in: U256,
        amount_out_minimum: U256,
        sqrt_price_limit_x96: U160,
    ) -> Result<Self, Error> {
        let (token_in, token_out, fee) = path.single_hop()?;

        Ok(Self {
            tokenIn: token_in.address(),
            tokenOut: token_out.address(),
            fee: U24::from(fee),
            recipient,
            amountIn: amount_in,
            amountOutMinimum: amount_out_minimum,
            sqrtPriceLimitX96: sqrt_price_limit_x96,
        })
    }
}

impl ExactOutputParams {
    pub fn new(
        path: &Path,
        recipient: Address,
        amount_out: U256,
        amount_in_maximum: U256,
    ) -> Result<Self, Error> {
        Ok(Self {
            path: path.bytes(true)?,
            recipient,
            amountOut: amount_out,
            amountInMaximum: amount_in_maximum,
        })
    }
}

impl ExactOutputSingleParams {
    pub fn new(
        path: &Path,
        recipient: Address,
        amount_out: U256,
        amount_in_maximum: U256,
        sqrt_price_limit_x96: U160,
    ) -> Result<Self, Error> {
        let (token_in, token_out, fee) = path.single_hop()?;

        Ok(Self {
            tokenIn: token_in.address(),
            tokenOut: token_out.address(),
            fee: U24::from(fee),
            recipient,
            amountOut: amount_out,
            amountInMaximum: amount_in_maximum,
            sqrtPriceLimitX96: sqrt_price_limit_x96,
        })
    }
}
