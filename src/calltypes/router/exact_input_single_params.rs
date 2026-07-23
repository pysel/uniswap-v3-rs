use alloy::primitives::{
    Address, TxHash, U256,
    aliases::{U24, U160},
};
use uniswap_sdk_core::prelude::{BaseCurrency, Error};

use crate::{
    calltypes::{Path, TransactionFuture},
    errors::UniswapV3Error,
};

pub use crate::objects::ExactInputSingleParams;

pub struct ExactInputSingleResponse {
    pub tx_hash: TxHash,
    pub amount_out: TransactionFuture<U256>,
}

impl ExactInputSingleParams {
    #[must_use]
    pub fn builder(path: &Path) -> ExactInputSingleParamsBuilder {
        ExactInputSingleParamsBuilder {
            path: path.clone(),
            recipient: None,
            amount_in: None,
            amount_out_minimum: None,
            sqrt_price_limit_x96: None,
        }
    }

    pub fn new(
        path: &Path,
        recipient: Address,
        amount_in: U256,
        amount_out_minimum: U256,
        sqrt_price_limit_x96: U160,
    ) -> Result<Self, Error> {
        let (token_in, token_out, fee) = path.single_hop()?;

        Ok(Self {
            tokenIn: token_in.address(),
            tokenOut: token_out.address(),
            fee: U24::from(fee),
            recipient,
            amountIn: amount_in,
            amountOutMinimum: amount_out_minimum,
            sqrtPriceLimitX96: sqrt_price_limit_x96,
        })
    }
}

pub struct ExactInputSingleParamsBuilder {
    path: Path,
    recipient: Option<Address>,
    amount_in: Option<U256>,
    amount_out_minimum: Option<U256>,
    sqrt_price_limit_x96: Option<U160>,
}

impl ExactInputSingleParamsBuilder {
    #[must_use]
    pub fn recipient(mut self, recipient: Address) -> Self {
        self.recipient = Some(recipient);
        self
    }

    #[must_use]
    pub fn amount_in(mut self, amount_in: U256) -> Self {
        self.amount_in = Some(amount_in);
        self
    }

    #[must_use]
    pub fn amount_out_minimum(mut self, amount_out_minimum: U256) -> Self {
        self.amount_out_minimum = Some(amount_out_minimum);
        self
    }

    #[must_use]
    pub fn sqrt_price_limit_x96(mut self, sqrt_price_limit_x96: U160) -> Self {
        self.sqrt_price_limit_x96 = Some(sqrt_price_limit_x96);
        self
    }

    #[must_use]
    pub fn then_default(mut self) -> Self {
        if self.amount_out_minimum.is_none() {
            self.amount_out_minimum = Some(U256::ZERO);
        }
        if self.sqrt_price_limit_x96.is_none() {
            self.sqrt_price_limit_x96 = Some(U160::ZERO);
        }
        self
    }

    pub fn build(self) -> Result<ExactInputSingleParams, UniswapV3Error> {
        ExactInputSingleParams::new(
            &self.path,
            self.recipient
                .ok_or_else(|| UniswapV3Error::Invalid("RECIPIENT".to_string()))?,
            self.amount_in
                .ok_or_else(|| UniswapV3Error::Invalid("AMOUNT_IN".to_string()))?,
            self.amount_out_minimum
                .ok_or_else(|| UniswapV3Error::Invalid("AMOUNT_OUT_MINIMUM".to_string()))?,
            self.sqrt_price_limit_x96
                .ok_or_else(|| UniswapV3Error::Invalid("SQRT_PRICE_LIMIT_X96".to_string()))?,
        )
        .map_err(UniswapV3Error::from)
    }
}
