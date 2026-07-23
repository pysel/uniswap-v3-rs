use alloy::{
    network::Ethereum,
    primitives::{
        Address, Log,
        aliases::{U24, U160},
    },
    providers::{PendingTransactionBuilder, RootProvider},
    rpc::types::TransactionReceipt,
    sol_types::SolEvent,
};

use crate::{
    calltypes::{
        ClosePositionResult, CollectPositionResult, CreatePositionResult, DecreaseLiquidityResult,
        IncreaseLiquidityResult, TransactionFuture,
    },
    errors::UniswapV3Error,
};

use super::super::NpmContract;

pub(super) fn create_position_result(
    pending: PendingTransactionBuilder<Ethereum>,
    manager: Address,
) -> TransactionFuture<CreatePositionResult> {
    Box::pin(async move {
        let receipt = confirmed_receipt(pending).await?;
        let event =
            extract_manager_event::<NpmContract::IncreaseLiquidity>(&receipt, manager)?.data;
        Ok(CreatePositionResult {
            token_id: event.tokenId,
            liquidity: event.liquidity,
            amount0: event.amount0,
            amount1: event.amount1,
        })
    })
}

pub(super) fn increase_liquidity_result(
    pending: PendingTransactionBuilder<Ethereum>,
    manager: Address,
) -> TransactionFuture<IncreaseLiquidityResult> {
    Box::pin(async move {
        let receipt = confirmed_receipt(pending).await?;
        let event =
            extract_manager_event::<NpmContract::IncreaseLiquidity>(&receipt, manager)?.data;
        Ok(IncreaseLiquidityResult {
            liquidity: event.liquidity,
            amount0: event.amount0,
            amount1: event.amount1,
        })
    })
}

pub(super) fn decrease_liquidity_result(
    pending: PendingTransactionBuilder<Ethereum>,
    manager: Address,
) -> TransactionFuture<DecreaseLiquidityResult> {
    Box::pin(async move {
        let receipt = confirmed_receipt(pending).await?;
        let event =
            extract_manager_event::<NpmContract::DecreaseLiquidity>(&receipt, manager)?.data;
        Ok(DecreaseLiquidityResult {
            amount0: event.amount0,
            amount1: event.amount1,
        })
    })
}

pub(super) fn collect_position_result(
    pending: PendingTransactionBuilder<Ethereum>,
    manager: Address,
) -> TransactionFuture<CollectPositionResult> {
    Box::pin(async move {
        let receipt = confirmed_receipt(pending).await?;
        let event = extract_manager_event::<NpmContract::Collect>(&receipt, manager)?.data;
        Ok(CollectPositionResult {
            amount0: event.amount0,
            amount1: event.amount1,
        })
    })
}

pub(super) fn close_position_result(
    pending: PendingTransactionBuilder<Ethereum>,
    manager: Address,
) -> TransactionFuture<ClosePositionResult> {
    Box::pin(async move {
        let receipt = confirmed_receipt(pending).await?;
        let event = extract_manager_event::<NpmContract::Collect>(&receipt, manager)?.data;
        Ok(ClosePositionResult {
            amount0: event.amount0,
            amount1: event.amount1,
        })
    })
}

pub(super) fn burn_result(pending: PendingTransactionBuilder<Ethereum>) -> TransactionFuture<()> {
    Box::pin(async move {
        confirmed_receipt(pending).await?;
        Ok(())
    })
}

pub(super) fn create_pool_result(
    pending: PendingTransactionBuilder<Ethereum>,
    provider: RootProvider<Ethereum>,
    manager: Address,
    token0: Address,
    token1: Address,
    fee: u32,
    sqrt_price_x96: U160,
) -> TransactionFuture<Address> {
    Box::pin(async move {
        confirmed_receipt(pending).await?;

        NpmContract::new(manager, &provider)
            .createAndInitializePoolIfNecessary(token0, token1, U24::from(fee), sqrt_price_x96)
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
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
            "position manager transaction reverted".to_owned(),
        ));
    }

    Ok(receipt)
}

fn extract_manager_event<E: SolEvent>(
    receipt: &TransactionReceipt,
    manager: Address,
) -> Result<Log<E>, UniswapV3Error> {
    receipt
        .logs()
        .iter()
        .filter(|log| log.address() == manager)
        .find_map(|log| E::decode_log(log.as_ref()).ok())
        .ok_or_else(|| UniswapV3Error::RpcError("position manager event not found".to_owned()))
}
