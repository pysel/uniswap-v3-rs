use alloy::{primitives::Address, providers::Provider};
use uniswap_sdk_core::prelude::Token;

use crate::errors::UniswapV3Error;

use super::abi_definitions::Erc20Metadata;

pub trait TokenExt {
    fn from_address<P: Provider>(
        address: Address,
        chain_id: u64,
        provider: &P,
    ) -> impl Future<Output = Result<Token, UniswapV3Error>>;
}

impl TokenExt for Token {
    async fn from_address<P: Provider>(
        address: Address,
        chain_id: u64,
        provider: &P,
    ) -> Result<Token, UniswapV3Error> {
        let contract = Erc20Metadata::new(address, provider);
        let decimals = contract
            .decimals()
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;
        let symbol = contract.symbol().call().await.ok();
        let name = contract.name().call().await.ok();

        Ok(Token::new(chain_id, address, decimals, symbol, name, 0, 0))
    }
}
