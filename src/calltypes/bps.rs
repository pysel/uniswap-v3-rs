use crate::errors::UniswapV3Error;

const BPS_DENOMINATOR: u16 = 10_000;

/// Basis points (`1 BPS = 0.01%`, `10_000 BPS = 100%`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BPS(u16);

impl BPS {
    /// Constructs from a raw basis-point value.
    #[must_use]
    pub const fn new(bps: u16) -> Self {
        Self(bps)
    }

    /// Constructs from a whole-number percent in `0..=100` (`1%` → `100` BPS).
    pub fn from_percent(percent: u16) -> Result<Self, UniswapV3Error> {
        if percent > 100 {
            return Err(UniswapV3Error::Math(format!(
                "percent {percent} exceeds 100"
            )));
        }
        Ok(Self(percent * 100))
    }

    pub const fn denominator() -> u16 {
        BPS_DENOMINATOR
    }

    /// Returns the negated value as a signed `i32`.
    #[must_use]
    pub const fn neg(self) -> i32 {
        -(self.0 as i32)
    }

    /// Returns the raw basis-point value.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl From<u16> for BPS {
    fn from(bps: u16) -> Self {
        Self::new(bps)
    }
}

impl From<BPS> for u16 {
    fn from(bps: BPS) -> Self {
        bps.get()
    }
}
