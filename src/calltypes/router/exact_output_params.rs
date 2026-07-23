use alloy::primitives::{Address, TxHash, U256};
use uniswap_sdk_core::prelude::Error;

use crate::calltypes::{Path, TransactionFuture};

pub use crate::objects::ExactOutputParams;

pub struct ExactOutputResponse {
    pub tx_hash: TxHash,
    pub amount_in: TransactionFuture<U256>,
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
