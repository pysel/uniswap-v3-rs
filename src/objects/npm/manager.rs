use alloy::{
    network::Ethereum,
    primitives::{
        Address, Bytes, U256,
        aliases::{U24, U160},
    },
    providers::{PendingTransactionBuilder, Provider},
    sol_types::SolCall,
};
use uniswap_sdk_core::prelude::{Error, NONFUNGIBLE_POSITION_MANAGER_ADDRESSES};

use crate::{
    calltypes::{
        BurnPositionResponse, ClosePositionResponse, CollectPositionResponse,
        CreateAndInitializePoolResponse, CreatePositionResponse, DecreaseLiquidityResponse,
        IncreaseLiquidityResponse,
    },
    errors::UniswapV3Error,
};

use super::{
    burn_result, close_position_result, collect_position_result, create_pool_result,
    create_position_result, decrease_liquidity_result, increase_liquidity_result,
};
use crate::objects::{
    CollectParams, CreatePositionParams, DecreaseLiquidityParams, IncreaseLiquidityParams,
    NpmContract, Position, PositionsReturn,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonfungiblePositionManager {
    chain_id: u64,
    address: Address,
}

impl NonfungiblePositionManager {
    pub(crate) fn new(chain_id: u64, address: Address) -> Result<Self, Error> {
        if chain_id == 0 {
            return Err(Error::Invalid("CHAIN_ID"));
        }

        Ok(Self { chain_id, address })
    }

    pub fn from_chain(chain_id: u64) -> Option<Self> {
        NONFUNGIBLE_POSITION_MANAGER_ADDRESSES
            .get(&chain_id)
            .copied()
            .and_then(|address| Self::new(chain_id, address).ok())
    }

    #[must_use]
    pub const fn chain_id(&self) -> u64 {
        self.chain_id
    }

    #[must_use]
    pub const fn address(&self) -> Address {
        self.address
    }

    pub(crate) async fn position<P: Provider>(
        &self,
        provider: &P,
        token_id: U256,
    ) -> Result<Position, UniswapV3Error> {
        Position::from_manager_token_id(*self, token_id, provider).await
    }

    pub(crate) async fn positions<P: Provider>(
        &self,
        provider: &P,
        token_id: U256,
    ) -> Result<PositionsReturn, UniswapV3Error> {
        NpmContract::new(self.address, provider)
            .positions(token_id)
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
    }

    pub(crate) async fn owner_of<P: Provider>(
        &self,
        provider: &P,
        token_id: U256,
    ) -> Result<Address, UniswapV3Error> {
        NpmContract::new(self.address, provider)
            .ownerOf(token_id)
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
    }

    pub(crate) async fn balance_of<P: Provider>(
        &self,
        provider: &P,
        owner: Address,
    ) -> Result<U256, UniswapV3Error> {
        NpmContract::new(self.address, provider)
            .balanceOf(owner)
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
    }

    pub(crate) async fn token_of_owner_by_index<P: Provider>(
        &self,
        provider: &P,
        owner: Address,
        index: U256,
    ) -> Result<U256, UniswapV3Error> {
        NpmContract::new(self.address, provider)
            .tokenOfOwnerByIndex(owner, index)
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
    }

    pub(crate) async fn collectable_amounts<P: Provider>(
        &self,
        provider: &P,
        token_id: U256,
    ) -> Result<(U256, U256), UniswapV3Error> {
        let owner = self.owner_of(provider, token_id).await?;
        let params = CollectParams::collect_all(token_id, owner);
        let amounts = NpmContract::new(self.address, provider)
            .collect(params)
            .from(owner)
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok((amounts.amount0, amounts.amount1))
    }

    pub(crate) async fn mint<P: Provider>(
        &self,
        provider: &P,
        params: CreatePositionParams,
        value: U256,
    ) -> Result<CreatePositionResponse, UniswapV3Error> {
        let pending = if value.is_zero() {
            self.send_mint(provider, params, value).await?
        } else {
            let contract = NpmContract::new(self.address, provider);
            let data = vec![
                contract.mint(params).calldata().clone(),
                contract.refundETH().calldata().clone(),
            ];
            self.send_multicall(provider, data, value).await?
        };

        Ok(CreatePositionResponse {
            tx_hash: *pending.tx_hash(),
            position: create_position_result(pending, self.address),
        })
    }

    pub(crate) async fn increase_liquidity<P: Provider>(
        &self,
        provider: &P,
        params: IncreaseLiquidityParams,
        value: U256,
    ) -> Result<IncreaseLiquidityResponse, UniswapV3Error> {
        let pending = if value.is_zero() {
            self.send_increase_liquidity(provider, params, value)
                .await?
        } else {
            let contract = NpmContract::new(self.address, provider);
            let data = vec![
                contract.increaseLiquidity(params).calldata().clone(),
                contract.refundETH().calldata().clone(),
            ];
            self.send_multicall(provider, data, value).await?
        };

        Ok(IncreaseLiquidityResponse {
            tx_hash: *pending.tx_hash(),
            result: increase_liquidity_result(pending, self.address),
        })
    }

    pub(crate) async fn decrease_liquidity<P: Provider>(
        &self,
        provider: &P,
        params: DecreaseLiquidityParams,
    ) -> Result<DecreaseLiquidityResponse, UniswapV3Error> {
        let pending = self.send_decrease_liquidity(provider, params).await?;

        Ok(DecreaseLiquidityResponse {
            tx_hash: *pending.tx_hash(),
            amounts: decrease_liquidity_result(pending, self.address),
        })
    }

    pub(crate) async fn collect<P: Provider>(
        &self,
        provider: &P,
        params: CollectParams,
    ) -> Result<CollectPositionResponse, UniswapV3Error> {
        let pending = NpmContract::new(self.address, provider)
            .collect(params)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(CollectPositionResponse {
            tx_hash: *pending.tx_hash(),
            amounts: collect_position_result(pending, self.address),
        })
    }

    pub(crate) async fn burn<P: Provider>(
        &self,
        provider: &P,
        token_id: U256,
    ) -> Result<BurnPositionResponse, UniswapV3Error> {
        let pending = NpmContract::new(self.address, provider)
            .burn(token_id)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(BurnPositionResponse {
            tx_hash: *pending.tx_hash(),
            confirmation: burn_result(pending),
        })
    }

    pub(crate) async fn close<P: Provider>(
        &self,
        provider: &P,
        data: Vec<Bytes>,
    ) -> Result<ClosePositionResponse, UniswapV3Error> {
        let pending = self.send_multicall(provider, data, U256::ZERO).await?;

        Ok(ClosePositionResponse {
            tx_hash: *pending.tx_hash(),
            amounts: close_position_result(pending, self.address),
        })
    }

    pub(crate) async fn create_and_initialize_pool_if_necessary<P: Provider>(
        &self,
        provider: &P,
        token0: Address,
        token1: Address,
        fee: u32,
        sqrt_price_x96: U160,
        value: U256,
    ) -> Result<CreateAndInitializePoolResponse, UniswapV3Error> {
        let pending = NpmContract::new(self.address, provider)
            .createAndInitializePoolIfNecessary(token0, token1, U24::from(fee), sqrt_price_x96)
            .value(value)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(CreateAndInitializePoolResponse {
            tx_hash: *pending.tx_hash(),
            pool: create_pool_result(
                pending,
                provider.root().clone(),
                self.address,
                token0,
                token1,
                fee,
                sqrt_price_x96,
            ),
        })
    }

    pub(crate) fn decrease_liquidity_calldata(&self, params: DecreaseLiquidityParams) -> Bytes {
        NpmContract::decreaseLiquidityCall { params }
            .abi_encode()
            .into()
    }

    pub(crate) fn collect_calldata(&self, params: CollectParams) -> Bytes {
        NpmContract::collectCall { params }.abi_encode().into()
    }

    pub(crate) fn burn_calldata(&self, token_id: U256) -> Bytes {
        NpmContract::burnCall { tokenId: token_id }
            .abi_encode()
            .into()
    }

    async fn send_mint<P: Provider>(
        &self,
        provider: &P,
        params: CreatePositionParams,
        value: U256,
    ) -> Result<PendingTransactionBuilder<Ethereum>, UniswapV3Error> {
        NpmContract::new(self.address, provider)
            .mint(params)
            .value(value)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
    }

    async fn send_increase_liquidity<P: Provider>(
        &self,
        provider: &P,
        params: IncreaseLiquidityParams,
        value: U256,
    ) -> Result<PendingTransactionBuilder<Ethereum>, UniswapV3Error> {
        NpmContract::new(self.address, provider)
            .increaseLiquidity(params)
            .value(value)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
    }

    async fn send_decrease_liquidity<P: Provider>(
        &self,
        provider: &P,
        params: DecreaseLiquidityParams,
    ) -> Result<PendingTransactionBuilder<Ethereum>, UniswapV3Error> {
        NpmContract::new(self.address, provider)
            .decreaseLiquidity(params)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
    }

    async fn send_multicall<P: Provider>(
        &self,
        provider: &P,
        data: Vec<Bytes>,
        value: U256,
    ) -> Result<PendingTransactionBuilder<Ethereum>, UniswapV3Error> {
        NpmContract::new(self.address, provider)
            .multicall(data)
            .value(value)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
    }
}
