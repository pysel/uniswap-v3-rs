use std::fmt::{Display, Formatter};

use alloy_primitives::U256;
use uniswap_sdk_core::entities::Token;

use super::errors::HedgerError;

/// Direction of the hedge leg relative to the venue's asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HedgeSide {
    Long,
    Short,
}

impl HedgeSide {
    pub fn from_size(size: U256) -> Self {
        if size.gt(&U256::ZERO) {
            HedgeSide::Long
        } else {
            HedgeSide::Short
        }
    }
}

/// One venue hedge leg managed for a Uniswap pool token.
///
/// Units:
/// - [`Self::size`]: absolute Hyperliquid base size expressed in the source ERC-20's
///   raw decimal units (`amount * 10^decimals`).
/// - [`Self::margin`]: USD/USDC margin allocated to the leg at 6-decimal atomic
///   precision (`1 USDC = 1_000_000`).
/// - [`Self::fees_paid`]: cumulative estimated taker fees for the current managed
///   session, also in 6-decimal USD/USDC units.
#[derive(Clone, Debug, PartialEq)]
pub struct Hedge {
    pub venue: String,
    pub asset: Token,
    pub side: HedgeSide,
    pub size: U256,
    pub fees_paid: U256,
}

impl Display for Hedge {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Hedge(venue={}, asset={}, size={}, fees_paid={})",
            self.venue, self.asset.address, self.size, self.fees_paid
        )
    }
}

impl Hedge {
    pub fn new(venue: String, asset: Token, side: HedgeSide, size: U256, fees_paid: U256) -> Self {
        Self {
            venue,
            asset,
            side,
            size,
            fees_paid,
        }
    }

    pub fn update_size(mut self, new_size: U256) -> Self {
        self.size = new_size;
        self
    }

    pub fn increase_fees(mut self, fees: U256) -> Self {
        self.fees_paid += fees;
        self
    }
}

/// Latest hedge state published on the hedger status channel.
///
/// - No legs and `error: None` means idle / cleaned up.
/// - Legs with `error: None` means actively hedged.
/// - `error: Some(_)` retains the last known legs for cleanup.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HedgeStatus {
    pub token0_hedge: Option<Hedge>,
    pub token1_hedge: Option<Hedge>,
    pub error: Option<HedgerError>,
}

impl Display for HedgeStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_idle() {
            write!(f, "Idle")
        } else if self.has_error() {
            write!(f, "Error: {:?}", self.error)
        } else {
            if let Some(token0_hedge) = &self.token0_hedge {
                write!(f, "{}", token0_hedge)?;
            }
            if let Some(token1_hedge) = &self.token1_hedge {
                write!(f, "{}", token1_hedge)?;
            }
            Ok(())
        }
    }
}

impl HedgeStatus {
    #[must_use]
    pub fn idle() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn hedged(token0_hedge: Option<Hedge>, token1_hedge: Option<Hedge>) -> Self {
        Self {
            token0_hedge,
            token1_hedge,
            error: None,
        }
    }

    #[must_use]
    pub fn with_error(self, error: HedgerError) -> Self {
        Self {
            token0_hedge: self.token0_hedge,
            token1_hedge: self.token1_hedge,
            error: Some(error),
        }
    }

    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.token0_hedge.is_none() && self.token1_hedge.is_none() && self.error.is_none()
    }

    #[must_use]
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn get_hedge(&self, token: &Token) -> Option<&Hedge> {
        [&self.token0_hedge, &self.token1_hedge]
            .into_iter()
            .flatten()
            .find(|hedge| hedge.asset.address == token.address)
    }
}
