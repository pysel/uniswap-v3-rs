use alloy::{
    network::{Ethereum, EthereumWallet, NetworkWallet},
    primitives::Address,
    providers::{DynProvider, Provider, ProviderBuilder},
};
use uniswap_sdk_core::entities::Token;

#[cfg(any(feature = "swap", feature = "positions"))]
use alloy::primitives::{TxHash, U256};

#[cfg(feature = "positions")]
use alloy::primitives::aliases::U160;

use crate::{
    errors::UniswapV3Error,
    objects::{Factory, Pool, SwapRouter},
};

#[cfg(feature = "swap")]
use crate::objects::{
    ExactInputParams, ExactInputSingleParams, ExactOutputParams, ExactOutputSingleParams,
};

#[cfg(feature = "positions")]
use crate::{
    calltypes::ClosePositionParams,
    objects::{
        CollectParams, DecreaseLiquidityParams, IncreaseLiquidityParams, CreatePositionParams,
        NonfungiblePositionManager, Position,
    },
};

#[derive(Clone, Debug)]
pub struct UniswapV3Client {
    rpc_url: String,
    provider: DynProvider,
    wallet: Option<EthereumWallet>,
    #[cfg(feature = "swap")]
    swap_router: Option<SwapRouter>,
    #[cfg(feature = "positions")]
    position_manager: Option<NonfungiblePositionManager>,
    factory: Factory,
}

