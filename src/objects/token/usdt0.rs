use uniswap_sdk_core::{prelude::Token, token};

/// USDT0 from Uniswap's default token list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct USDT0;

impl USDT0 {
    #[must_use]
    pub fn on_chain(chain_id: u64) -> Option<Token> {
        match chain_id {
            10 => Some(token!(
                10,
                "0x01bFF41798a0BcF287b996046Ca68b395DbC1071",
                6,
                "USDT0",
                "USDT0"
            )),
            4217 => Some(token!(
                4217,
                "0x20C00000000000000000000014f22CA97301EB73",
                6,
                "USDT0",
                "USDT0"
            )),
            42161 => Some(token!(
                42161,
                "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9",
                6,
                "USDT0",
                "USDT0"
            )),
            _ => None,
        }
    }
}
