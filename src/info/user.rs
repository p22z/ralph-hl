//! User clearinghouse state info endpoints
//!
//! This module provides methods for querying user account state from the Hyperliquid API.

use crate::client::Client;
use crate::error::Result;
use crate::types::{
    ClearinghouseState, ClearinghouseStateRequest, SpotClearinghouseState,
    SpotClearinghouseStateRequest,
};

impl Client {
    /// Retrieve perpetuals clearinghouse state for a user
    ///
    /// Returns the user's perpetual account state including positions, margin summary,
    /// and withdrawable balance.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    /// * `dex` - Optional DEX identifier. If None, uses the default perp DEX.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let state = client.clearinghouse_state("0x1234...", None).await?;
    /// println!("Account value: {}", state.margin_summary.account_value);
    /// println!("Withdrawable: {}", state.withdrawable);
    /// for pos in state.asset_positions {
    ///     println!("{}: {} @ {}", pos.position.coin, pos.position.szi, pos.position.entry_px);
    /// }
    /// ```
    pub async fn clearinghouse_state(
        &self,
        user: &str,
        dex: Option<&str>,
    ) -> Result<ClearinghouseState> {
        let mut request = ClearinghouseStateRequest::new(user);
        if let Some(d) = dex {
            request = request.with_dex(d);
        }
        self.post_info(&request).await
    }

