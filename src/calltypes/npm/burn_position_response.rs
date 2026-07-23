use alloy::primitives::TxHash;

use crate::calltypes::TransactionFuture;

pub struct BurnPositionResponse {
    pub tx_hash: TxHash,
    pub confirmation: TransactionFuture<()>,
}
