use alloy::primitives::{U256, aliases::U160};

use crate::{calltypes::Path, errors::UniswapV3Error};

#[derive(Clone, Debug)]
pub struct QuoteExactInputSingleParams {
    pub(crate) path: Path,
    pub(crate) amount_in: U256,
    pub(crate) sqrt_price_limit_x96: U160,
}

#[derive(Clone, Debug)]
pub struct QuoteExactInputSingleResult {
    pub path: Path,
    pub amount_in: U256,
    pub sqrt_price_limit_x96: U160,
    pub amount_out: U256,
    pub sqrt_price_x96_after: U160,
    pub initialized_ticks_crossed: u32,
    pub gas_estimate: U256,
}

impl QuoteExactInputSingleParams {
    #[must_use]
    pub fn builder(path: &Path) -> QuoteExactInputSingleParamsBuilder {
        QuoteExactInputSingleParamsBuilder {
            path: path.clone(),
            amount_in: None,
            sqrt_price_limit_x96: None,
        }
    }

    pub fn new(
        path: &Path,
        amount_in: U256,
        sqrt_price_limit_x96: U160,
    ) -> Result<Self, UniswapV3Error> {
        path.single_hop().map_err(UniswapV3Error::from)?;

        Ok(Self {
            path: path.clone(),
            amount_in,
            sqrt_price_limit_x96,
        })
    }
}

pub struct QuoteExactInputSingleParamsBuilder {
    path: Path,
    amount_in: Option<U256>,
    sqrt_price_limit_x96: Option<U160>,
}

impl QuoteExactInputSingleParamsBuilder {
    #[must_use]
    pub fn amount_in(mut self, amount_in: U256) -> Self {
        self.amount_in = Some(amount_in);
        self
    }

    #[must_use]
    pub fn sqrt_price_limit_x96(mut self, sqrt_price_limit_x96: U160) -> Self {
        self.sqrt_price_limit_x96 = Some(sqrt_price_limit_x96);
        self
    }

    #[must_use]
    pub fn then_default(mut self) -> Self {
        if self.sqrt_price_limit_x96.is_none() {
            self.sqrt_price_limit_x96 = Some(U160::ZERO);
        }
        self
    }

    pub fn build(self) -> Result<QuoteExactInputSingleParams, UniswapV3Error> {
        QuoteExactInputSingleParams::new(
            &self.path,
            self.amount_in
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("AMOUNT_IN".to_string()))?,
            self.sqrt_price_limit_x96.ok_or_else(|| {
                UniswapV3Error::RequiredFieldMissing("SQRT_PRICE_LIMIT_X96".to_string())
            })?,
        )
    }
}
