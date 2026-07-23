use alloy::primitives::{Address, TxHash};

use crate::calltypes::TransactionFuture;

pub struct CreateAndInitializePoolResponse {
    pub tx_hash: TxHash,
    pub pool: TransactionFuture<Address>,
}
