use alloy::{
    network::Ethereum,
    primitives::{Log, U256},
    providers::PendingTransactionBuilder,
    sol_types::SolEvent,
};

use crate::{calltypes::TransactionFuture, errors::UniswapV3Error};

use super::super::PoolContract;

async fn confirmed_swaps(
    pending: PendingTransactionBuilder<Ethereum>,
) -> Result<Vec<Log<PoolContract::Swap>>, UniswapV3Error> {
    let receipt = pending
        .get_receipt()
        .await
        .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

    if !receipt.status() {
        return Err(UniswapV3Error::RpcError(
            "swap transaction reverted".to_owned(),
        ));
    }

    Ok(receipt
        .logs()
        .iter()
        .filter_map(|log| PoolContract::Swap::decode_log(log.as_ref()).ok())
        .collect())
}

pub(super) fn amount_out_future(
    pending: PendingTransactionBuilder<Ethereum>,
) -> TransactionFuture<U256> {
    Box::pin(async move {
        let swaps = confirmed_swaps(pending).await?;
        let swap = swaps
            .last()
            .ok_or_else(|| UniswapV3Error::RpcError("swap event not found".to_owned()))?;

        [swap.data.amount0, swap.data.amount1]
            .into_iter()
            .find(|amount| amount.is_negative())
            .map(|amount| amount.unsigned_abs())
            .ok_or_else(|| UniswapV3Error::RpcError("swap output amount not found".to_owned()))
    })
}

pub(super) fn amount_in_future(
    pending: PendingTransactionBuilder<Ethereum>,
) -> TransactionFuture<U256> {
    Box::pin(async move {
        let swaps = confirmed_swaps(pending).await?;
        let swap = swaps
            .first()
            .ok_or_else(|| UniswapV3Error::RpcError("swap event not found".to_owned()))?;

        [swap.data.amount0, swap.data.amount1]
            .into_iter()
            .find(|amount| !amount.is_negative())
            .map(|amount| amount.unsigned_abs())
            .ok_or_else(|| UniswapV3Error::RpcError("swap input amount not found".to_owned()))
    })
}
