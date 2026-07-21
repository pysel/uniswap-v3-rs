use uniswap_sdk_core::{prelude::Token, token};

/// Ethena USDe.
///
/// Addresses from [Ethena key addresses](https://docs.ethena.fi/solution-design/key-addresses)
/// and verified on-chain (symbol `USDe`, 18 decimals). Not deployed on Polygon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct USDe;

impl USDe {
    #[must_use]
    pub fn on_chain(chain_id: u64) -> Option<Token> {
        match chain_id {
            1 => Some(token!(
                1,
                "0x4c9edd5852cd905f086c759e8383e09bff1e68b3",
                18,
                "USDe",
                "USDe"
            )),
            10 => Some(token!(
                10,
                "0x5d3a1Ff2b6BAb83b63cd9AD0787074081a52ef34",
                18,
                "USDe",
                "USDe"
            )),
            8453 => Some(token!(
                8453,
                "0x5d3a1Ff2b6BAb83b63cd9AD0787074081a52ef34",
                18,
                "USDe",
                "USDe"
            )),
            42161 => Some(token!(
                42161,
                "0x5d3a1Ff2b6BAb83b63cd9AD0787074081a52ef34",
                18,
                "USDe",
                "USDe"
            )),
            43114 => Some(token!(
                43114,
                "0x5d3a1Ff2b6BAb83b63cd9AD0787074081a52ef34",
                18,
                "USDe",
                "USDe"
            )),
            4217 => Some(token!(
                4217,
                "0x20c0000000000000000000002F52D5CC21A3207B",
                18,
                "USDe",
                "USDe"
            )),
            _ => None,
        }
    }
}
