use thiserror::Error;

use crate::strategies::price_source::PriceSourceError;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StrategyError {
    #[error("strategy is already running")]
    AlreadyRunning,
    #[error(transparent)]
    PriceSource(#[from] PriceSourceError),
}
