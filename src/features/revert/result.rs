use alloy::{
    network::Ethereum,
    primitives::{Address, Log, U256},
    providers::PendingTransactionBuilder,
    rpc::types::TransactionReceipt,
    sol_types::SolEvent,
};

use crate::{calltypes::TransactionFuture, errors::UniswapV3Error, objects::NpmContract};

use super::{LeverageDownResult, LeverageUpResult, VaultContract};

pub(super) fn deposit_result(
    pending: PendingTransactionBuilder<Ethereum>,
    vault: Address,
) -> TransactionFuture<U256> {
    Box::pin(async move {
        let receipt = confirmed_receipt(pending).await?;
        Ok(extract_event::<VaultContract::Add>(&receipt, vault)?
            .data
            .tokenId)
    })
}

pub(super) fn leverage_up_result(
    pending: PendingTransactionBuilder<Ethereum>,
    vault: Address,
    manager: Address,
) -> TransactionFuture<LeverageUpResult> {
    Box::pin(async move {
        let receipt = confirmed_receipt(pending).await?;
        let borrow = extract_event::<VaultContract::Borrow>(&receipt, vault)?.data;
        let increase = extract_event::<NpmContract::IncreaseLiquidity>(&receipt, manager)?.data;
        Ok(LeverageUpResult {
            borrowed: borrow.assets,
            liquidity: increase.liquidity,
            amount0: increase.amount0,
            amount1: increase.amount1,
        })
    })
}

pub(super) fn leverage_down_result(
    pending: PendingTransactionBuilder<Ethereum>,
    vault: Address,
    manager: Address,
) -> TransactionFuture<LeverageDownResult> {
    Box::pin(async move {
        let receipt = confirmed_receipt(pending).await?;
        let decrease = extract_event::<NpmContract::DecreaseLiquidity>(&receipt, manager)?.data;
        let repay = extract_event::<VaultContract::Repay>(&receipt, vault)?.data;
        Ok(LeverageDownResult {
            removed0: decrease.amount0,
            removed1: decrease.amount1,
            repaid: repay.assets,
        })
    })
}

pub(super) fn withdraw_result(
    pending: PendingTransactionBuilder<Ethereum>,
    vault: Address,
) -> TransactionFuture<U256> {
    Box::pin(async move {
        let receipt = confirmed_receipt(pending).await?;
        Ok(extract_event::<VaultContract::Remove>(&receipt, vault)?
            .data
            .tokenId)
    })
}

async fn confirmed_receipt(
    pending: PendingTransactionBuilder<Ethereum>,
) -> Result<TransactionReceipt, UniswapV3Error> {
    let receipt = pending
        .get_receipt()
        .await
        .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

    if !receipt.status() {
        return Err(UniswapV3Error::RpcError(
            "revert lend transaction reverted".to_owned(),
        ));
    }

    Ok(receipt)
}

fn extract_event<E: SolEvent>(
    receipt: &TransactionReceipt,
    emitter: Address,
) -> Result<Log<E>, UniswapV3Error> {
    receipt
        .logs()
        .iter()
        .filter(|log| log.address() == emitter)
        .find_map(|log| E::decode_log(log.as_ref()).ok())
        .ok_or_else(|| {
            UniswapV3Error::RpcError(format!("{} event not found in receipt", E::SIGNATURE))
        })
}
