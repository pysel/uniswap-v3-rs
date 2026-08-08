use alloy_primitives::U256;

use super::errors::HedgerError;

/// Direction of the hedge leg relative to the venue's asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HedgeSide {
    Long,
    Short,
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
    pub fn new(
        venue: String,
        asset: String,
        side: HedgeSide,
        margin: U256,
        size: U256,
        fees_paid: U256,
    ) -> Self {
        Self {
            venue,
            asset,
            side,
            margin,
            size,
            fees_paid,
        }
    }
}

/// Latest hedge state published on the hedger status channel.
///
/// - No legs and `error: None` means idle / cleaned up.
/// - Legs with `error: None` means actively hedged.
/// - `error: Some(_)` retains the last known legs for cleanup.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HedgeStatus {
    pub token0_hedge: Option<Hedge>,
    pub token1_hedge: Option<Hedge>,
    pub error: Option<HedgerError>,
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
    pub fn with_error(
        error: HedgerError,
        token0_hedge: Option<Hedge>,
        token1_hedge: Option<Hedge>,
    ) -> Self {
        Self {
            token0_hedge,
            token1_hedge,
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
}
