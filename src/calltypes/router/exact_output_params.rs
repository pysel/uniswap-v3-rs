use alloy::primitives::{Address, TxHash, U256};
use uniswap_sdk_core::prelude::Error;

use crate::{
    calltypes::{Path, TransactionFuture},
    errors::UniswapV3Error,
};

#[cfg(feature = "swap")]
use crate::calltypes::{BPS, QuoteExactOutputResult, apply_positive_slippage};

pub use crate::objects::ExactOutputParams;

pub struct ExactOutputResponse {
    pub tx_hash: TxHash,
    pub amount_in: TransactionFuture<U256>,
}

impl ExactOutputParams {
    #[must_use]
    pub fn builder(path: &Path) -> ExactOutputParamsBuilder {
        ExactOutputParamsBuilder {
            path: path.clone(),
            recipient: None,
            amount_out: None,
            amount_in_maximum: None,
        }
    }

    pub fn new(
        path: &Path,
        recipient: Address,
        amount_out: U256,
        amount_in_maximum: U256,
    ) -> Result<Self, Error> {
        Ok(Self {
            path: path.bytes(true)?,
            recipient,
            amountOut: amount_out,
            amountInMaximum: amount_in_maximum,
        })
    }
}

pub struct ExactOutputParamsBuilder {
    path: Path,
    recipient: Option<Address>,
    amount_out: Option<U256>,
    amount_in_maximum: Option<U256>,
}

#[cfg(feature = "swap")]
impl From<QuoteExactOutputResult> for ExactOutputParamsBuilder {
    fn from(result: QuoteExactOutputResult) -> Self {
        Self {
            path: result.path,
            recipient: None,
            amount_out: Some(result.amount_out),
            amount_in_maximum: Some(result.amount_in),
        }
    }
}

impl ExactOutputParamsBuilder {
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

    #[cfg(feature = "swap")]
    pub fn apply_amount_in_slippage(mut self, bps: BPS) -> Result<Self, UniswapV3Error> {
        let amount_in_maximum = self
            .amount_in_maximum
            .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("AMOUNT_IN_MAXIMUM".to_string()))?;
        self.amount_in_maximum = Some(apply_positive_slippage(amount_in_maximum, bps)?);
        Ok(self)
    }

    #[must_use]
    pub fn then_default(mut self) -> Self {
        if self.amount_in_maximum.is_none() {
            self.amount_in_maximum = Some(U256::MAX);
        }
        self
    }

    pub fn build(self) -> Result<ExactOutputParams, UniswapV3Error> {
        ExactOutputParams::new(
            &self.path,
            self.recipient
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("RECIPIENT".to_string()))?,
            self.amount_out
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("AMOUNT_OUT".to_string()))?,
            self.amount_in_maximum.ok_or_else(|| {
                UniswapV3Error::RequiredFieldMissing("AMOUNT_IN_MAXIMUM".to_string())
            })?,
        )
        .map_err(UniswapV3Error::from)
    }
}
