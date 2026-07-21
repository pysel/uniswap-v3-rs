use uniswap_sdk_core::{prelude::Token, token};

/// Coinbase Wrapped BTC from Uniswap's default token list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CbBTC;

impl CbBTC {
    #[must_use]
    pub fn on_chain(chain_id: u64) -> Option<Token> {
        match chain_id {
            1 => Some(token!(
                1,
                "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf",
                8,
                "cbBTC",
                "Coinbase Wrapped BTC"
            )),
            4217 => Some(token!(
                4217,
                "0x20C000000000000000000000c412Ec89D0c08be5",
                6,
                "cbBTC",
                "Coinbase Wrapped BTC"
            )),
            8453 => Some(token!(
                8453,
                "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf",
                8,
                "cbBTC",
                "Coinbase Wrapped BTC"
            )),
            42161 => Some(token!(
                42161,
                "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf",
                8,
                "cbBTC",
                "Coinbase Wrapped BTC"
            )),
            _ => None,
        }
    }
}
