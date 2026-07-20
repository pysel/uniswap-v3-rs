use alloy::{
    primitives::{Address, TxHash, U256},
    providers::Provider,
};
use uniswap_sdk_core::prelude::{BaseCurrency, Token};

use crate::errors::UniswapV3Error;

use super::abi_definitions::Erc20;

pub trait TokenExt {
    fn from_address<P: Provider>(
        address: Address,
        chain_id: u64,
        provider: &P,
    ) -> impl Future<Output = Result<Token, UniswapV3Error>>;

    fn approve<P: Provider>(
        &self,
        provider: &P,
        spender: Address,
        amount: U256,
    ) -> impl Future<Output = Result<TxHash, UniswapV3Error>>;

    fn approve_unlimited<P: Provider>(
        &self,
        provider: &P,
        spender: Address,
    ) -> impl Future<Output = Result<TxHash, UniswapV3Error>>;
}

impl TokenExt for Token {
    async fn from_address<P: Provider>(
        address: Address,
        chain_id: u64,
        provider: &P,
    ) -> Result<Token, UniswapV3Error> {
        let contract = Erc20::new(address, provider);
        let decimals = contract
            .decimals()
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;
        let symbol = contract.symbol().call().await.ok();
        let name = contract.name().call().await.ok();

        Ok(Token::new(chain_id, address, decimals, symbol, name, 0, 0))
    }

    async fn approve<P: Provider>(
        &self,
        provider: &P,
        spender: Address,
        amount: U256,
    ) -> Result<TxHash, UniswapV3Error> {
        let pending = Erc20::new(self.address(), provider)
            .approve(spender, amount)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(*pending.tx_hash())
    }

    async fn approve_unlimited<P: Provider>(
        &self,
        provider: &P,
        spender: Address,
    ) -> Result<TxHash, UniswapV3Error> {
        self.approve(provider, spender, U256::MAX).await
    }
}
