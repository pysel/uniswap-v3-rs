use std::fmt::{Display, Formatter};

use alloy_primitives::{Address, U256};

use crate::objects::{Pool, Position as NpmPosition};

/// In-memory bookkeeping for the strategy-managed NPM position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub open_price: f64,
    pub position_id: U256,
    pub pool: Address,
    pub chain_id: u64,
    pub lower_tick: i32,
    pub upper_tick: i32,
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let pool = format!("{}", self.pool);
        let pool = if pool.len() >= 10 {
            format!("{}…{}", &pool[..6], &pool[pool.len() - 4..])
        } else {
            pool
        };
        write!(
            f,
            "Position(open_price={}, pool={}, [{}, {}])",
            self.open_price, pool, self.lower_tick, self.upper_tick
        )
    }
}

impl Position {
    pub(crate) const fn new(
        open_price: f64,
        position_id: U256,
        pool: Address,
        chain_id: u64,
        lower_tick: i32,
        upper_tick: i32,
    ) -> Self {
        Self {
            open_price,
            position_id,
            pool,
            chain_id,
            lower_tick,
            upper_tick,
        }
    }

    /// Builds strategy bookkeeping for an on-chain NPM position in `pool`.
    ///
    /// The NPM position carries neither the open price nor the pool address / chain, so the
    /// caller supplies `open_price` and the `pool` the position belongs to.
    pub(crate) fn from_sdk(sdk_position: &NpmPosition, pool: &Pool, open_price: f64) -> Self {
        Self::new(
            open_price,
            sdk_position.token_id(),
            pool.address(),
            pool.chain_id(),
            sdk_position.tick_lower(),
            sdk_position.tick_upper(),
        )
    }
}
