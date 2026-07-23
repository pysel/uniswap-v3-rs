use alloy::primitives::{
    Address, TxHash, U256,
    aliases::{U24, U160},
};
use uniswap_sdk_core::prelude::{BaseCurrency, Error};

use crate::calltypes::{Path, TransactionFuture};

pub use crate::objects::ExactOutputSingleParams;

pub struct ExactOutputSingleResponse {
    pub tx_hash: TxHash,
    pub amount_in: TransactionFuture<U256>,
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
