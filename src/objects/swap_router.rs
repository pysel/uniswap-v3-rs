use alloy::{
    primitives::{Address, TxHash, U256},
    providers::Provider,
};
use uniswap_sdk_core::prelude::{Error, SWAP_ROUTER_02_ADDRESSES};

use crate::errors::UniswapV3Error;

use super::{
    ExactInputParams, ExactInputSingleParams, ExactOutputParams, ExactOutputSingleParams,
    SwapRouterContract,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwapRouter {
    chain_id: u64,
    address: Address,
}

impl SwapRouter {
    pub(crate) fn new(chain_id: u64, address: Address) -> Result<Self, Error> {
        if chain_id == 0 {
            return Err(Error::Invalid("CHAIN_ID"));
        }

        Ok(Self { chain_id, address })
    }

    pub fn from_chain(chain_id: u64) -> Option<Self> {
        SWAP_ROUTER_02_ADDRESSES
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

    pub(crate) async fn exact_input<P: Provider>(
        &self,
        provider: &P,
        params: ExactInputParams,
        value: U256,
    ) -> Result<TxHash, UniswapV3Error> {
        let pending = SwapRouterContract::new(self.address, provider)
            .exactInput(params)
            .value(value)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(*pending.tx_hash())
    }

    pub(crate) async fn exact_input_single<P: Provider>(
        &self,
        provider: &P,
        params: ExactInputSingleParams,
        value: U256,
    ) -> Result<TxHash, UniswapV3Error> {
        let pending = SwapRouterContract::new(self.address, provider)
            .exactInputSingle(params)
            .value(value)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(*pending.tx_hash())
    }

    pub(crate) async fn exact_output<P: Provider>(
        &self,
        provider: &P,
        params: ExactOutputParams,
        value: U256,
    ) -> Result<TxHash, UniswapV3Error> {
        let pending = SwapRouterContract::new(self.address, provider)
            .exactOutput(params)
            .value(value)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(*pending.tx_hash())
    }

    pub(crate) async fn exact_output_single<P: Provider>(
        &self,
        provider: &P,
        params: ExactOutputSingleParams,
        value: U256,
    ) -> Result<TxHash, UniswapV3Error> {
        let pending = SwapRouterContract::new(self.address, provider)
            .exactOutputSingle(params)
            .value(value)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(*pending.tx_hash())
    }
}