    /// Retrieve spot clearinghouse state for a user
    ///
    /// Returns the user's spot account state including token balances.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let state = client.spot_clearinghouse_state("0x1234...").await?;
    /// for balance in state.balances {
    ///     println!("{}: {} (hold: {})", balance.coin, balance.total, balance.hold);
    /// }
    /// ```
    pub async fn spot_clearinghouse_state(&self, user: &str) -> Result<SpotClearinghouseState> {
        let request = SpotClearinghouseStateRequest::new(user);
        self.post_info(&request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::types::ClearinghouseStateRequest;
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

        async fn clearinghouse_state(
            &self,
            user: &str,
            dex: Option<&str>,
        ) -> Result<ClearinghouseState> {
            let mut request = ClearinghouseStateRequest::new(user);
            if let Some(d) = dex {
                request = request.with_dex(d);
            }
            self.post_info(&request).await
        }

        async fn spot_clearinghouse_state(&self, user: &str) -> Result<SpotClearinghouseState> {
            let request = SpotClearinghouseStateRequest::new(user);
            self.post_info(&request).await
        }
    }

    #[tokio::test]
    async fn test_clearinghouse_state_without_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "clearinghouseState",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "assetPositions": [
                        {
                            "position": {
                                "coin": "BTC",
                                "cumFunding": {
                                    "allTime": "100.0",
                                    "sinceChange": "10.0",
                                    "sinceOpen": "5.0"
                                },
                                "entryPx": "50000.0",
                                "leverage": {
                                    "type": "cross",
                                    "value": 10
                                },
                                "liquidationPx": "45000.0",
                                "marginUsed": "500.0",
                                "maxLeverage": 50,
                                "positionValue": "5000.0",
                                "returnOnEquity": "0.1",
                                "szi": "0.1",
                                "unrealizedPnl": "100.0"
                            },
                            "type": "oneWay"
                        }
                    ],
                    "crossMaintenanceMarginUsed": "250.0",
                    "crossMarginSummary": {
                        "accountValue": "10000.0",
                        "totalMarginUsed": "500.0",
                        "totalNtlPos": "5000.0",
                        "totalRawUsd": "10000.0"
                    },
                    "marginSummary": {
                        "accountValue": "10000.0",
                        "totalMarginUsed": "500.0",
                        "totalNtlPos": "5000.0",
                        "totalRawUsd": "10000.0"
                    },
                    "time": 1700000000000,
                    "withdrawable": "9500.0"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let state = client
            .clearinghouse_state("0x1234567890123456789012345678901234567890", None)
            .await
            .unwrap();

        assert_eq!(state.asset_positions.len(), 1);
        assert_eq!(state.asset_positions[0].position.coin, "BTC");
        assert_eq!(state.asset_positions[0].position.entry_px, "50000.0");
        assert_eq!(state.asset_positions[0].position.szi, "0.1");
        assert_eq!(state.margin_summary.account_value, "10000.0");
        assert_eq!(state.withdrawable, "9500.0");
        assert_eq!(state.time, 1700000000000);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_clearinghouse_state_with_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "clearinghouseState",
                "user": "0x1234567890123456789012345678901234567890",
                "dex": "customdex"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "assetPositions": [],
                    "crossMaintenanceMarginUsed": "0.0",
                    "crossMarginSummary": {
                        "accountValue": "5000.0",
                        "totalMarginUsed": "0.0",
                        "totalNtlPos": "0.0",
                        "totalRawUsd": "5000.0"
                    },
                    "marginSummary": {
                        "accountValue": "5000.0",
                        "totalMarginUsed": "0.0",
                        "totalNtlPos": "0.0",
                        "totalRawUsd": "5000.0"
                    },
                    "time": 1700000000000,
                    "withdrawable": "5000.0"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let state = client
            .clearinghouse_state(
                "0x1234567890123456789012345678901234567890",
                Some("customdex"),
            )
            .await
            .unwrap();

        assert_eq!(state.asset_positions.len(), 0);
        assert_eq!(state.margin_summary.account_value, "5000.0");
        assert_eq!(state.withdrawable, "5000.0");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_clearinghouse_state_with_multiple_positions() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "clearinghouseState",
                "user": "0xabcdef1234567890123456789012345678901234"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "assetPositions": [
                        {
                            "position": {
                                "coin": "BTC",
                                "cumFunding": {
                                    "allTime": "100.0",
                                    "sinceChange": "10.0",
                                    "sinceOpen": "5.0"
                                },
                                "entryPx": "50000.0",
                                "leverage": {
                                    "type": "cross",
                                    "value": 10
                                },
                                "liquidationPx": "45000.0",
                                "marginUsed": "500.0",
                                "maxLeverage": 50,
                                "positionValue": "5000.0",
                                "returnOnEquity": "0.1",
                                "szi": "0.1",
                                "unrealizedPnl": "100.0"
                            },
                            "type": "oneWay"
                        },
                        {
                            "position": {
                                "coin": "ETH",
                                "cumFunding": {
                                    "allTime": "50.0",
                                    "sinceChange": "5.0",
                                    "sinceOpen": "2.0"
                                },
                                "entryPx": "3000.0",
                                "leverage": {
                                    "type": "isolated",
                                    "value": 5,
                                    "rawUsd": "600.0"
                                },
                                "liquidationPx": "2500.0",
                                "marginUsed": "300.0",
                                "maxLeverage": 50,
                                "positionValue": "1500.0",
                                "returnOnEquity": "0.05",
                                "szi": "-0.5",
                                "unrealizedPnl": "-25.0"
                            },
                            "type": "oneWay"
                        }
                    ],
                    "crossMaintenanceMarginUsed": "400.0",
                    "crossMarginSummary": {
                        "accountValue": "15000.0",
                        "totalMarginUsed": "800.0",
                        "totalNtlPos": "6500.0",
                        "totalRawUsd": "15000.0"
                    },
                    "marginSummary": {
                        "accountValue": "15000.0",
                        "totalMarginUsed": "800.0",
                        "totalNtlPos": "6500.0",
                        "totalRawUsd": "15000.0"
                    },
                    "time": 1700000000000,
                    "withdrawable": "14200.0"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let state = client
            .clearinghouse_state("0xabcdef1234567890123456789012345678901234", None)
            .await
            .unwrap();

        assert_eq!(state.asset_positions.len(), 2);

        // Check BTC position (long)
        assert_eq!(state.asset_positions[0].position.coin, "BTC");
        assert_eq!(state.asset_positions[0].position.szi, "0.1");
        assert_eq!(state.asset_positions[0].position.leverage.leverage_type, "cross");

        // Check ETH position (short)
        assert_eq!(state.asset_positions[1].position.coin, "ETH");
        assert_eq!(state.asset_positions[1].position.szi, "-0.5");
        assert_eq!(state.asset_positions[1].position.leverage.leverage_type, "isolated");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_clearinghouse_state() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "spotClearinghouseState",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "balances": [
                        {
                            "coin": "USDC",
                            "token": 0,
                            "hold": "100.0",
                            "total": "1000.0",
                            "entryNtl": "1000.0"
                        },
                        {
                            "coin": "HYPE",
                            "token": 1,
                            "hold": "0.0",
                            "total": "500.0",
                            "entryNtl": "250.0"
                        }
                    ]
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let state = client
            .spot_clearinghouse_state("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(state.balances.len(), 2);
        assert_eq!(state.balances[0].coin, "USDC");
        assert_eq!(state.balances[0].total, "1000.0");
        assert_eq!(state.balances[0].hold, "100.0");
        assert_eq!(state.balances[1].coin, "HYPE");
        assert_eq!(state.balances[1].total, "500.0");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_clearinghouse_state_empty_balances() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "spotClearinghouseState",
                "user": "0xdeadbeef12345678901234567890123456789012"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"balances": []}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let state = client
            .spot_clearinghouse_state("0xdeadbeef12345678901234567890123456789012")
            .await
            .unwrap();

        assert_eq!(state.balances.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_clearinghouse_state_error_handling() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .clearinghouse_state("0x1234567890123456789012345678901234567890", None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("500"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_clearinghouse_state_error_handling() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .spot_clearinghouse_state("0x1234567890123456789012345678901234567890")
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("401"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_clearinghouse_state_api_error_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid user address"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result: Result<ClearinghouseState> = client
            .clearinghouse_state("invalid-address", None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid user address"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_clearinghouse_state_request_serialization_without_dex() {
        let request = ClearinghouseStateRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"clearinghouseState","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_clearinghouse_state_request_serialization_with_dex() {
        let request = ClearinghouseStateRequest::new("0x1234567890123456789012345678901234567890")
            .with_dex("testdex");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"clearinghouseState\""));
        assert!(json.contains("\"user\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"dex\":\"testdex\""));
    }

    #[tokio::test]
    async fn test_spot_clearinghouse_state_request_serialization() {
        let request = SpotClearinghouseStateRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"spotClearinghouseState","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }
}
