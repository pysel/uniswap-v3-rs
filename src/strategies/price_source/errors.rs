use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PriceSourceError {
    #[error("token symbol is missing")]
    TokenSymbolMissing,
    #[error("token is unsupported by this price source: {0}")]
    UnsupportedToken(String),
    #[error("price subscription failed: {0}")]
    SubscriptionError(String),
}
