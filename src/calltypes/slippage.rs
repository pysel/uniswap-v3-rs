use alloy::primitives::U256;

use crate::errors::UniswapV3Error;

use super::BPS;

/// Reduces `amount` by `bps` (for exact-input `amountOutMinimum`).
pub(crate) fn apply_negative_slippage(amount: U256, bps: BPS) -> Result<U256, UniswapV3Error> {
    let bps = bps.get();
    if bps > BPS::denominator() {
        return Err(UniswapV3Error::Math(format!(
            "slippage bps {bps} exceeds {}", BPS::denominator()
        )));
    }

    let numerator = U256::from(BPS::denominator() - bps);
    let denominator = U256::from(BPS::denominator());
    amount
        .checked_mul(numerator)
        .and_then(|product| product.checked_div(denominator))
        .ok_or_else(|| UniswapV3Error::Math("slippage calculation overflowed".to_string()))
}

/// Increases `amount` by `bps` (for exact-output `amountInMaximum`).
pub(crate) fn apply_positive_slippage(amount: U256, bps: BPS) -> Result<U256, UniswapV3Error> {
    let bps = bps.get();
    let numerator = U256::from(BPS::denominator())
        .checked_add(U256::from(bps))
        .ok_or_else(|| UniswapV3Error::Math("slippage bps overflowed".to_string()))?;
    let denominator = U256::from(BPS::denominator());
    amount
        .checked_mul(numerator)
        .and_then(|product| product.checked_div(denominator))
        .ok_or_else(|| UniswapV3Error::Math("slippage calculation overflowed".to_string()))
}
