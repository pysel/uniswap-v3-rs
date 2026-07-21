use alloy::{
    network::{Ethereum, EthereumWallet, NetworkWallet},
    primitives::Address,
    providers::{DynProvider, Provider, ProviderBuilder},
};
use uniswap_sdk_core::entities::Token;

#[cfg(feature = "swap")]
use alloy::primitives::{TxHash, U256};

use crate::{
    errors::UniswapV3Error,
    objects::{Factory, Pool, SwapRouter},
};

#[cfg(feature = "swap")]
use crate::calltypes::{
    ExactInputParams, ExactInputSingleParams, ExactOutputParams, ExactOutputSingleParams,
};

#[derive(Clone, Debug)]
pub struct UniswapV3Client {
    rpc_url: String,
    provider: DynProvider,
    wallet: Option<EthereumWallet>,
    swap_router: Option<SwapRouter>,
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

        Ok(UniswapV3Client {
            rpc_url,
            provider,
            wallet: self.wallet,
            factory,
            swap_router,
        })
    }
}
