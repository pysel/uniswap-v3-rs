use alloy::primitives::Address;
use uniswap_sdk_core::prelude::{BaseCurrency, Error, Token};

use super::UniswapV3Factory;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UniswapV3Pool {
    factory: UniswapV3Factory,
    token0: Token,
    token1: Token,
    fee: u32,
}

impl UniswapV3Pool {
    pub fn new(
        factory: UniswapV3Factory,
        token_a: Token,
        token_b: Token,
        fee: u32,
    ) -> Result<Self, Error> {
        let token_a_sorts_before = factory.validate_pool_key(&token_a, &token_b, fee)?;
        let (token0, token1) = if token_a_sorts_before {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };

        Ok(Self {
            factory,
            token0,
            token1,
            fee,
        })
    }

    #[must_use]
    pub const fn factory(&self) -> &UniswapV3Factory {
        &self.factory
    }

    #[must_use]
    pub const fn token0(&self) -> &Token {
        &self.token0
    }

    #[must_use]
    pub const fn token1(&self) -> &Token {
        &self.token1
    }

    #[must_use]
    pub const fn fee(&self) -> u32 {
        self.fee
    }

    #[must_use]
    pub const fn chain_id(&self) -> u64 {
        self.factory.chain_id()
    }

    #[must_use]
    pub fn address(&self) -> Address {
        self.factory
            .derive_pool_address(self.token0.address(), self.token1.address(), self.fee)
    }

    #[must_use]
    pub fn involves_token(&self, token: &Token) -> bool {
        self.token0.equals(token) || self.token1.equals(token)
    }
}
