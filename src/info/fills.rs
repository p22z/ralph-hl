//! User fills info endpoints
//!
//! This module provides methods for querying user fill (trade) information from the Hyperliquid API.

use crate::client::Client;
use crate::error::Result;
use crate::types::{
    TwapSliceFill, UserFill, UserFillsByTimeRequest, UserFillsRequest, UserTwapSliceFillsRequest,
};

impl Client {
    /// Retrieve recent fills for a user
    ///
    /// Returns recent fills (trades) for the specified user.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    /// * `dex` - Optional DEX identifier. If None, uses the default perp DEX.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let fills = client.user_fills("0x1234...", None).await?;
    /// for fill in fills {
    ///     println!("{}: {} {} @ {} (pnl: {})", fill.coin, fill.side, fill.sz, fill.px, fill.closed_pnl);
    /// }
    /// ```
    pub async fn user_fills(&self, user: &str, dex: Option<&str>) -> Result<Vec<UserFill>> {
        let mut request = UserFillsRequest::new(user);
        if let Some(d) = dex {
            request = request.with_dex(d);
        }
        self.post_info(&request).await
    }

    /// Retrieve fills for a user within a time range
    ///
    /// Returns fills (trades) for the specified user within the given time range.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    /// * `start_time` - Start time in milliseconds since Unix epoch
    /// * `end_time` - Optional end time in milliseconds since Unix epoch. If None, returns fills up to now.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let start = 1700000000000u64; // Nov 14, 2023
    /// let end = 1700100000000u64;
    /// let fills = client.user_fills_by_time("0x1234...", start, Some(end)).await?;
    /// for fill in fills {
    ///     println!("{}: {} at time {}", fill.coin, fill.sz, fill.time);
    /// }
    /// ```
    pub async fn user_fills_by_time(
        &self,
        user: &str,
        start_time: u64,
        end_time: Option<u64>,
    ) -> Result<Vec<UserFill>> {
        let mut request = UserFillsByTimeRequest::new(user, start_time);
        if let Some(t) = end_time {
            request = request.with_end_time(t);
        }
        self.post_info(&request).await
    }

