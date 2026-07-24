use alloy::{
    primitives::{Address, aliases::U24},
    providers::Provider,
};
use uniswap_sdk_core::prelude::{BaseCurrency, Error, QUOTER_V2_ADDRESSES};

use crate::{
    calltypes::{
        QuoteExactInputParams, QuoteExactInputResult, QuoteExactInputSingleParams,
        QuoteExactInputSingleResult, QuoteExactOutputParams, QuoteExactOutputResult,
        QuoteExactOutputSingleParams, QuoteExactOutputSingleResult,
    },
    errors::UniswapV3Error,
    objects::{
        QuoteExactInputSingleAbiParams, QuoteExactOutputSingleAbiParams, QuoterV2Contract,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuoterV2 {
    chain_id: u64,
    address: Address,
}

impl QuoterV2 {
    pub(crate) fn new(chain_id: u64, address: Address) -> Result<Self, Error> {
        if chain_id == 0 {
            return Err(Error::Invalid("CHAIN_ID"));
        }

        Ok(Self { chain_id, address })
    }

    pub fn from_chain(chain_id: u64) -> Option<Self> {
        QUOTER_V2_ADDRESSES
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

    pub(crate) async fn quote_exact_input<P: Provider>(
        &self,
        provider: &P,
        params: QuoteExactInputParams,
    ) -> Result<QuoteExactInputResult, UniswapV3Error> {
        let path = params
            .path
            .bytes(false)
            .map_err(UniswapV3Error::from)?;
        let result = QuoterV2Contract::new(self.address, provider)
            .quoteExactInput(path, params.amount_in)
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(QuoteExactInputResult {
            path: params.path,
            amount_in: params.amount_in,
            amount_out: result.amountOut,
            sqrt_price_x96_after_list: result.sqrtPriceX96AfterList,
            initialized_ticks_crossed_list: result.initializedTicksCrossedList,
            gas_estimate: result.gasEstimate,
        })
    }

    pub(crate) async fn quote_exact_input_single<P: Provider>(
        &self,
        provider: &P,
        params: QuoteExactInputSingleParams,
    ) -> Result<QuoteExactInputSingleResult, UniswapV3Error> {
        let (token_in, token_out, fee) = params
            .path
            .single_hop()
            .map_err(UniswapV3Error::from)?;
        let abi_params = QuoteExactInputSingleAbiParams {
            tokenIn: token_in.address(),
            tokenOut: token_out.address(),
            amountIn: params.amount_in,
            fee: U24::from(fee),
            sqrtPriceLimitX96: params.sqrt_price_limit_x96,
        };
        let result = QuoterV2Contract::new(self.address, provider)
            .quoteExactInputSingle(abi_params)
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(QuoteExactInputSingleResult {
            path: params.path,
            amount_in: params.amount_in,
            sqrt_price_limit_x96: params.sqrt_price_limit_x96,
            amount_out: result.amountOut,
            sqrt_price_x96_after: result.sqrtPriceX96After,
            initialized_ticks_crossed: result.initializedTicksCrossed,
            gas_estimate: result.gasEstimate,
        })
    }

    pub(crate) async fn quote_exact_output<P: Provider>(
        &self,
        provider: &P,
        params: QuoteExactOutputParams,
    ) -> Result<QuoteExactOutputResult, UniswapV3Error> {
        let path = params
            .path
            .bytes(true)
            .map_err(UniswapV3Error::from)?;
        let result = QuoterV2Contract::new(self.address, provider)
            .quoteExactOutput(path, params.amount_out)
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(QuoteExactOutputResult {
            path: params.path,
            amount_out: params.amount_out,
            amount_in: result.amountIn,
            sqrt_price_x96_after_list: result.sqrtPriceX96AfterList,
            initialized_ticks_crossed_list: result.initializedTicksCrossedList,
            gas_estimate: result.gasEstimate,
        })
    }

    pub(crate) async fn quote_exact_output_single<P: Provider>(
        &self,
        provider: &P,
        params: QuoteExactOutputSingleParams,
    ) -> Result<QuoteExactOutputSingleResult, UniswapV3Error> {
        let (token_in, token_out, fee) = params
            .path
            .single_hop()
            .map_err(UniswapV3Error::from)?;
        let abi_params = QuoteExactOutputSingleAbiParams {
            tokenIn: token_in.address(),
            tokenOut: token_out.address(),
            amount: params.amount_out,
            fee: U24::from(fee),
            sqrtPriceLimitX96: params.sqrt_price_limit_x96,
        };
        let result = QuoterV2Contract::new(self.address, provider)
            .quoteExactOutputSingle(abi_params)
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(QuoteExactOutputSingleResult {
            path: params.path,
            amount_out: params.amount_out,
            sqrt_price_limit_x96: params.sqrt_price_limit_x96,
            amount_in: result.amountIn,
            sqrt_price_x96_after: result.sqrtPriceX96After,
            initialized_ticks_crossed: result.initializedTicksCrossed,
            gas_estimate: result.gasEstimate,
        })
    }
}
