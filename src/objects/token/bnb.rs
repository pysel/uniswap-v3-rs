use uniswap_sdk_core::prelude::Token;

/// BNB from Uniswap's default token list.
///
/// Not present on the currently supported chain lists
/// (`mainnet`, `arbitrum`, `base`, `avalanche`, `optimism`, `polygon`, `tempo`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BNB;

impl BNB {
    #[must_use]
    pub fn on_chain(_chain_id: u64) -> Option<Token> {
        None
    }
}
