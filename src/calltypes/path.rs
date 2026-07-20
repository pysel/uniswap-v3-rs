use alloy::primitives::Bytes;
use uniswap_sdk_core::prelude::{BaseCurrency, Error, Token};

const MAX_FEE: u32 = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Hop {
    token: Token,
    fee: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    input_token: Token,
    hops: Vec<Hop>,
}

impl Path {
    #[must_use]
    pub const fn new(input_token: Token) -> Self {
        Self {
            input_token,
            hops: Vec::new(),
        }
    }

    pub fn add_hop(&mut self, token: Token, fee: u32) -> Result<&mut Self, Error> {
        if fee >= MAX_FEE {
            return Err(Error::Invalid("FEE"));
        }

        let previous_token = self
            .hops
            .last()
            .map_or(&self.input_token, |hop| &hop.token);
        if previous_token.chain_id != token.chain_id {
            return Err(Error::ChainIdMismatch(
                previous_token.chain_id,
                token.chain_id,
            ));
        }
        if previous_token.equals(&token) {
            return Err(Error::EqualAddresses);
        }

        self.hops.push(Hop { token, fee });
        Ok(self)
    }

    #[must_use]
    pub const fn input_token(&self) -> &Token {
        &self.input_token
    }

    #[must_use]
    pub fn output_token(&self) -> &Token {
        self.hops
            .last()
            .map_or(&self.input_token, |hop| &hop.token)
    }

    #[must_use]
    pub const fn num_hops(&self) -> usize {
        self.hops.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.hops.is_empty()
    }

    pub fn single_hop(&self) -> Result<(&Token, &Token, u32), Error> {
        let [hop] = self.hops.as_slice() else {
            return Err(Error::Invalid("SINGLE_HOP_PATH"));
        };

        Ok((&self.input_token, &hop.token, hop.fee))
    }

    pub fn bytes(&self, exact_output: bool) -> Result<Bytes, Error> {
        if self.hops.is_empty() {
            return Err(Error::Invalid("PATH"));
        }

        let mut encoded = Vec::with_capacity(20 + 23 * self.hops.len());
        if exact_output {
            encoded.extend_from_slice(self.output_token().address().as_slice());
            for (index, hop) in self.hops.iter().enumerate().rev() {
                encoded.extend_from_slice(&hop.fee.to_be_bytes()[1..]);
                let previous_token = if index == 0 {
                    &self.input_token
                } else {
                    &self.hops[index - 1].token
                };
                encoded.extend_from_slice(previous_token.address().as_slice());
            }
        } else {
            encoded.extend_from_slice(self.input_token.address().as_slice());
            for hop in &self.hops {
                encoded.extend_from_slice(&hop.fee.to_be_bytes()[1..]);
                encoded.extend_from_slice(hop.token.address().as_slice());
            }
        }

        Ok(encoded.into())
    }
}
