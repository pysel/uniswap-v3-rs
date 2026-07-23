use alloy::primitives::{Address, TxHash, U256};
use uniswap_sdk_core::prelude::Error;

use crate::calltypes::{Path, TransactionFuture};

pub use crate::objects::ExactInputParams;

pub struct ExactInputResponse {
    pub tx_hash: TxHash,
    pub amount_out: TransactionFuture<U256>,
}

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
