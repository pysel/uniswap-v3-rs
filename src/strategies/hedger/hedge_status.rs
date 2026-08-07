use alloy_primitives::U256;

use super::errors::HedgerError;

/// Direction of the hedge leg relative to the venue's asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HedgeSide {
    Long,
    Short,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hedge {
    pub venue: String,
    pub asset: String,
    pub side: HedgeSide,
    pub margin: U256,
    pub size: U256,
    pub fees_paid: U256,
}

impl Hedge {
    pub fn new(venue: String, asset: String, side: HedgeSide, margin: U256, size: U256, fees_paid: U256) -> Self {
        Self { venue, asset, side, margin, size, fees_paid }
    }
}

/// Observable status of the hedge managed by a [`super::Hedger`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HedgeStatus {
    /// No active hedge is currently required or held.
    NoHedge,
    /// An active hedge is open on the named venue.
    Hedged {
        token0_hedge: Option<Hedge>,
        token1_hedge: Option<Hedge>,
    },
    /// The hedger cannot maintain a hedge; see the wrapped error.
    Error(HedgerError),
}