impl UniswapV3Client {
    pub fn builder() -> UniswapV3ClientBuilder {
        UniswapV3ClientBuilder::default()
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    pub fn provider(&self) -> &DynProvider {
        &self.provider
    }

    pub fn wallet(&self) -> Option<&EthereumWallet> {
        self.wallet.as_ref()
    }

    pub fn factory(&self) -> &Factory {
        &self.factory
    }

    pub fn swap_router(&self) -> Option<&SwapRouter> {
        self.swap_router.as_ref()
    }

    #[cfg(feature = "positions")]
    pub fn position_manager(&self) -> Option<&NonfungiblePositionManager> {
        self.position_manager.as_ref()
    }

    pub async fn get_chain_id(&self) -> Result<u64, UniswapV3Error> {
        self.provider
            .get_chain_id()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
    }

    pub fn signer_address(&self) -> Option<Address> {
        self.wallet
            .as_ref()
            .map(NetworkWallet::<Ethereum>::default_signer_address)
    }

    pub fn factory_address(&self) -> Address {
        self.factory.address()
    }

    pub async fn get_pool(
        &self,
        token0: Token,
        token1: Token,
        fee: u32,
    ) -> Result<Pool, UniswapV3Error> {
        self.factory.pool(token0, token1, fee, &self.provider).await
    }

    #[cfg(feature = "positions")]
    pub async fn get_position(&self, token_id: U256) -> Result<Position, UniswapV3Error> {
        self.require_position_manager()?
            .position(&self.provider, token_id)
            .await
    }

    #[cfg(feature = "positions")]
    pub async fn get_position_count(&self, owner: Address) -> Result<U256, UniswapV3Error> {
        self.require_position_manager()?
            .balance_of(&self.provider, owner)
            .await
    }

    #[cfg(feature = "positions")]
    pub async fn get_position_id(
        &self,
        owner: Address,
        index: U256,
    ) -> Result<U256, UniswapV3Error> {
        self.require_position_manager()?
            .token_of_owner_by_index(&self.provider, owner, index)
            .await
    }

    #[cfg(feature = "positions")]
    pub async fn get_positions(&self, owner: Address) -> Result<Vec<Position>, UniswapV3Error> {
        let count = self.get_position_count(owner).await?;
        let count = usize::try_from(count).map_err(|error| {
            UniswapV3Error::RpcError(format!("position count too large: {error}"))
        })?;
        let mut positions = Vec::with_capacity(count);

        for index in 0..count {
            let token_id = self.get_position_id(owner, U256::from(index)).await?;
            positions.push(self.get_position(token_id).await?);
        }

        Ok(positions)
    }

    #[cfg(feature = "positions")]
    pub async fn create_position(
        &self,
        params: CreatePositionParams,
        value: Option<U256>,
    ) -> Result<TxHash, UniswapV3Error> {
        self.require_position_manager()?
            .mint(&self.provider, params, value.unwrap_or_default())
            .await
    }

    #[cfg(feature = "positions")]
    pub async fn increase_position_liquidity(
        &self,
        params: IncreaseLiquidityParams,
        value: Option<U256>,
    ) -> Result<TxHash, UniswapV3Error> {
        self.require_position_manager()?
            .increase_liquidity(&self.provider, params, value.unwrap_or_default())
            .await
    }

    #[cfg(feature = "positions")]
    pub async fn decrease_position_liquidity(
        &self,
        params: DecreaseLiquidityParams,
    ) -> Result<TxHash, UniswapV3Error> {
        self.require_position_manager()?
            .decrease_liquidity(&self.provider, params)
            .await
    }

    #[cfg(feature = "positions")]
    pub async fn collect_position(&self, params: CollectParams) -> Result<TxHash, UniswapV3Error> {
        self.require_position_manager()?
            .collect(&self.provider, params)
            .await
    }

    #[cfg(feature = "positions")]
    pub async fn burn_position(&self, position: &Position) -> Result<TxHash, UniswapV3Error> {
        self.ensure_position_manager(position)?;
        self.require_position_manager()?
            .burn(&self.provider, position.token_id())
            .await
    }

    #[cfg(feature = "positions")]
    pub async fn create_and_initialize_pool_if_necessary(
        &self,
        token0: Address,
        token1: Address,
        fee: u32,
        sqrt_price_x96: U160,
        value: Option<U256>,
    ) -> Result<TxHash, UniswapV3Error> {
        self.require_position_manager()?
            .create_and_initialize_pool_if_necessary(
                &self.provider,
                token0,
                token1,
                fee,
                sqrt_price_x96,
                value.unwrap_or_default(),
            )
            .await
    }

    #[cfg(feature = "positions")]
    pub async fn close_position(
        &self,
        position: &Position,
        params: ClosePositionParams,
    ) -> Result<TxHash, UniswapV3Error> {
        self.ensure_position_manager(position)?;
        let manager = self.require_position_manager()?;
        let liquidity = position.liquidity(&self.provider).await?;
        let mut data = Vec::with_capacity(if liquidity == 0 { 2 } else { 3 });

        if liquidity != 0 {
            data.push(
                manager.decrease_liquidity_calldata(DecreaseLiquidityParams::from_token_id(
                    position.token_id(),
                    liquidity,
                    params.amount0_min(),
                    params.amount1_min(),
                    params.deadline(),
                )),
            );
        }

        data.push(manager.collect_calldata(CollectParams::collect_all(
            position.token_id(),
            params.recipient(),
        )));
        data.push(manager.burn_calldata(position.token_id()));

        manager.multicall(&self.provider, data, U256::ZERO).await
    }

    #[cfg(feature = "swap")]
    pub async fn swap_exact_input(
        &self,
        params: ExactInputParams,
        value: Option<U256>,
    ) -> Result<TxHash, UniswapV3Error> {
        let value = value.unwrap_or(U256::from(0));
        self.require_swap_router()?
            .exact_input(&self.provider, params, value)
            .await
    }

    #[cfg(feature = "swap")]
    pub async fn swap_exact_output(
        &self,
        params: ExactOutputParams,
        value: Option<U256>,
    ) -> Result<TxHash, UniswapV3Error> {
        let value = value.unwrap_or(U256::from(0));
        self.require_swap_router()?
            .exact_output(&self.provider, params, value)
            .await
    }

    #[cfg(feature = "swap")]
    pub async fn swap_exact_input_single(
        &self,
        params: ExactInputSingleParams,
        value: Option<U256>,
    ) -> Result<TxHash, UniswapV3Error> {
        let value = value.unwrap_or(U256::from(0));
        self.require_swap_router()?
            .exact_input_single(&self.provider, params, value)
            .await
    }

    #[cfg(feature = "swap")]
    pub async fn swap_exact_output_single(
        &self,
        params: ExactOutputSingleParams,
        value: Option<U256>,
    ) -> Result<TxHash, UniswapV3Error> {
        let value = value.unwrap_or(U256::from(0));
        self.require_swap_router()?
            .exact_output_single(&self.provider, params, value)
            .await
    }

    #[cfg(feature = "swap")]
    fn require_swap_router(&self) -> Result<&SwapRouter, UniswapV3Error> {
        self.swap_router
            .as_ref()
            .ok_or_else(|| UniswapV3Error::BuildError("no swap router for this chain".to_string()))
    }

    #[cfg(feature = "positions")]
    fn require_position_manager(&self) -> Result<&NonfungiblePositionManager, UniswapV3Error> {
        self.position_manager.as_ref().ok_or_else(|| {
            UniswapV3Error::BuildError("no nonfungible position manager for this chain".to_string())
        })
    }

    #[cfg(feature = "positions")]
    fn ensure_position_manager(&self, position: &Position) -> Result<(), UniswapV3Error> {
        let manager = self.require_position_manager()?;
        if position.manager() != *manager {
            return Err(UniswapV3Error::BuildError(
                "position belongs to a different nonfungible position manager".to_string(),
            ));
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct UniswapV3ClientBuilder {
    rpc_url: Option<String>,
    wallet: Option<EthereumWallet>,
}

impl UniswapV3ClientBuilder {
    pub fn rpc_url(mut self, rpc_url: impl Into<String>) -> Self {
        self.rpc_url = Some(rpc_url.into());
        self
    }

    pub fn signer(mut self, signer: impl Into<EthereumWallet>) -> Self {
        self.wallet = Some(signer.into());
        self
    }

    pub fn wallet(mut self, wallet: EthereumWallet) -> Self {
        self.wallet = Some(wallet);
        self
    }

    pub async fn build(self) -> Result<UniswapV3Client, UniswapV3Error> {
        let rpc_url = self
            .rpc_url
            .filter(|rpc_url| !rpc_url.trim().is_empty())
            .ok_or_else(|| UniswapV3Error::BuildError("RPC URL is required".to_string()))?;

        let url = rpc_url
            .parse()
            .map_err(|_| UniswapV3Error::BuildError("Invalid RPC URL".to_string()))?;

        let provider = match &self.wallet {
            Some(wallet) => ProviderBuilder::new()
                .wallet(wallet.clone())
                .connect_http(url)
                .erased(),
            None => ProviderBuilder::default().connect_http(url).erased(),
        };

        let chain_id = provider
            .get_chain_id()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        let factory = Factory::from_chain(chain_id).ok_or_else(|| {
            UniswapV3Error::BuildError(format!("no V3 factory for chain id {chain_id}"))
        })?;
        let swap_router = SwapRouter::from_chain(chain_id);
        #[cfg(feature = "positions")]
        let position_manager = NonfungiblePositionManager::from_chain(chain_id);

        Ok(UniswapV3Client {
            rpc_url,
            provider,
            wallet: self.wallet,
            factory,
            swap_router,
            #[cfg(feature = "positions")]
            position_manager,
        })
    }
}