    /// Retrieve TWAP slice fills for a user
    ///
    /// Returns fills from TWAP (Time-Weighted Average Price) order slices for the specified user.
    /// TWAP orders are split into multiple slices executed over time.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let fills = client.user_twap_slice_fills("0x1234...").await?;
    /// for fill in fills {
    ///     println!("TWAP {}: {} {} @ {}", fill.twap_id, fill.coin, fill.sz, fill.px);
    /// }
    /// ```
    pub async fn user_twap_slice_fills(&self, user: &str) -> Result<Vec<TwapSliceFill>> {
        let request = UserTwapSliceFillsRequest::new(user);
        self.post_info(&request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::types::Side;
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

        async fn user_fills(&self, user: &str, dex: Option<&str>) -> Result<Vec<UserFill>> {
            let mut request = UserFillsRequest::new(user);
            if let Some(d) = dex {
                request = request.with_dex(d);
            }
            self.post_info(&request).await
        }

        async fn user_fills_by_time(
            &self,
            user: &str,
            start_time: u64,
            end_time: Option<u64>,
        ) -> Result<Vec<UserFill>> {
            let mut request = UserFillsByTimeRequest::new(user, start_time);
            if let Some(t) = end_time {
                request = request.with_end_time(t);
            }
            self.post_info(&request).await
        }

        async fn user_twap_slice_fills(&self, user: &str) -> Result<Vec<TwapSliceFill>> {
            let request = UserTwapSliceFillsRequest::new(user);
            self.post_info(&request).await
        }
    }

    // ==================== User Fills Tests ====================

    #[tokio::test]
    async fn test_user_fills_without_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFills",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "closedPnl": "100.50",
                        "coin": "BTC",
                        "crossed": true,
                        "dir": "Open Long",
                        "hash": "0xabc123",
                        "oid": 12345,
                        "px": "50000.0",
                        "side": "B",
                        "startPosition": "0.0",
                        "sz": "0.1",
                        "time": 1700000000000,
                        "fee": "5.0",
                        "feeToken": "USDC",
                        "tid": 99999
                    },
                    {
                        "closedPnl": "-50.25",
                        "coin": "ETH",
                        "crossed": false,
                        "dir": "Close Short",
                        "hash": "0xdef456",
                        "oid": 12346,
                        "px": "3000.0",
                        "side": "A",
                        "startPosition": "1.0",
                        "sz": "1.0",
                        "time": 1700000001000,
                        "fee": "3.0",
                        "feeToken": "USDC",
                        "tid": 99998
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fills = client
            .user_fills("0x1234567890123456789012345678901234567890", None)
            .await
            .unwrap();

        assert_eq!(fills.len(), 2);
        assert_eq!(fills[0].coin, "BTC");
        assert_eq!(fills[0].px, "50000.0");
        assert_eq!(fills[0].side, Side::Buy);
        assert_eq!(fills[0].sz, "0.1");
        assert_eq!(fills[0].closed_pnl, "100.50");
        assert!(fills[0].crossed);
        assert_eq!(fills[0].dir, "Open Long");
        assert_eq!(fills[0].fee, "5.0");
        assert_eq!(fills[0].fee_token, "USDC");
        assert_eq!(fills[1].coin, "ETH");
        assert_eq!(fills[1].side, Side::Sell);
        assert!(!fills[1].crossed);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_fills_with_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFills",
                "user": "0x1234567890123456789012345678901234567890",
                "dex": "customdex"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fills = client
            .user_fills(
                "0x1234567890123456789012345678901234567890",
                Some("customdex"),
            )
            .await
            .unwrap();

        assert_eq!(fills.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_fills_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFills",
                "user": "0xdeadbeef12345678901234567890123456789012"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fills = client
            .user_fills("0xdeadbeef12345678901234567890123456789012", None)
            .await
            .unwrap();

        assert_eq!(fills.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_fills_with_builder_fee() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFills",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "closedPnl": "0.0",
                        "coin": "BTC",
                        "crossed": true,
                        "dir": "Open Long",
                        "hash": "0xabc123",
                        "oid": 12345,
                        "px": "50000.0",
                        "side": "B",
                        "startPosition": "0.0",
                        "sz": "0.1",
                        "time": 1700000000000,
                        "fee": "5.0",
                        "feeToken": "USDC",
                        "tid": 99999,
                        "builderFee": "1.0"
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fills = client
            .user_fills("0x1234567890123456789012345678901234567890", None)
            .await
            .unwrap();

        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].builder_fee, Some("1.0".to_string()));

        mock.assert_async().await;
    }

    // ==================== User Fills By Time Tests ====================

