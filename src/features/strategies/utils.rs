use crate::calltypes::BPS;

pub(crate) fn apply_bps_below(price: f64, bps: BPS) -> f64 {
    price * (1.0 - f64::from(bps.get()) / f64::from(BPS::denominator()))
}

pub(crate) fn apply_bps_above(price: f64, bps: BPS) -> f64 {
    price * (1.0 + f64::from(bps.get()) / f64::from(BPS::denominator()))
}
