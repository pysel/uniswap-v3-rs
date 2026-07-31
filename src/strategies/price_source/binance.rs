use std::{future::Future, time::Duration};

use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::watch;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tracing::{info, warn};
use uniswap_sdk_core::prelude::{BaseCurrencyCore, Token};

use crate::strategies::price_source::{PriceSource, PriceSourceError};

const BINANCE_STREAM_URL: &str = "wss://stream.binance.com:9443/ws";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const FIRST_TICK_TIMEOUT: Duration = Duration::from_secs(10);

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

        // Binance combined/raw stream names are lowercase (e.g. ethusdt@bookTicker).
        Ok(format!("{base}usdt").to_lowercase())
    }

    fn midpoint(book_ticker: &BookTicker) -> Option<f64> {
        let bid = book_ticker.bid.parse::<f64>().ok()?;
        let ask = book_ticker.ask.parse::<f64>().ok()?;
        Some((bid + ask) / 2.0)
    }

    async fn connect_with_timeout(
        url: &str,
    ) -> Result<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, PriceSourceError> {
        let (stream, _) = tokio::time::timeout(CONNECT_TIMEOUT, connect_async(url))
            .await
            .map_err(|_| PriceSourceError::SubscriptionError(format!("connect timeout: {url}")))?
            .map_err(|error| PriceSourceError::SubscriptionError(error.to_string()))?;

        Ok(stream)
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
            info!(%url, "connecting binance bookTicker stream");
            let mut stream = Self::connect_with_timeout(&url).await?;

            let initial = tokio::time::timeout(FIRST_TICK_TIMEOUT, async {
                loop {
                    let message = stream.next().await.ok_or_else(|| {
                        PriceSourceError::SubscriptionError(
                            "binance stream closed before first price".into(),
                        )
                    })?;
                    let message = message
                        .map_err(|error| PriceSourceError::SubscriptionError(error.to_string()))?;
                    let Ok(text) = message.into_text() else {
                        continue;
                    };
                    let Ok(book_ticker) = serde_json::from_str::<BookTicker>(&text) else {
                        continue;
                    };
                    if let Some(mid) = Self::midpoint(&book_ticker) {
                        break Ok::<f64, PriceSourceError>(mid);
                    }
                }
            })
            .await
            .map_err(|_| {
                PriceSourceError::SubscriptionError(format!("first bookTicker timeout: {url}"))
            })??;

            let (tx, rx) = watch::channel(initial);

            tokio::spawn(async move {
                while let Some(message) = stream.next().await {
                    let Ok(message) = message else {
                        if let Err(error) = message {
                            warn!("binance stream error: {}", error);
                            if let Ok(new_stream) = Self::connect_with_timeout(&url).await {
                                stream = new_stream;
                                continue;
                            }

                            break;
                        }
                        continue;
                    };

                    let Ok(text) = message.into_text() else {
                        continue;
                    };
                    let Ok(book_ticker) = serde_json::from_str::<BookTicker>(&text) else {
                        continue;
                    };
                    let Some(mid) = Self::midpoint(&book_ticker) else {
                        continue;
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

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use uniswap_sdk_core::prelude::Token;

    use super::BinancePriceSource;

    fn token(symbol: &str) -> Token {
        Token::new(1, Address::ZERO, 18, Some(symbol.into()), None, 0, 0)
    }

    #[test]
    fn stream_symbol_is_lowercase() {
        assert_eq!(
            BinancePriceSource::symbol(&token("WETH")).unwrap(),
            "ethusdt"
        );
        assert_eq!(
            BinancePriceSource::symbol(&token("WBTC")).unwrap(),
            "btcusdt"
        );
        assert_eq!(
            BinancePriceSource::symbol(&token("UNI")).unwrap(),
            "uniusdt"
        );
    }
}