    #[tokio::test]
    async fn test_user_fills_by_time_without_end_time() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFillsByTime",
                "user": "0x1234567890123456789012345678901234567890",
                "startTime": 1700000000000u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "closedPnl": "0.0",
                        "coin": "BTC",
                        "crossed": true,
                        "dir": "Open Long",
                        "hash": "0xabc123",
                        "oid": 12345,
                        "px": "50000.0",
                        "side": "B",
                        "startPosition": "0.0",
                        "sz": "0.1",
                        "time": 1700000000000,
                        "fee": "5.0",
                        "feeToken": "USDC",
                        "tid": 99999
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fills = client
            .user_fills_by_time(
                "0x1234567890123456789012345678901234567890",
                1700000000000,
                None,
            )
            .await
            .unwrap();

        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].coin, "BTC");
        assert_eq!(fills[0].time, 1700000000000);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_fills_by_time_with_end_time() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFillsByTime",
                "user": "0x1234567890123456789012345678901234567890",
                "startTime": 1700000000000u64,
                "endTime": 1700100000000u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "closedPnl": "100.0",
                        "coin": "ETH",
                        "crossed": false,
                        "dir": "Close Long",
                        "hash": "0xdef456",
                        "oid": 12346,
                        "px": "3000.0",
                        "side": "A",
                        "startPosition": "1.0",
                        "sz": "1.0",
                        "time": 1700050000000,
                        "fee": "3.0",
                        "feeToken": "USDC",
                        "tid": 99998
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fills = client
            .user_fills_by_time(
                "0x1234567890123456789012345678901234567890",
                1700000000000,
                Some(1700100000000),
            )
            .await
            .unwrap();

        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].coin, "ETH");
        assert_eq!(fills[0].time, 1700050000000);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_fills_by_time_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFillsByTime",
                "user": "0x1234567890123456789012345678901234567890",
                "startTime": 1700000000000u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fills = client
            .user_fills_by_time(
                "0x1234567890123456789012345678901234567890",
                1700000000000,
                None,
            )
            .await
            .unwrap();

        assert_eq!(fills.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_fills_by_time_multiple_fills() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFillsByTime",
                "user": "0x1234567890123456789012345678901234567890",
                "startTime": 1700000000000u64,
                "endTime": 1700200000000u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "closedPnl": "0.0",
                        "coin": "BTC",
                        "crossed": true,
                        "dir": "Open Long",
                        "hash": "0xabc123",
                        "oid": 12345,
                        "px": "50000.0",
                        "side": "B",
                        "startPosition": "0.0",
                        "sz": "0.1",
                        "time": 1700000000000,
                        "fee": "5.0",
                        "feeToken": "USDC",
                        "tid": 99999
                    },
                    {
                        "closedPnl": "50.0",
                        "coin": "BTC",
                        "crossed": true,
                        "dir": "Close Long",
                        "hash": "0xabc124",
                        "oid": 12347,
                        "px": "50500.0",
                        "side": "A",
                        "startPosition": "0.1",
                        "sz": "0.1",
                        "time": 1700100000000,
                        "fee": "5.05",
                        "feeToken": "USDC",
                        "tid": 99997
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fills = client
            .user_fills_by_time(
                "0x1234567890123456789012345678901234567890",
                1700000000000,
                Some(1700200000000),
            )
            .await
            .unwrap();

        assert_eq!(fills.len(), 2);
        assert_eq!(fills[0].time, 1700000000000);
        assert_eq!(fills[1].time, 1700100000000);
        assert_eq!(fills[0].dir, "Open Long");
        assert_eq!(fills[1].dir, "Close Long");

        mock.assert_async().await;
    }

    // ==================== User TWAP Slice Fills Tests ====================

    #[tokio::test]
    async fn test_user_twap_slice_fills() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userTwapSliceFills",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "fill": {
                            "closedPnl": "0.0",
                            "coin": "BTC",
                            "crossed": true,
                            "dir": "Open Long",
                            "hash": "0xabc123",
                            "oid": 12345,
                            "px": "50000.0",
                            "side": "B",
                            "startPosition": "0.0",
                            "sz": "0.01",
                            "time": 1700000000000,
                            "fee": "0.5",
                            "feeToken": "USDC",
                            "tid": 99999
                        },
                        "twapId": 1001
                    },
                    {
                        "fill": {
                            "closedPnl": "0.0",
                            "coin": "BTC",
                            "crossed": true,
                            "dir": "Open Long",
                            "hash": "0xabc124",
                            "oid": 12346,
                            "px": "50100.0",
                            "side": "B",
                            "startPosition": "0.01",
                            "sz": "0.01",
                            "time": 1700000060000,
                            "fee": "0.501",
                            "feeToken": "USDC",
                            "tid": 99998
                        },
                        "twapId": 1001
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fills = client
            .user_twap_slice_fills("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(fills.len(), 2);
        assert_eq!(fills[0].twap_id, 1001);
        assert_eq!(fills[0].fill.coin, "BTC");
        assert_eq!(fills[0].fill.px, "50000.0");
        assert_eq!(fills[0].fill.sz, "0.01");
        assert_eq!(fills[1].twap_id, 1001);
        assert_eq!(fills[1].fill.px, "50100.0");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_twap_slice_fills_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userTwapSliceFills",
                "user": "0xdeadbeef12345678901234567890123456789012"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fills = client
            .user_twap_slice_fills("0xdeadbeef12345678901234567890123456789012")
            .await
            .unwrap();

        assert_eq!(fills.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_twap_slice_fills_multiple_twaps() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userTwapSliceFills",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "fill": {
                            "closedPnl": "0.0",
                            "coin": "BTC",
                            "crossed": true,
                            "dir": "Open Long",
                            "hash": "0xabc123",
                            "oid": 12345,
                            "px": "50000.0",
                            "side": "B",
                            "startPosition": "0.0",
                            "sz": "0.01",
                            "time": 1700000000000,
                            "fee": "0.5",
                            "feeToken": "USDC",
                            "tid": 99999
                        },
                        "twapId": 1001
                    },
                    {
                        "fill": {
                            "closedPnl": "0.0",
                            "coin": "ETH",
                            "crossed": false,
                            "dir": "Open Short",
                            "hash": "0xdef456",
                            "oid": 12347,
                            "px": "3000.0",
                            "side": "A",
                            "startPosition": "0.0",
                            "sz": "1.0",
                            "time": 1700000000000,
                            "fee": "3.0",
                            "feeToken": "USDC",
                            "tid": 99997
                        },
                        "twapId": 1002
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fills = client
            .user_twap_slice_fills("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(fills.len(), 2);
        assert_eq!(fills[0].twap_id, 1001);
        assert_eq!(fills[0].fill.coin, "BTC");
        assert_eq!(fills[1].twap_id, 1002);
        assert_eq!(fills[1].fill.coin, "ETH");

        mock.assert_async().await;
    }

    // ==================== Error Handling Tests ====================

    #[tokio::test]
    async fn test_user_fills_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .user_fills("0x1234567890123456789012345678901234567890", None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("500"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_fills_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid user address"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result: Result<Vec<UserFill>> = client.user_fills("invalid-address", None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid user address"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_fills_by_time_rate_limited() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .user_fills_by_time(
                "0x1234567890123456789012345678901234567890",
                1700000000000,
                None,
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("429"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_twap_slice_fills_unauthorized() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .user_twap_slice_fills("0x1234567890123456789012345678901234567890")
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("401"));

        mock.assert_async().await;
    }

    // ==================== Request Serialization Tests ====================

    #[tokio::test]
    async fn test_user_fills_request_serialization_without_dex() {
        let request = UserFillsRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"userFills","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_user_fills_request_serialization_with_dex() {
        let request =
            UserFillsRequest::new("0x1234567890123456789012345678901234567890").with_dex("testdex");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"userFills\""));
        assert!(json.contains("\"user\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"dex\":\"testdex\""));
    }

    #[tokio::test]
    async fn test_user_fills_request_serialization_with_aggregate() {
        let request = UserFillsRequest::new("0x1234567890123456789012345678901234567890")
            .with_aggregate_by_time(true);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"userFills\""));
        assert!(json.contains("\"aggregateByTime\":true"));
    }

    #[tokio::test]
    async fn test_user_fills_by_time_request_serialization_without_end() {
        let request = UserFillsByTimeRequest::new(
            "0x1234567890123456789012345678901234567890",
            1700000000000,
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"userFillsByTime\""));
        assert!(json.contains("\"user\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"startTime\":1700000000000"));
        assert!(!json.contains("endTime"));
    }

    #[tokio::test]
    async fn test_user_fills_by_time_request_serialization_with_end() {
        let request = UserFillsByTimeRequest::new(
            "0x1234567890123456789012345678901234567890",
            1700000000000,
        )
        .with_end_time(1700100000000);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"userFillsByTime\""));
        assert!(json.contains("\"startTime\":1700000000000"));
        assert!(json.contains("\"endTime\":1700100000000"));
    }

    #[tokio::test]
    async fn test_user_twap_slice_fills_request_serialization() {
        let request = UserTwapSliceFillsRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"userTwapSliceFills","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }
}
