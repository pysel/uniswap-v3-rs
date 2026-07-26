use std::future::Future;

use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use uniswap_sdk_core::prelude::{BaseCurrencyCore, Token};

use crate::strategies::price_source::{PriceSource, PriceSourceError};

const BINANCE_STREAM_URL: &str = "wss://stream.binance.com:9443/ws";

#[derive(Default)]
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
}

impl PriceSource for BinancePriceSource {
    fn price(
        &self,
        token: Token,
    ) -> impl Future<Output = Result<mpsc::UnboundedReceiver<f64>, PriceSourceError>> + Send {
        async move {
            let symbol = Self::symbol(&token)?;
            let url = format!("{BINANCE_STREAM_URL}/{symbol}@bookTicker");
            let (stream, _) = connect_async(url)
                .await
                .map_err(|error| PriceSourceError::SubscriptionError(error.to_string()))?;
            let (sender, receiver) = mpsc::unbounded_channel();

            tokio::spawn(async move {
                let mut stream = stream;

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
                    let (Ok(bid), Ok(ask)) = (
                        book_ticker.bid.parse::<f64>(),
                        book_ticker.ask.parse::<f64>(),
                    ) else {
                        break;
                    };

                    if sender.send((bid + ask) / 2.0).is_err() {
                        break;
                    }
                }
            });

            Ok(receiver)
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
