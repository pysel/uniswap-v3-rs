use uniswap_sdk_core::{prelude::Token, token};

/// Wrapped BTC from Uniswap's default token list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WBTC;

impl WBTC {
    #[must_use]
    pub fn on_chain(chain_id: u64) -> Option<Token> {
        match chain_id {
            1 => Some(token!(
                1,
                "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599",
                8,
                "WBTC",
                "Wrapped BTC"
            )),
            137 => Some(token!(
                137,
                "0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6",
                8,
                "WBTC",
                "Wrapped BTC"
            )),
            _ => None,
        }
    }
}
