use alloy_primitives::{U256, utils::format_units};
use uniswap_sdk_core::prelude::Token;

use crate::objects::TokenExt;

use super::HedgerError;

/// USD/USDC atomic decimal precision used by [`Hedge`](super::Hedge) margin and fee fields.
pub(crate) const USD_DECIMALS: u8 = 6;

pub(crate) fn parse_f64(value: &str, context: &str) -> Result<f64, HedgerError> {
    value
        .parse::<f64>()
        .map_err(|error| HedgerError::NumericConversion(format!("{context}: {value}: {error}")))
}

/// Converts an ERC-20 raw amount into a human `f64` using `decimals`.
pub(crate) fn raw_to_f64(raw: U256, decimals: u8) -> Result<f64, HedgerError> {
    let formatted = format_units(raw, decimals)
        .map_err(|error| HedgerError::NumericConversion(error.to_string()))?;
    parse_f64(&formatted, "raw token amount")
}

/// Converts a human token amount into ERC-20 raw units via [`TokenExt::from_amount`].
pub(crate) fn f64_to_raw(token: &Token, amount: f64) -> Result<U256, HedgerError> {
    if !amount.is_finite() || amount < 0.0 {
        return Err(HedgerError::NumericConversion(format!(
            "amount must be finite and non-negative: {amount}"
        )));
    }
    Ok(token.from_amount(amount))
}

/// Converts a USD amount into 6-decimal atomic units.
pub(crate) fn usd_to_atomic(usd: f64) -> Result<U256, HedgerError> {
    if !usd.is_finite() || usd < 0.0 {
        return Err(HedgerError::NumericConversion(format!(
            "usd amount must be finite and non-negative: {usd}"
        )));
    }
    let scale = 10f64.powi(i32::from(USD_DECIMALS));
    let scaled = (usd * scale).round();
    if !scaled.is_finite() || scaled > u128::MAX as f64 {
        return Err(HedgerError::NumericConversion(format!(
            "usd amount out of range: {usd}"
        )));
    }
    Ok(U256::from(scaled as u128))
}
