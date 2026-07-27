use std::future::Future;

use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::watch;
use tokio_tungstenite::connect_async;
use uniswap_sdk_core::prelude::{BaseCurrencyCore, Token};

use crate::strategies::price_source::{PriceSource, PriceSourceError};

const BINANCE_STREAM_URL: &str = "wss://stream.binance.com:9443/ws";

#[derive(Clone, Default)]
pub struct BinancePriceSource;

impl BinancePriceSource {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn symbol(token: &Token) -> Result<String, PriceSourceError> {
        let symbol = token.symbol().ok_or(PriceSourceError::TokenSymbolMissing)?;
        let symbol = symbol.to_uppercase();
        let base = match symbol.as_str() {
            "WETH" => "ETH",
            "WBTC" => "BTC",
            "USDT" => return Err(PriceSourceError::UnsupportedToken(symbol.clone())),
            symbol => symbol,
        };

        Ok(format!("{base}usdt"))
    }

    fn midpoint(book_ticker: &BookTicker) -> Option<f64> {
        let bid = book_ticker.bid.parse::<f64>().ok()?;
        let ask = book_ticker.ask.parse::<f64>().ok()?;
        Some((bid + ask) / 2.0)
    }
}

impl PriceSource for BinancePriceSource {
    #[allow(clippy::manual_async_fn)]
    fn price(
        &self,
        token: Token,
    ) -> impl Future<Output = Result<watch::Receiver<f64>, PriceSourceError>> + Send {
        async move {
            let symbol = Self::symbol(&token)?;
            let url = format!("{BINANCE_STREAM_URL}/{symbol}@bookTicker");
            let (stream, _) = connect_async(url)
                .await
                .map_err(|error| PriceSourceError::SubscriptionError(error.to_string()))?;
            let mut stream = stream;

            let initial = loop {
                let message = stream.next().await.ok_or_else(|| {
                    PriceSourceError::SubscriptionError("binance stream closed before first price".into())
                })?;
                let message = message.map_err(|error| {
                    PriceSourceError::SubscriptionError(error.to_string())
                })?;
                let Ok(text) = message.into_text() else {
                    continue;
                };
                let Ok(book_ticker) = serde_json::from_str::<BookTicker>(&text) else {
                    continue;
                };
                if let Some(mid) = Self::midpoint(&book_ticker) {
                    break mid;
                }
            };

            let (tx, rx) = watch::channel(initial);

            tokio::spawn(async move {
                while let Some(message) = stream.next().await {
                    let Ok(message) = message else {
                        break;
                    };
                    let Ok(text) = message.into_text() else {
                        continue;
                    };
                    let Ok(book_ticker) = serde_json::from_str::<BookTicker>(&text) else {
                        break;
                    };
                    let Some(mid) = Self::midpoint(&book_ticker) else {
                        break;
                    };

                    if tx.send(mid).is_err() {
                        break;
                    }
                }
            });

            Ok(rx)
        }
    }
}

#[derive(Deserialize)]
struct BookTicker {
    #[serde(rename = "a")]
    ask: String,
    #[serde(rename = "b")]
    bid: String,
}
