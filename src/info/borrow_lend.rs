//! Borrow/Lend info endpoints
//!
//! This module provides methods for querying borrow/lend protocol information from the
//! Hyperliquid API, including user positions, reserve states, and quote token status.

use crate::client::Client;
use crate::error::Result;
use crate::types::{
    AlignedQuoteToken, AlignedQuoteTokenRequest, AllBorrowLendReserveStatesRequest,
    BorrowLendReserveState, BorrowLendReserveStateRequest, BorrowLendUserState,
    BorrowLendUserStateRequest,
};

impl Client {
    /// Retrieve user's borrow/lend state
    ///
    /// Returns the user's positions in the borrow/lend protocol including
    /// supplied and borrowed amounts for each asset, along with APY information.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let state = client.borrow_lend_user_state("0x1234...").await?;
    /// for position in state.positions {
    ///     println!("{}: supplied={}, borrowed={}", position.coin, position.supplied, position.borrowed);
    /// }
    /// ```
    pub async fn borrow_lend_user_state(&self, user: &str) -> Result<BorrowLendUserState> {
        let request = BorrowLendUserStateRequest::new(user);
        self.post_info(&request).await
    }

    /// Retrieve reserve state for a specific coin
    ///
    /// Returns the current state of the borrow/lend reserve for a specific asset,
    /// including total supplied/borrowed amounts, APYs, and utilization rate.
    ///
    /// # Arguments
    /// * `coin` - The asset symbol (e.g., "USDC", "ETH")
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let reserve = client.borrow_lend_reserve_state("USDC").await?;
    /// println!("USDC Reserve: supply APY={}, borrow APY={}, utilization={}",
    ///     reserve.supply_apy, reserve.borrow_apy, reserve.utilization_rate);
    /// ```
    pub async fn borrow_lend_reserve_state(&self, coin: &str) -> Result<BorrowLendReserveState> {
        let request = BorrowLendReserveStateRequest::new(coin);
        self.post_info(&request).await
    }

    /// Retrieve all borrow/lend reserve states
    ///
    /// Returns the current state of all borrow/lend reserves in the protocol,
    /// including supply/borrow amounts, APYs, and utilization rates for each asset.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let reserves = client.all_borrow_lend_reserve_states().await?;
    /// for reserve in reserves {
    ///     println!("{}: utilization={}", reserve.coin, reserve.utilization_rate);
    /// }
    /// ```
    pub async fn all_borrow_lend_reserve_states(&self) -> Result<Vec<BorrowLendReserveState>> {
        let request = AllBorrowLendReserveStatesRequest::default();
        self.post_info(&request).await
    }

    /// Check if a coin is an aligned quote token
    ///
    /// Returns whether the specified coin is configured as an aligned quote token
    /// in the protocol, which determines its usage in trading pairs.
    ///
    /// # Arguments
    /// * `coin` - The asset symbol to check (e.g., "USDC")
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let status = client.aligned_quote_token("USDC").await?;
    /// if status.is_aligned {
    ///     println!("USDC is an aligned quote token with index {:?}", status.quote_token_index);
    /// }
    /// ```
    pub async fn aligned_quote_token(&self, coin: &str) -> Result<AlignedQuoteToken> {
        let request = AlignedQuoteTokenRequest::new(coin);
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

        async fn borrow_lend_user_state(&self, user: &str) -> Result<BorrowLendUserState> {
            let request = BorrowLendUserStateRequest::new(user);
            self.post_info(&request).await
        }

        async fn borrow_lend_reserve_state(&self, coin: &str) -> Result<BorrowLendReserveState> {
            let request = BorrowLendReserveStateRequest::new(coin);
            self.post_info(&request).await
        }

        async fn all_borrow_lend_reserve_states(&self) -> Result<Vec<BorrowLendReserveState>> {
            let request = AllBorrowLendReserveStatesRequest::default();
            self.post_info(&request).await
        }

        async fn aligned_quote_token(&self, coin: &str) -> Result<AlignedQuoteToken> {
            let request = AlignedQuoteTokenRequest::new(coin);
            self.post_info(&request).await
        }
    }

    // ========================================================================
    // Borrow/Lend User State Tests
    // ========================================================================

