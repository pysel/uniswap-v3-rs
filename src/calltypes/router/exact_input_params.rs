use alloy::primitives::{Address, TxHash, U256};
use uniswap_sdk_core::prelude::Error;

use crate::{
    calltypes::{Path, TransactionFuture},
    errors::UniswapV3Error,
};

#[cfg(feature = "swap")]
use crate::calltypes::{BPS, QuoteExactInputResult, apply_negative_slippage};

pub use crate::objects::ExactInputParams;

pub struct ExactInputResponse {
    pub tx_hash: TxHash,
    pub amount_out: TransactionFuture<U256>,
}

impl ExactInputParams {
    #[must_use]
    pub fn builder(path: &Path) -> ExactInputParamsBuilder {
        ExactInputParamsBuilder {
            path: path.clone(),
            recipient: None,
            amount_in: None,
            amount_out_minimum: None,
        }
    }

    pub fn new(
        path: &Path,
        recipient: Address,
        amount_in: U256,
        amount_out_minimum: U256,
    ) -> Result<Self, Error> {
        Ok(Self {
            path: path.bytes(false)?,
            recipient,
            amountIn: amount_in,
            amountOutMinimum: amount_out_minimum,
        })
    }
}

pub struct ExactInputParamsBuilder {
    path: Path,
    recipient: Option<Address>,
    amount_in: Option<U256>,
    amount_out_minimum: Option<U256>,
}

#[cfg(feature = "swap")]
impl From<QuoteExactInputResult> for ExactInputParamsBuilder {
    fn from(result: QuoteExactInputResult) -> Self {
        Self {
            path: result.path,
            recipient: None,
            amount_in: Some(result.amount_in),
            amount_out_minimum: Some(result.amount_out),
        }
    }
}

impl ExactInputParamsBuilder {
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

    #[cfg(feature = "swap")]
    pub fn apply_amount_out_slippage(mut self, bps: BPS) -> Result<Self, UniswapV3Error> {
        let amount_out_minimum = self.amount_out_minimum.ok_or_else(|| {
            UniswapV3Error::RequiredFieldMissing("AMOUNT_OUT_MINIMUM".to_string())
        })?;
        self.amount_out_minimum = Some(apply_negative_slippage(amount_out_minimum, bps)?);
        Ok(self)
    }

    #[must_use]
    pub fn then_default(mut self) -> Self {
        if self.amount_out_minimum.is_none() {
            self.amount_out_minimum = Some(U256::ZERO);
        }
        self
    }

    pub fn build(self) -> Result<ExactInputParams, UniswapV3Error> {
        ExactInputParams::new(
            &self.path,
            self.recipient
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("RECIPIENT".to_string()))?,
            self.amount_in
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("AMOUNT_IN".to_string()))?,
            self.amount_out_minimum.ok_or_else(|| {
                UniswapV3Error::RequiredFieldMissing("AMOUNT_OUT_MINIMUM".to_string())
            })?,
        )
        .map_err(UniswapV3Error::from)
    }
}
