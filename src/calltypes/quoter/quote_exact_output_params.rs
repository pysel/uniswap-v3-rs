use alloy::primitives::{U256, aliases::U160};

use crate::{
    calltypes::Path,
    errors::UniswapV3Error,
};

#[derive(Clone, Debug)]
pub struct QuoteExactOutputParams {
    pub(crate) path: Path,
    pub(crate) amount_out: U256,
}

#[derive(Clone, Debug)]
pub struct QuoteExactOutputResult {
    pub path: Path,
    pub amount_out: U256,
    pub amount_in: U256,
    pub sqrt_price_x96_after_list: Vec<U160>,
    pub initialized_ticks_crossed_list: Vec<u32>,
    pub gas_estimate: U256,
}

impl QuoteExactOutputParams {
    #[must_use]
    pub fn builder(path: &Path) -> QuoteExactOutputParamsBuilder {
        QuoteExactOutputParamsBuilder {
            path: path.clone(),
            amount_out: None,
        }
    }

    pub fn new(path: &Path, amount_out: U256) -> Result<Self, UniswapV3Error> {
        if path.is_empty() {
            return Err(UniswapV3Error::RequiredFieldMissing("PATH".to_string()));
        }

        Ok(Self {
            path: path.clone(),
            amount_out,
        })
    }
}


pub struct QuoteExactOutputParamsBuilder {
    path: Path,
    amount_out: Option<U256>,
}

impl QuoteExactOutputParamsBuilder {
    #[must_use]
    pub fn amount_out(mut self, amount_out: U256) -> Self {
        self.amount_out = Some(amount_out);
        self
    }

    pub fn build(self) -> Result<QuoteExactOutputParams, UniswapV3Error> {
        QuoteExactOutputParams::new(
            &self.path,
            self.amount_out
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("AMOUNT_OUT".to_string()))?,
        )
    }
}
