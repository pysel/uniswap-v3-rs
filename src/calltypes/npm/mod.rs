mod burn_position_response;
mod close_position_params;
mod collect_params;
mod create_and_initialize_pool_response;
mod decrease_liquidity_params;
mod increase_liquidity_params;
mod mint_params;

use std::time::SystemTime;

use alloy::primitives::U256;

pub use burn_position_response::BurnPositionResponse;
pub use close_position_params::{
    ClosePositionParams, ClosePositionParamsBuilder, ClosePositionResponse, ClosePositionResult,
};
pub use collect_params::{
    CollectParams, CollectParamsBuilder, CollectPositionResponse, CollectPositionResult,
};
pub use create_and_initialize_pool_response::CreateAndInitializePoolResponse;
pub use decrease_liquidity_params::{
    DecreaseLiquidityParams, DecreaseLiquidityParamsBuilder, DecreaseLiquidityResponse,
    DecreaseLiquidityResult,
};
pub use increase_liquidity_params::{
    IncreaseLiquidityParams, IncreaseLiquidityParamsBuilder, IncreaseLiquidityResponse,
    IncreaseLiquidityResult,
};
pub use mint_params::{
    CreatePositionParams, CreatePositionParamsBuilder, CreatePositionResponse, CreatePositionResult,
};

const DEFAULT_DEADLINE_FROM_NOW_SECS: u64 = 60 * 60 * 24 * 30; // 30 days

/// Default NPM deadline: ~30 days from now.
#[must_use]
pub fn default_deadline() -> Option<U256> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|now| U256::from(now.as_secs() + DEFAULT_DEADLINE_FROM_NOW_SECS))
}
