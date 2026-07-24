use alloy::primitives::{U256, aliases::U160};

use crate::{
    calltypes::Path,
    errors::UniswapV3Error,
};

#[derive(Clone, Debug)]
pub struct QuoteExactInputParams {
    pub(crate) path: Path,
    pub(crate) amount_in: U256,
}

#[derive(Clone, Debug)]
pub struct QuoteExactInputResult {
    pub path: Path,
    pub amount_in: U256,
    pub amount_out: U256,
    pub sqrt_price_x96_after_list: Vec<U160>,
    pub initialized_ticks_crossed_list: Vec<u32>,
    pub gas_estimate: U256,
}

impl QuoteExactInputParams {
    #[must_use]
    pub fn builder(path: &Path) -> QuoteExactInputParamsBuilder {
        QuoteExactInputParamsBuilder {
            path: path.clone(),
            amount_in: None,
        }
    }

    pub fn new(path: &Path, amount_in: U256) -> Result<Self, UniswapV3Error> {
        if path.is_empty() {
            return Err(UniswapV3Error::RequiredFieldMissing("PATH".to_string()));
        }

        Ok(Self {
            path: path.clone(),
            amount_in,
        })
    }
}

impl From<&QuoteExactInputParams> for QuoteExactInputParams {
    fn from(params: &QuoteExactInputParams) -> Self {
        params.clone()
    }
}

pub struct QuoteExactInputParamsBuilder {
    path: Path,
    amount_in: Option<U256>,
}

impl QuoteExactInputParamsBuilder {
    #[must_use]
    pub fn amount_in(mut self, amount_in: U256) -> Self {
        self.amount_in = Some(amount_in);
        self
    }

    pub fn build(self) -> Result<QuoteExactInputParams, UniswapV3Error> {
        QuoteExactInputParams::new(
            &self.path,
            self.amount_in
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("AMOUNT_IN".to_string()))?,
        )
    }
}
