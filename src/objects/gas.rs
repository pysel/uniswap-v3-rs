use alloy::{
    contract::{CallBuilder, CallDecoder},
    network::Ethereum,
    providers::{PendingTransactionBuilder, Provider},
};

use crate::errors::UniswapV3Error;

/// Scales an `eth_estimateGas` result by `multiplier`, rounding up.
pub(crate) fn scale_gas(gas: u64, multiplier: f64) -> u64 {
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return gas;
    }

    let scaled = (gas as f64) * multiplier;
    if !scaled.is_finite() || scaled >= u64::MAX as f64 {
        return u64::MAX;
    }

    scaled.ceil() as u64
}

/// Estimates gas for `call`, optionally pads with `gas_multiplier`, then sends.
pub(crate) async fn send_with_gas_multiplier<P, D>(
    call: CallBuilder<P, D, Ethereum>,
    gas_multiplier: Option<f64>,
) -> Result<PendingTransactionBuilder<Ethereum>, UniswapV3Error>
where
    P: Provider,
    D: CallDecoder + Send,
{
    let mut gas = call
        .estimate_gas()
        .await
        .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

    if let Some(multiplier) = gas_multiplier {
        gas = scale_gas(gas, multiplier);
    }

    call.gas(gas)
        .send()
        .await
        .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
}
