use std::future::Future;

use tokio::sync::watch;
use uniswap_sdk_core::{entities::BaseCurrency, prelude::{BaseCurrencyCore, Token}};

use crate::{
    objects::TokenExt,
    strategies::price_source::{PriceSource, PriceSourceError},
};

/// Sources a constant `1.0` USD price for supported stablecoins.
#[derive(Clone, Default)]
pub struct StablePriceSource;

impl StablePriceSource {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PriceSource for StablePriceSource {
    #[allow(clippy::manual_async_fn)]
    fn price(
        &self,
        token: Token,
    ) -> impl Future<Output = Result<watch::Receiver<f64>, PriceSourceError>> + Send {
        async move {
            if !token.is_stablecoin() {
                return Err(PriceSourceError::UnsupportedToken(token.symbol().unwrap_or(&token.address().to_string()).to_owned()));
            }

            let (tx, rx) = watch::channel(1.0);
            // Keep the sender alive until all receivers are dropped so the channel stays open.
            tokio::spawn(async move {
                tx.closed().await;
            });
            Ok(rx)
        }
    }
}
