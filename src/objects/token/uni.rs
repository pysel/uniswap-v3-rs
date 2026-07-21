use uniswap_sdk_core::{prelude::Token, token};

/// Uniswap token from Uniswap's default token list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UNI;

impl UNI {
    #[must_use]
    pub fn on_chain(chain_id: u64) -> Option<Token> {
        match chain_id {
            1 => Some(token!(
                1,
                "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984",
                18,
                "UNI",
                "Uniswap"
            )),
            137 => Some(token!(
                137,
                "0xb33EaAd8d922B1083446DC23f610c2567fB5180f",
                18,
                "UNI",
                "Uniswap"
            )),
            8453 => Some(token!(
                8453,
                "0xc3De830EA07524a0761646a6a4e4be0e114a3C83",
                18,
                "UNI",
                "Uniswap"
            )),
            _ => None,
        }
    }
}
