use uniswap_sdk_core::{prelude::Token, token};

/// Native Circle USDC from Uniswap's default token list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct USDC;

impl USDC {
    #[must_use]
    pub fn on_chain(chain_id: u64) -> Option<Token> {
        match chain_id {
            1 => Some(token!(
                1,
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
                6,
                "USDC",
                "USDCoin"
            )),
            10 => Some(token!(
                10,
                "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
                6,
                "USDC",
                "USDCoin"
            )),
            137 => Some(token!(
                137,
                "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359",
                6,
                "USDC",
                "USDCoin"
            )),
            8453 => Some(token!(
                8453,
                "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
                6,
                "USDC",
                "USD Coin"
            )),
            43114 => Some(token!(
                43114,
                "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E",
                6,
                "USDC",
                "USDC Token"
            )),
            _ => None,
        }
    }
}
