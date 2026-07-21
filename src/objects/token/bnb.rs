use uniswap_sdk_core::{prelude::Token, token};

/// BNB / wrapped BNB representations used on EVM chains.
///
/// Sources: Binance ERC-20 on Ethereum, Polygon PoS bridge, PancakeSwap/Uniswap
/// WBNB on Arbitrum, and Wrapped BNB on BNB Chain. Verified on-chain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BNB;

impl BNB {
    #[must_use]
    pub fn on_chain(chain_id: u64) -> Option<Token> {
        match chain_id {
            1 => Some(token!(
                1,
                "0xB8c77482e45F1F44dE1745F52C74426C631bDD52",
                18,
                "BNB",
                "BNB"
            )),
            // BNB Chain — tradeable form is WBNB.
            56 => Some(token!(
                56,
                "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c",
                18,
                "WBNB",
                "Wrapped BNB"
            )),
            137 => Some(token!(
                137,
                "0x3BA4c387f786bFEE076A58914F5Bd38d668B42c3",
                18,
                "BNB",
                "BNB (PoS)"
            )),
            43114 => Some(token!(
                43114,
                "0x264c1383ea520f73dd837f915ef3a732e204a493",
                18,
                "BNB",
                "Binance"
            )),
            42161 => Some(token!(
                42161,
                "0xa9004A5421372E1D83fB1f85b0fc986c912f91f3",
                18,
                "WBNB",
                "Wrapped BNB"
            )),
            _ => None,
        }
    }
}
