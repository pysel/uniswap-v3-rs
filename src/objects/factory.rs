use alloy::{
    primitives::{Address, B256, aliases::U24, b256, keccak256},
    providers::Provider,
    sol_types::SolValue,
};
use uniswap_sdk_core::prelude::{
    BaseCurrency, ChainId, Error, Token, V3_CORE_FACTORY_ADDRESSES, compute_zksync_create2_address,
};

use crate::{errors::UniswapV3Error, objects::Pool};

const POOL_INIT_CODE_HASH: B256 =
    b256!("e34f199b19b2b4f47f68442619d555527d244f78a3297ea89325f843f87b8b54");

const ZKSYNC_POOL_BYTECODE_HASH: B256 =
    b256!("010013f177ea1fcbc4520f9a3ca7cd2d1d77959e05aa66484027cb38e712aeed");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Factory {
    chain_id: u64,
    address: Address,
}

impl Factory {
    pub fn new(chain_id: u64, address: Address) -> Result<Self, Error> {
        if chain_id == 0 {
            return Err(Error::Invalid("CHAIN_ID"));
        }

        Ok(Self { chain_id, address })
    }

    pub fn from_chain(chain_id: u64) -> Option<Self> {
        V3_CORE_FACTORY_ADDRESSES
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

    pub fn pool_address(
        &self,
        token_a: &Token,
        token_b: &Token,
        fee: u32,
    ) -> Result<Address, Error> {
        let token_a_sorts_before = self.validate_pool_key(token_a, token_b, fee)?;
        let (token0, token1) = if token_a_sorts_before {
            (token_a.address(), token_b.address())
        } else {
            (token_b.address(), token_a.address())
        };

        Ok(self.derive_pool_address(token0, token1, fee))
    }

    pub async fn pool<P: Provider>(
        &self,
        token0: Token,
        token1: Token,
        fee: u32,
        provider: &P,
    ) -> Result<Pool, UniswapV3Error> {
        let address = self.pool_address(&token0, &token1, fee)?;
        Pool::from_address(address, provider).await
    }

    pub(crate) fn validate_pool_key(
        &self,
        token_a: &Token,
        token_b: &Token,
        fee: u32,
    ) -> Result<bool, Error> {
        let token_a_sorts_before = token_a.sorts_before(token_b)?;
        if token_a.chain_id != self.chain_id {
            return Err(Error::ChainIdMismatch(token_a.chain_id, self.chain_id));
        }
        if fee >= 1_000_000 {
            return Err(Error::Invalid("FEE"));
        }

        Ok(token_a_sorts_before)
    }

    pub(crate) fn derive_pool_address(
        &self,
        token0: Address,
        token1: Address,
        fee: u32,
    ) -> Address {
        let salt = keccak256((token0, token1, U24::from(fee)).abi_encode());

        if self.chain_id == ChainId::ZKSYNC as u64 {
            compute_zksync_create2_address(self.address, ZKSYNC_POOL_BYTECODE_HASH, salt, None)
        } else {
            self.address.create2(salt, POOL_INIT_CODE_HASH)
        }
    }
}