    #[tokio::test]
    async fn test_borrow_lend_user_state() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "borrowLendUserState",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "positions": [
                        {
                            "coin": "USDC",
                            "supplied": "10000.00",
                            "borrowed": "0.00",
                            "supplyApy": "0.05",
                            "borrowApy": "0.08",
                            "accruedInterest": "50.00"
                        },
                        {
                            "coin": "ETH",
                            "supplied": "0.00",
                            "borrowed": "2.5",
                            "supplyApy": "0.02",
                            "borrowApy": "0.05",
                            "accruedInterest": "-0.125"
                        }
                    ],
                    "totalSuppliedValue": "10000.00",
                    "totalBorrowedValue": "5000.00",
                    "healthFactor": "1.8"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let state = client
            .borrow_lend_user_state("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(state.positions.len(), 2);
        assert_eq!(state.positions[0].coin, "USDC");
        assert_eq!(state.positions[0].supplied, "10000.00");
        assert_eq!(state.positions[0].borrowed, "0.00");
        assert_eq!(state.positions[0].supply_apy, Some("0.05".to_string()));
        assert_eq!(state.positions[0].borrow_apy, Some("0.08".to_string()));
        assert_eq!(
            state.positions[0].accrued_interest,
            Some("50.00".to_string())
        );

        assert_eq!(state.positions[1].coin, "ETH");
        assert_eq!(state.positions[1].supplied, "0.00");
        assert_eq!(state.positions[1].borrowed, "2.5");

        assert_eq!(state.total_supplied_value, Some("10000.00".to_string()));
        assert_eq!(state.total_borrowed_value, Some("5000.00".to_string()));
        assert_eq!(state.health_factor, Some("1.8".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_borrow_lend_user_state_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "borrowLendUserState",
                "user": "0xdeadbeef12345678901234567890123456789012"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "positions": []
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let state = client
            .borrow_lend_user_state("0xdeadbeef12345678901234567890123456789012")
            .await
            .unwrap();

        assert_eq!(state.positions.len(), 0);
        assert!(state.total_supplied_value.is_none());
        assert!(state.total_borrowed_value.is_none());
        assert!(state.health_factor.is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_borrow_lend_user_state_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "borrowLendUserState",
                "user": "invalid_address"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid user address"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.borrow_lend_user_state("invalid_address").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert_eq!(msg, "Invalid user address"),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_borrow_lend_user_state_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .borrow_lend_user_state("0x1234567890123456789012345678901234567890")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert!(msg.contains("500")),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    // ========================================================================
    // Borrow/Lend Reserve State Tests
    // ========================================================================

    #[tokio::test]
    async fn test_borrow_lend_reserve_state() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "borrowLendReserveState",
                "coin": "USDC"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "coin": "USDC",
                    "totalSupplied": "100000000.00",
                    "totalBorrowed": "75000000.00",
                    "supplyApy": "0.045",
                    "borrowApy": "0.06",
                    "utilizationRate": "0.75",
                    "availableLiquidity": "25000000.00",
                    "ltv": "0.8",
                    "liquidationThreshold": "0.85"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let reserve = client.borrow_lend_reserve_state("USDC").await.unwrap();

        assert_eq!(reserve.coin, "USDC");
        assert_eq!(reserve.total_supplied, "100000000.00");
        assert_eq!(reserve.total_borrowed, "75000000.00");
        assert_eq!(reserve.supply_apy, "0.045");
        assert_eq!(reserve.borrow_apy, "0.06");
        assert_eq!(reserve.utilization_rate, "0.75");
        assert_eq!(reserve.available_liquidity, Some("25000000.00".to_string()));
        assert_eq!(reserve.ltv, Some("0.8".to_string()));
        assert_eq!(reserve.liquidation_threshold, Some("0.85".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_borrow_lend_reserve_state_minimal() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "borrowLendReserveState",
                "coin": "ETH"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "coin": "ETH",
                    "totalSupplied": "50000.00",
                    "totalBorrowed": "10000.00",
                    "supplyApy": "0.02",
                    "borrowApy": "0.04",
                    "utilizationRate": "0.2"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let reserve = client.borrow_lend_reserve_state("ETH").await.unwrap();

        assert_eq!(reserve.coin, "ETH");
        assert_eq!(reserve.total_supplied, "50000.00");
        assert_eq!(reserve.total_borrowed, "10000.00");
        assert_eq!(reserve.utilization_rate, "0.2");
        assert!(reserve.available_liquidity.is_none());
        assert!(reserve.ltv.is_none());
        assert!(reserve.liquidation_threshold.is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_borrow_lend_reserve_state_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "borrowLendReserveState",
                "coin": "INVALID"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Unknown coin"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.borrow_lend_reserve_state("INVALID").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert_eq!(msg, "Unknown coin"),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_borrow_lend_reserve_state_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(503)
            .with_body("Service Unavailable")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.borrow_lend_reserve_state("USDC").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert!(msg.contains("503")),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    // ========================================================================
    // All Borrow/Lend Reserve States Tests
    // ========================================================================

    #[tokio::test]
    async fn test_all_borrow_lend_reserve_states() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "allBorrowLendReserveStates"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "coin": "USDC",
                        "totalSupplied": "100000000.00",
                        "totalBorrowed": "75000000.00",
                        "supplyApy": "0.045",
                        "borrowApy": "0.06",
                        "utilizationRate": "0.75"
                    },
                    {
                        "coin": "ETH",
                        "totalSupplied": "50000.00",
                        "totalBorrowed": "10000.00",
                        "supplyApy": "0.02",
                        "borrowApy": "0.04",
                        "utilizationRate": "0.2"
                    },
                    {
                        "coin": "BTC",
                        "totalSupplied": "1000.00",
                        "totalBorrowed": "500.00",
                        "supplyApy": "0.015",
                        "borrowApy": "0.03",
                        "utilizationRate": "0.5",
                        "availableLiquidity": "500.00"
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let reserves = client.all_borrow_lend_reserve_states().await.unwrap();

        assert_eq!(reserves.len(), 3);

        assert_eq!(reserves[0].coin, "USDC");
        assert_eq!(reserves[0].total_supplied, "100000000.00");
        assert_eq!(reserves[0].utilization_rate, "0.75");

        assert_eq!(reserves[1].coin, "ETH");
        assert_eq!(reserves[1].total_supplied, "50000.00");
        assert_eq!(reserves[1].utilization_rate, "0.2");

        assert_eq!(reserves[2].coin, "BTC");
        assert_eq!(reserves[2].total_supplied, "1000.00");
        assert_eq!(reserves[2].available_liquidity, Some("500.00".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_all_borrow_lend_reserve_states_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "allBorrowLendReserveStates"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let reserves = client.all_borrow_lend_reserve_states().await.unwrap();

        assert_eq!(reserves.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_all_borrow_lend_reserve_states_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "allBorrowLendReserveStates"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Service temporarily unavailable"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.all_borrow_lend_reserve_states().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert_eq!(msg, "Service temporarily unavailable"),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_all_borrow_lend_reserve_states_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(429)
            .with_body("Rate Limited")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.all_borrow_lend_reserve_states().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert!(msg.contains("429")),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    // ========================================================================
    // Aligned Quote Token Tests
    // ========================================================================

    #[tokio::test]
    async fn test_aligned_quote_token_aligned() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "alignedQuoteToken",
                "coin": "USDC"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "coin": "USDC",
                    "isAligned": true,
                    "quoteTokenIndex": 0
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let status = client.aligned_quote_token("USDC").await.unwrap();

        assert_eq!(status.coin, "USDC");
        assert!(status.is_aligned);
        assert_eq!(status.quote_token_index, Some(0));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_aligned_quote_token_not_aligned() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "alignedQuoteToken",
                "coin": "ETH"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "coin": "ETH",
                    "isAligned": false
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let status = client.aligned_quote_token("ETH").await.unwrap();

        assert_eq!(status.coin, "ETH");
        assert!(!status.is_aligned);
        assert!(status.quote_token_index.is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_aligned_quote_token_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "alignedQuoteToken",
                "coin": "INVALID"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Unknown coin"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.aligned_quote_token("INVALID").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert_eq!(msg, "Unknown coin"),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_aligned_quote_token_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.aligned_quote_token("USDC").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert!(msg.contains("500")),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    // ========================================================================
    // Request Serialization Tests
    // ========================================================================

    #[test]
    fn test_borrow_lend_user_state_request_serialization() {
        let request = BorrowLendUserStateRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"borrowLendUserState","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[test]
    fn test_borrow_lend_reserve_state_request_serialization() {
        let request = BorrowLendReserveStateRequest::new("USDC");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"borrowLendReserveState","coin":"USDC"}"#);
    }

    #[test]
    fn test_all_borrow_lend_reserve_states_request_serialization() {
        let request = AllBorrowLendReserveStatesRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"allBorrowLendReserveStates"}"#);
    }

    #[test]
    fn test_aligned_quote_token_request_serialization() {
        let request = AlignedQuoteTokenRequest::new("USDC");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"alignedQuoteToken","coin":"USDC"}"#);
    }

    // ========================================================================
    // Response Type Deserialization Tests
    // ========================================================================

    #[test]
    fn test_borrow_lend_user_state_deserialization() {
        let json = r#"{
            "positions": [
                {
                    "coin": "USDC",
                    "supplied": "1000.00",
                    "borrowed": "0.00"
                }
            ]
        }"#;
        let state: BorrowLendUserState = serde_json::from_str(json).unwrap();
        assert_eq!(state.positions.len(), 1);
        assert_eq!(state.positions[0].coin, "USDC");
        assert_eq!(state.positions[0].supplied, "1000.00");
        assert!(state.positions[0].supply_apy.is_none());
    }

    #[test]
    fn test_borrow_lend_reserve_state_deserialization() {
        let json = r#"{
            "coin": "USDC",
            "totalSupplied": "1000000.00",
            "totalBorrowed": "500000.00",
            "supplyApy": "0.05",
            "borrowApy": "0.08",
            "utilizationRate": "0.5"
        }"#;
        let reserve: BorrowLendReserveState = serde_json::from_str(json).unwrap();
        assert_eq!(reserve.coin, "USDC");
        assert_eq!(reserve.total_supplied, "1000000.00");
        assert_eq!(reserve.total_borrowed, "500000.00");
        assert_eq!(reserve.supply_apy, "0.05");
        assert_eq!(reserve.borrow_apy, "0.08");
        assert_eq!(reserve.utilization_rate, "0.5");
        assert!(reserve.available_liquidity.is_none());
    }

    #[test]
    fn test_aligned_quote_token_deserialization() {
        let json = r#"{
            "coin": "USDC",
            "isAligned": true,
            "quoteTokenIndex": 0
        }"#;
        let token: AlignedQuoteToken = serde_json::from_str(json).unwrap();
        assert_eq!(token.coin, "USDC");
        assert!(token.is_aligned);
        assert_eq!(token.quote_token_index, Some(0));
    }
}
