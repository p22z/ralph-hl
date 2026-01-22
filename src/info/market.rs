//! Market data info endpoints
//!
//! This module provides methods for querying market data from the Hyperliquid API,
//! including mid prices, order book snapshots, and candlestick data.

use std::collections::HashMap;

use crate::client::Client;
use crate::error::Result;
use crate::types::{
    AllMidsRequest, Candle, CandleInterval, CandleSnapshotRequest, L2BookRequest, L2BookResponse,
};

impl Client {
    /// Retrieve all mid prices
    ///
    /// Returns a map of all asset mid prices. The keys are asset names (e.g., "BTC", "ETH")
    /// and the values are the mid prices as strings.
    ///
    /// # Arguments
    /// * `dex` - Optional DEX identifier. If None, returns mid prices from the default perp DEX.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let mids = client.all_mids(None).await?;
    /// if let Some(btc_mid) = mids.get("BTC") {
    ///     println!("BTC mid price: {}", btc_mid);
    /// }
    /// ```
    pub async fn all_mids(&self, dex: Option<&str>) -> Result<HashMap<String, String>> {
        let request = AllMidsRequest {
            dex: dex.map(|s| s.to_string()),
            ..Default::default()
        };
        self.post_info(&request).await
    }

    /// Retrieve L2 order book snapshot
    ///
    /// Returns the current order book for a specific asset, including bid and ask levels.
    /// Each level contains price, size, and number of orders at that price.
    ///
    /// # Arguments
    /// * `coin` - The asset symbol (e.g., "BTC", "ETH")
    /// * `n_sig_figs` - Optional number of significant figures for price aggregation (2-5).
    ///                  When provided, prices are rounded/aggregated to this precision.
    /// * `mantissa` - Optional mantissa for price aggregation. Used together with n_sig_figs
    ///                to control aggregation level (e.g., mantissa=2, n_sig_figs=5 groups
    ///                prices like 49950, 49960, 49970...).
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// // Get raw order book
    /// let book = client.l2_book("BTC", None, None).await?;
    /// println!("Best bid: {} @ {}", book.levels.0[0].sz, book.levels.0[0].px);
    /// println!("Best ask: {} @ {}", book.levels.1[0].sz, book.levels.1[0].px);
    ///
    /// // Get aggregated order book with 3 significant figures
    /// let agg_book = client.l2_book("ETH", Some(3), None).await?;
    /// ```
    pub async fn l2_book(
        &self,
        coin: &str,
        n_sig_figs: Option<u8>,
        mantissa: Option<u8>,
    ) -> Result<L2BookResponse> {
        let mut request = L2BookRequest::new(coin);
        if let Some(figs) = n_sig_figs {
            request = request.with_sig_figs(figs);
        }
        if let Some(m) = mantissa {
            request = request.with_mantissa(m);
        }
        self.post_info(&request).await
    }

