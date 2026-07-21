use uniswap_sdk_core::{prelude::Token, token};

/// Tether USD from Uniswap's default token list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct USDT;

impl USDT {
    #[must_use]
    pub fn on_chain(chain_id: u64) -> Option<Token> {
        match chain_id {
            1 => Some(token!(
                1,
                "0xdAC17F958D2ee523a2206206994597C13D831ec7",
                6,
                "USDT",
                "Tether USD"
            )),
            137 => Some(token!(
                137,
                "0xc2132D05D31c914a87C6611C10748AEb04B58e8F",
                6,
                "USDT",
                "Tether USD"
            )),
            _ => None,
        }
    }
}
