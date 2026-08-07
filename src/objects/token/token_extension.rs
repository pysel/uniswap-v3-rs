use alloy::primitives::{Address, TxHash, U256, utils::parse_units};
use uniswap_sdk_core::{
    entities::BaseCurrencyCore,
    prelude::{BaseCurrency, Token},
};

use crate::{client::UniswapV3Client, errors::UniswapV3Error};

use crate::objects::Erc20Contract;

pub trait TokenExt {
    fn from_address(
        address: Address,
        chain_id: u64,
        client: &UniswapV3Client,
    ) -> impl Future<Output = Result<Token, UniswapV3Error>>;

    fn balance_of(
        &self,
        client: &UniswapV3Client,
        owner: Address,
    ) -> impl Future<Output = Result<U256, UniswapV3Error>>;

    fn allowance(
        &self,
        client: &UniswapV3Client,
        owner: Address,
        spender: Address,
    ) -> impl Future<Output = Result<U256, UniswapV3Error>>;

    fn approve(
        &self,
        client: &UniswapV3Client,
        spender: Address,
        amount: U256,
    ) -> impl Future<Output = Result<TxHash, UniswapV3Error>>;

    fn approve_unlimited(
        &self,
        client: &UniswapV3Client,
        spender: Address,
    ) -> impl Future<Output = Result<TxHash, UniswapV3Error>>;

    /// Converts a human-token `amount` into raw units: `amount * 10^decimals`.
    ///
    /// Fractional amounts are supported (e.g. `0.01` WETH). Panics if `amount` is
    /// negative, non-finite, or overflows the token's decimal scale.
    #[allow(clippy::wrong_self_convention)] // Conversion requires token decimals
    fn from_amount(&self, amount: f64) -> U256;

    /// Returns whether this token's symbol is a known USD stablecoin supported by
    /// this crate (`USDT`, `USDC`, `DAI`, `USDE`, `USDG`, `USDT0`).
    ///
    /// Tokens without a symbol are treated as non-stablecoins.
    fn is_stablecoin(&self) -> bool;
}

impl TokenExt for Token {
    async fn from_address(
        address: Address,
        chain_id: u64,
        client: &UniswapV3Client,
    ) -> Result<Token, UniswapV3Error> {
        let contract = Erc20Contract::new(address, client.provider());
        let decimals = contract
            .decimals()
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;
        let symbol = contract.symbol().call().await.ok();
        let name = contract.name().call().await.ok();

        Ok(Token::new(chain_id, address, decimals, symbol, name, 0, 0))
    }

    async fn balance_of(
        &self,
        client: &UniswapV3Client,
        owner: Address,
    ) -> Result<U256, UniswapV3Error> {
        Erc20Contract::new(self.address(), client.provider())
            .balanceOf(owner)
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
    }

    async fn allowance(
        &self,
        client: &UniswapV3Client,
        owner: Address,
        spender: Address,
    ) -> Result<U256, UniswapV3Error> {
        Erc20Contract::new(self.address(), client.provider())
            .allowance(owner, spender)
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
    }

    async fn approve(
        &self,
        client: &UniswapV3Client,
        spender: Address,
        amount: U256,
    ) -> Result<TxHash, UniswapV3Error> {
        let pending = Erc20Contract::new(self.address(), client.provider())
            .approve(spender, amount)
            .send()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        Ok(*pending.tx_hash())
    }

    async fn approve_unlimited(
        &self,
        client: &UniswapV3Client,
        spender: Address,
    ) -> Result<TxHash, UniswapV3Error> {
        let owner = client.signer_address().ok_or_else(|| {
            UniswapV3Error::BuildError("signer is required for approve".to_string())
        })?;
        // Treat "still huge" as unlimited so tokens that decrement allowance after
        // transferFrom (e.g. USDC) do not re-approve on every run.
        let unlimited_threshold = U256::MAX / U256::from(10u8) * U256::from(7u8);
        if self.allowance(client, owner, spender).await? >= unlimited_threshold {
            return Ok(TxHash::default());
        }
        self.approve(client, spender, U256::MAX).await
    }

    fn from_amount(&self, amount: f64) -> U256 {
        assert!(
            amount.is_finite() && amount >= 0.0,
            "amount must be finite and non-negative"
        );
        let decimals = self.decimals();
        let formatted = format!("{amount:.prec$}", prec = usize::from(decimals));
        parse_units(&formatted, decimals)
            .expect("token amount overflowed decimal scale")
            .into()
    }

    fn is_stablecoin(&self) -> bool {
        self.symbol()
            .map(|symbol| {
                matches!(
                    symbol.to_uppercase().as_str(),
                    "USDT" | "USDC" | "DAI" | "USDE" | "USDG" | "USDT0"
                )
            })
            .unwrap_or(false)
    }
}
