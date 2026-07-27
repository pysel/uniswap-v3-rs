use alloy_primitives::U256;

/// In-memory bookkeeping for the strategy-managed NPM position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Position {
    pub open_price: f64,
    pub position_id: U256,
    pub lower_tick: i32,
    pub upper_tick: i32,
}

impl Position {
    pub(super) const fn new(
        open_price: f64,
        position_id: U256,
        lower_tick: i32,
        upper_tick: i32,
    ) -> Self {
        Self {
            open_price,
            position_id,
            lower_tick,
            upper_tick,
        }
    }
}
