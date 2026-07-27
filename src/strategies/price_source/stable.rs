use std::future::Future;

use tokio::sync::watch;
use uniswap_sdk_core::prelude::{BaseCurrencyCore, Token};

use crate::strategies::price_source::{PriceSource, PriceSourceError};

/// Sources a constant `1.0` USD price for supported stablecoins.
#[derive(Clone, Default)]
pub struct StablePriceSource;

impl StablePriceSource {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn ensure_stable(token: &Token) -> Result<(), PriceSourceError> {
        let symbol = token.symbol().ok_or(PriceSourceError::TokenSymbolMissing)?;
        let symbol = symbol.to_uppercase();
        match symbol.as_str() {
            "USDT" | "USDC" | "DAI" | "USDE" | "USDG" | "USDT0" => Ok(()),
            _ => Err(PriceSourceError::UnsupportedToken(symbol)),
        }
    }
}

impl PriceSource for StablePriceSource {
    #[allow(clippy::manual_async_fn)]
    fn price(
        &self,
        token: Token,
    ) -> impl Future<Output = Result<watch::Receiver<f64>, PriceSourceError>> + Send {
        async move {
            Self::ensure_stable(&token)?;
            let (tx, rx) = watch::channel(1.0);
            // Keep the sender alive until all receivers are dropped so the channel stays open.
            tokio::spawn(async move {
                tx.closed().await;
            });
            Ok(rx)
        }
    }
}
