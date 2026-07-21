use uniswap_sdk_core::{prelude::Token, token};

/// Chainlink token from Uniswap's default token list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LINK;

impl LINK {
    #[must_use]
    pub fn on_chain(chain_id: u64) -> Option<Token> {
        match chain_id {
            1 => Some(token!(
                1,
                "0x514910771AF9Ca656af840dff83E8264EcF986CA",
                18,
                "LINK",
                "ChainLink Token"
            )),
            137 => Some(token!(
                137,
                "0x53E0bca35eC356BD5ddDFebbD1Fc0fD03FaBad39",
                18,
                "LINK",
                "ChainLink Token"
            )),
            _ => None,
        }
    }
}