    /// Retrieve candlestick (OHLCV) data
    ///
    /// Returns historical candlestick data for an asset. Each candle contains
    /// open, high, low, close prices, volume, and number of trades.
    ///
    /// # Arguments
    /// * `coin` - The asset symbol (e.g., "BTC", "ETH")
    /// * `interval` - The candle interval (e.g., OneMinute, OneHour, OneDay)
    /// * `start_time` - Start timestamp in milliseconds
    /// * `end_time` - Optional end timestamp in milliseconds. If not provided,
    ///                returns candles up to the current time.
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::CandleInterval;
    ///
    /// let client = Client::mainnet()?;
    ///
    /// // Get hourly candles for the last 24 hours
    /// let now = std::time::SystemTime::now()
    ///     .duration_since(std::time::UNIX_EPOCH)
    ///     .unwrap()
    ///     .as_millis() as u64;
    /// let start = now - 24 * 60 * 60 * 1000; // 24 hours ago
    ///
    /// let candles = client.candle("BTC", CandleInterval::OneHour, start, None).await?;
    /// for candle in candles {
    ///     println!("Open: {}, Close: {}, Volume: {}", candle.o, candle.c, candle.v);
    /// }
    /// ```
    pub async fn candle(
        &self,
        coin: &str,
        interval: CandleInterval,
        start_time: u64,
        end_time: Option<u64>,
    ) -> Result<Vec<Candle>> {
        let mut request = CandleSnapshotRequest::new(coin, interval, start_time);
        if let Some(end) = end_time {
            request = request.with_end_time(end);
        }
        self.post_info(&request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use mockito::{Matcher, Server};
    use reqwest::Client as ReqwestClient;
    use serde::{de::DeserializeOwned, Serialize};

    /// Test client that can be configured with a custom base URL
    struct TestClient {
        http: ReqwestClient,
        base_url: String,
    }

    impl TestClient {
        fn new(base_url: &str) -> Self {
            Self {
                http: ReqwestClient::new(),
                base_url: base_url.to_string(),
            }
        }

        fn info_url(&self) -> String {
            format!("{}/info", self.base_url)
        }

        async fn post_info<T, R>(&self, request: &T) -> Result<R>
        where
            T: Serialize + ?Sized,
            R: DeserializeOwned,
        {
            let url = self.info_url();
            let response = self
                .http
                .post(&url)
                .json(request)
                .send()
                .await
                .map_err(Error::Http)?;

            let status = response.status();
            let body = response.text().await.map_err(Error::Http)?;

            if !status.is_success() {
                return Err(Error::Api(format!("HTTP {} - {}", status.as_u16(), body)));
            }

            serde_json::from_str(&body).map_err(|e| {
                if let Ok(error_obj) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(error_msg) = error_obj.get("error").and_then(|v| v.as_str()) {
                        return Error::Api(error_msg.to_string());
                    }
                }
                Error::Json(e)
            })
        }

        async fn all_mids(&self, dex: Option<&str>) -> Result<HashMap<String, String>> {
            let request = AllMidsRequest {
                dex: dex.map(|s| s.to_string()),
                ..Default::default()
            };
            self.post_info(&request).await
        }

        async fn l2_book(
            &self,
            coin: &str,
            n_sig_figs: Option<u8>,
            mantissa: Option<u8>,
        ) -> Result<L2BookResponse> {
            let mut request = L2BookRequest::new(coin);
            if let Some(figs) = n_sig_figs {
                request = request.with_sig_figs(figs);
            }
            if let Some(m) = mantissa {
                request = request.with_mantissa(m);
            }
            self.post_info(&request).await
        }

        async fn candle(
            &self,
            coin: &str,
            interval: CandleInterval,
            start_time: u64,
            end_time: Option<u64>,
        ) -> Result<Vec<Candle>> {
            let mut request = CandleSnapshotRequest::new(coin, interval, start_time);
            if let Some(end) = end_time {
                request = request.with_end_time(end);
            }
            self.post_info(&request).await
        }
    }

    #[tokio::test]
    async fn test_all_mids_without_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({"type": "allMids"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"BTC": "50000.0", "ETH": "3000.0", "SOL": "100.0"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let mids = client.all_mids(None).await.unwrap();

        assert_eq!(mids.len(), 3);
        assert_eq!(mids.get("BTC"), Some(&"50000.0".to_string()));
        assert_eq!(mids.get("ETH"), Some(&"3000.0".to_string()));
        assert_eq!(mids.get("SOL"), Some(&"100.0".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_all_mids_with_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "allMids", "dex": "customdex"}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"DOGE": "0.1", "SHIB": "0.00001"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let mids = client.all_mids(Some("customdex")).await.unwrap();

        assert_eq!(mids.len(), 2);
        assert_eq!(mids.get("DOGE"), Some(&"0.1".to_string()));
        assert_eq!(mids.get("SHIB"), Some(&"0.00001".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_all_mids_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({"type": "allMids"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let mids = client.all_mids(None).await.unwrap();

        assert!(mids.is_empty());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_l2_book_basic() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "l2Book", "coin": "BTC"}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "coin": "BTC",
                    "time": 1700000000000,
                    "levels": [
                        [
                            {"px": "50000.0", "sz": "1.5", "n": 10},
                            {"px": "49990.0", "sz": "2.0", "n": 15}
                        ],
                        [
                            {"px": "50010.0", "sz": "1.0", "n": 5},
                            {"px": "50020.0", "sz": "3.0", "n": 8}
                        ]
                    ]
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let book = client.l2_book("BTC", None, None).await.unwrap();

        assert_eq!(book.coin, "BTC");
        assert_eq!(book.time, 1700000000000);
        // Bids
        assert_eq!(book.levels.0.len(), 2);
        assert_eq!(book.levels.0[0].px, "50000.0");
        assert_eq!(book.levels.0[0].sz, "1.5");
        assert_eq!(book.levels.0[0].n, 10);
        assert_eq!(book.levels.0[1].px, "49990.0");
        // Asks
        assert_eq!(book.levels.1.len(), 2);
        assert_eq!(book.levels.1[0].px, "50010.0");
        assert_eq!(book.levels.1[0].sz, "1.0");
        assert_eq!(book.levels.1[0].n, 5);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_l2_book_with_sig_figs() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "l2Book", "coin": "ETH", "nSigFigs": 3}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "coin": "ETH",
                    "time": 1700000000000,
                    "levels": [
                        [{"px": "3000.0", "sz": "10.0", "n": 50}],
                        [{"px": "3010.0", "sz": "8.0", "n": 40}]
                    ]
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let book = client.l2_book("ETH", Some(3), None).await.unwrap();

        assert_eq!(book.coin, "ETH");
        assert_eq!(book.levels.0[0].px, "3000.0");
        assert_eq!(book.levels.0[0].n, 50);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_l2_book_with_mantissa() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "l2Book", "coin": "SOL", "nSigFigs": 4, "mantissa": 2}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "coin": "SOL",
                    "time": 1700000000000,
                    "levels": [
                        [{"px": "100.0", "sz": "50.0", "n": 20}],
                        [{"px": "100.2", "sz": "45.0", "n": 18}]
                    ]
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let book = client.l2_book("SOL", Some(4), Some(2)).await.unwrap();

        assert_eq!(book.coin, "SOL");
        assert_eq!(book.levels.0[0].px, "100.0");
        assert_eq!(book.levels.1[0].px, "100.2");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_l2_book_empty_levels() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "l2Book", "coin": "RARE"}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "coin": "RARE",
                    "time": 1700000000000,
                    "levels": [[], []]
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let book = client.l2_book("RARE", None, None).await.unwrap();

        assert_eq!(book.coin, "RARE");
        assert!(book.levels.0.is_empty());
        assert!(book.levels.1.is_empty());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_candle_basic() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "candleSnapshot",
                "req": {
                    "coin": "BTC",
                    "interval": "1h",
                    "startTime": 1700000000000_u64
                }
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "T": 1700003600000,
                        "c": "50100.0",
                        "h": "50200.0",
                        "i": "1h",
                        "l": "49900.0",
                        "n": 1500,
                        "o": "50000.0",
                        "s": "BTC",
                        "t": 1700000000000,
                        "v": "100.5"
                    },
                    {
                        "T": 1700007200000,
                        "c": "50300.0",
                        "h": "50400.0",
                        "i": "1h",
                        "l": "50000.0",
                        "n": 1200,
                        "o": "50100.0",
                        "s": "BTC",
                        "t": 1700003600000,
                        "v": "80.2"
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let candles = client
            .candle("BTC", CandleInterval::OneHour, 1700000000000, None)
            .await
            .unwrap();

        assert_eq!(candles.len(), 2);

        // First candle
        assert_eq!(candles[0].s, "BTC");
        assert_eq!(candles[0].i, "1h");
        assert_eq!(candles[0].t, 1700000000000);
        assert_eq!(candles[0].close_time, 1700003600000);
        assert_eq!(candles[0].o, "50000.0");
        assert_eq!(candles[0].h, "50200.0");
        assert_eq!(candles[0].l, "49900.0");
        assert_eq!(candles[0].c, "50100.0");
        assert_eq!(candles[0].v, "100.5");
        assert_eq!(candles[0].n, 1500);

        // Second candle
        assert_eq!(candles[1].o, "50100.0");
        assert_eq!(candles[1].c, "50300.0");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_candle_with_end_time() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "candleSnapshot",
                "req": {
                    "coin": "ETH",
                    "interval": "15m",
                    "startTime": 1700000000000_u64,
                    "endTime": 1700086400000_u64
                }
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "T": 1700000900000,
                        "c": "3010.0",
                        "h": "3020.0",
                        "i": "15m",
                        "l": "2990.0",
                        "n": 500,
                        "o": "3000.0",
                        "s": "ETH",
                        "t": 1700000000000,
                        "v": "500.0"
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let candles = client
            .candle(
                "ETH",
                CandleInterval::FifteenMinutes,
                1700000000000,
                Some(1700086400000),
            )
            .await
            .unwrap();

        assert_eq!(candles.len(), 1);
        assert_eq!(candles[0].s, "ETH");
        assert_eq!(candles[0].i, "15m");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_candle_different_intervals() {
        // Test 1 minute interval
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "candleSnapshot",
                "req": {
                    "coin": "SOL",
                    "interval": "1m",
                    "startTime": 1700000000000_u64
                }
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let candles = client
            .candle("SOL", CandleInterval::OneMinute, 1700000000000, None)
            .await
            .unwrap();
        assert!(candles.is_empty());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_candle_daily_interval() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "candleSnapshot",
                "req": {
                    "coin": "BTC",
                    "interval": "1d",
                    "startTime": 1700000000000_u64
                }
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "T": 1700086400000,
                        "c": "51000.0",
                        "h": "52000.0",
                        "i": "1d",
                        "l": "49000.0",
                        "n": 50000,
                        "o": "50000.0",
                        "s": "BTC",
                        "t": 1700000000000,
                        "v": "5000.0"
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let candles = client
            .candle("BTC", CandleInterval::OneDay, 1700000000000, None)
            .await
            .unwrap();

        assert_eq!(candles.len(), 1);
        assert_eq!(candles[0].i, "1d");
        assert_eq!(candles[0].n, 50000);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_all_mids_error_handling() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.all_mids(None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("500"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_l2_book_error_handling() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid coin"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.l2_book("INVALID", None, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid coin"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_candle_error_handling() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(429)
            .with_body("Rate limit exceeded")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .candle("BTC", CandleInterval::OneHour, 1700000000000, None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("429"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_all_mids_request_serialization() {
        let request = AllMidsRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"allMids"}"#);
    }

    #[tokio::test]
    async fn test_all_mids_request_serialization_with_dex() {
        let mut request = AllMidsRequest::default();
        request.dex = Some("testdex".to_string());
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"allMids","dex":"testdex"}"#);
    }

    #[tokio::test]
    async fn test_l2_book_request_serialization() {
        let request = L2BookRequest::new("BTC");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"l2Book","coin":"BTC"}"#);
    }

    #[tokio::test]
    async fn test_l2_book_request_serialization_with_params() {
        let request = L2BookRequest::new("ETH").with_sig_figs(3).with_mantissa(5);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"nSigFigs\":3"));
        assert!(json.contains("\"mantissa\":5"));
    }

    #[tokio::test]
    async fn test_candle_request_serialization() {
        let request = CandleSnapshotRequest::new("BTC", CandleInterval::OneHour, 1700000000000);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"candleSnapshot\""));
        assert!(json.contains("\"coin\":\"BTC\""));
        assert!(json.contains("\"interval\":\"1h\""));
        assert!(json.contains("\"startTime\":1700000000000"));
    }

    #[tokio::test]
    async fn test_candle_request_serialization_with_end_time() {
        let request = CandleSnapshotRequest::new("ETH", CandleInterval::FiveMinutes, 1700000000000)
            .with_end_time(1700100000000);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"endTime\":1700100000000"));
    }
}
