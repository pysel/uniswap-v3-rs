use uniswap_sdk_core::{prelude::Token, token};

/// Dai Stablecoin from Uniswap's default token list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DAI;

impl DAI {
    #[must_use]
    pub fn on_chain(chain_id: u64) -> Option<Token> {
        match chain_id {
            1 => Some(token!(
                1,
                "0x6B175474E89094C44Da98b954EedeAC495271d0F",
                18,
                "DAI",
                "Dai Stablecoin"
            )),
            137 => Some(token!(
                137,
                "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063",
                18,
                "DAI",
                "Dai Stablecoin"
            )),
            8453 => Some(token!(
                8453,
                "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb",
                18,
                "DAI",
                "Dai Stablecoin"
            )),
            _ => None,
        }
    }
}
