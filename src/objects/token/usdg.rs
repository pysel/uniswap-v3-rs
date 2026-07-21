use uniswap_sdk_core::{prelude::Token, token};

/// Global Dollar (USDG) from Uniswap's default token list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct USDG;

impl USDG {
    #[must_use]
    pub fn on_chain(chain_id: u64) -> Option<Token> {
        match chain_id {
            1 => Some(token!(
                1,
                "0xe343167631d89B6Ffc58B88d6b7fB0228795491D",
                6,
                "USDG",
                "Global Dollar"
            )),
            _ => None,
        }
    }
}
