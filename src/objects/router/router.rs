use alloy::{
    primitives::{Address, U256},
    providers::Provider,
};
use uniswap_sdk_core::prelude::{Error, SWAP_ROUTER_02_ADDRESSES};

use crate::{
    calltypes::{
        ExactInputResponse, ExactInputSingleResponse, ExactOutputResponse,
        ExactOutputSingleResponse,
    },
    errors::UniswapV3Error,
    objects::send_with_gas_multiplier,
};

use super::{amount_in_future, amount_out_future};
use crate::objects::{
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
        gas_multiplier: Option<f64>,
    ) -> Result<ExactInputResponse, UniswapV3Error> {
        let contract = SwapRouterContract::new(self.address, provider);
        let call = contract.exactInput(params).value(value);
        let pending = send_with_gas_multiplier(call, gas_multiplier).await?;

        Ok(ExactInputResponse {
            tx_hash: *pending.tx_hash(),
            amount_out: amount_out_future(pending),
        })
    }

    pub(crate) async fn exact_input_single<P: Provider>(
        &self,
        provider: &P,
        params: ExactInputSingleParams,
        value: U256,
        gas_multiplier: Option<f64>,
    ) -> Result<ExactInputSingleResponse, UniswapV3Error> {
        let contract = SwapRouterContract::new(self.address, provider);
        let call = contract.exactInputSingle(params).value(value);
        let pending = send_with_gas_multiplier(call, gas_multiplier).await?;

        Ok(ExactInputSingleResponse {
            tx_hash: *pending.tx_hash(),
            amount_out: amount_out_future(pending),
        })
    }

    pub(crate) async fn exact_output<P: Provider>(
        &self,
        provider: &P,
        params: ExactOutputParams,
        value: U256,
        gas_multiplier: Option<f64>,
    ) -> Result<ExactOutputResponse, UniswapV3Error> {
        let contract = SwapRouterContract::new(self.address, provider);
        let call = contract.exactOutput(params).value(value);
        let pending = send_with_gas_multiplier(call, gas_multiplier).await?;

        Ok(ExactOutputResponse {
            tx_hash: *pending.tx_hash(),
            amount_in: amount_in_future(pending),
        })
    }

    pub(crate) async fn exact_output_single<P: Provider>(
        &self,
        provider: &P,
        params: ExactOutputSingleParams,
        value: U256,
        gas_multiplier: Option<f64>,
    ) -> Result<ExactOutputSingleResponse, UniswapV3Error> {
        let contract = SwapRouterContract::new(self.address, provider);
        let call = contract.exactOutputSingle(params).value(value);
        let pending = send_with_gas_multiplier(call, gas_multiplier).await?;

        Ok(ExactOutputSingleResponse {
            tx_hash: *pending.tx_hash(),
            amount_in: amount_in_future(pending),
        })
    }
}
