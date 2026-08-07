use tokio::task::AbortHandle;
use tracing::info;

use crate::calltypes::ClosePositionParams;
use crate::{calltypes::BPS, client::UniswapV3Client};
use crate::strategies::StrategyError;

pub(crate) fn apply_bps_below(price: f64, bps: BPS) -> f64 {
    price * (1.0 - f64::from(bps.get()) / f64::from(BPS::denominator()))
}

pub(crate) fn apply_bps_above(price: f64, bps: BPS) -> f64 {
    price * (1.0 + f64::from(bps.get()) / f64::from(BPS::denominator()))
}

pub async fn abort_strategy(
    client: &UniswapV3Client,
    handle: AbortHandle,
) -> Result<(), StrategyError> {
    handle.abort();

    let positions = client
        .get_positions(client.signer_address().unwrap())
        .await?;

    info!(
        positions = %positions.len(),
        "closing positions"
    );
    for position in positions {
        let params = ClosePositionParams::builder()
            .recipient(client.signer_address().unwrap())
            .then_default()
            .build()?;
        let response = client.close_position(&position, params).await?;
        let _ = response.amounts.await?;
    }

    Ok(())
}