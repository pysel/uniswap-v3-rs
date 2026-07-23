use alloy::primitives::{
    Address, TxHash, U256,
    aliases::{U24, U160},
};
use uniswap_sdk_core::prelude::{BaseCurrency, Error};

use crate::{
    calltypes::{Path, TransactionFuture},
    errors::UniswapV3Error,
};

pub use crate::objects::ExactOutputSingleParams;

pub struct ExactOutputSingleResponse {
    pub tx_hash: TxHash,
    pub amount_in: TransactionFuture<U256>,
}

impl ExactOutputSingleParams {
    #[must_use]
    pub fn builder(path: &Path) -> ExactOutputSingleParamsBuilder {
        ExactOutputSingleParamsBuilder {
            path: path.clone(),
            recipient: None,
            amount_out: None,
            amount_in_maximum: None,
            sqrt_price_limit_x96: None,
        }
    }

    pub fn new(
        path: &Path,
        recipient: Address,
        amount_out: U256,
        amount_in_maximum: U256,
        sqrt_price_limit_x96: U160,
    ) -> Result<Self, Error> {
        let (token_in, token_out, fee) = path.single_hop()?;

        Ok(Self {
            tokenIn: token_in.address(),
            tokenOut: token_out.address(),
            fee: U24::from(fee),
            recipient,
            amountOut: amount_out,
            amountInMaximum: amount_in_maximum,
            sqrtPriceLimitX96: sqrt_price_limit_x96,
        })
    }
}

pub struct ExactOutputSingleParamsBuilder {
    path: Path,
    recipient: Option<Address>,
    amount_out: Option<U256>,
    amount_in_maximum: Option<U256>,
    sqrt_price_limit_x96: Option<U160>,
}

impl ExactOutputSingleParamsBuilder {
    #[must_use]
    pub fn recipient(mut self, recipient: Address) -> Self {
        self.recipient = Some(recipient);
        self
    }

    #[must_use]
    pub fn amount_out(mut self, amount_out: U256) -> Self {
        self.amount_out = Some(amount_out);
        self
    }

    #[must_use]
    pub fn amount_in_maximum(mut self, amount_in_maximum: U256) -> Self {
        self.amount_in_maximum = Some(amount_in_maximum);
        self
    }

    #[must_use]
    pub fn sqrt_price_limit_x96(mut self, sqrt_price_limit_x96: U160) -> Self {
        self.sqrt_price_limit_x96 = Some(sqrt_price_limit_x96);
        self
    }

    #[must_use]
    pub fn then_default(mut self) -> Self {
        if self.amount_in_maximum.is_none() {
            self.amount_in_maximum = Some(U256::MAX);
        }
        if self.sqrt_price_limit_x96.is_none() {
            self.sqrt_price_limit_x96 = Some(U160::ZERO);
        }
        self
    }

    pub fn build(self) -> Result<ExactOutputSingleParams, UniswapV3Error> {
        ExactOutputSingleParams::new(
            &self.path,
            self.recipient
                .ok_or_else(|| UniswapV3Error::Invalid("RECIPIENT".to_string()))?,
            self.amount_out
                .ok_or_else(|| UniswapV3Error::Invalid("AMOUNT_OUT".to_string()))?,
            self.amount_in_maximum
                .ok_or_else(|| UniswapV3Error::Invalid("AMOUNT_IN_MAXIMUM".to_string()))?,
            self.sqrt_price_limit_x96
                .ok_or_else(|| UniswapV3Error::Invalid("SQRT_PRICE_LIMIT_X96".to_string()))?,
        )
        .map_err(UniswapV3Error::from)
    }
}
