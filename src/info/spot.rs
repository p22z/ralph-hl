//! Spot metadata info endpoints
//!
//! This module provides methods for querying spot metadata from the Hyperliquid API.

use crate::client::Client;
use crate::error::Result;
use crate::types::{
    SpotAssetCtx, SpotDeployState, SpotDeployStateRequest, SpotMetaAndAssetCtxsRequest,
    SpotMetaRequest, SpotMetaResponse, SpotPairAuctionStatus, SpotPairDeployAuctionStatusRequest,
    TokenDetails, TokenDetailsRequest,
};

impl Client {
    /// Retrieve spot metadata
    ///
    /// Returns metadata for all spot tokens and trading pairs.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let meta = client.spot_meta().await?;
    /// for token in meta.tokens {
    ///     println!("{}: {} decimals", token.name, token.sz_decimals);
    /// }
    /// for pair in meta.universe {
    ///     println!("Pair: {}", pair.name);
    /// }
    /// ```
    pub async fn spot_meta(&self) -> Result<SpotMetaResponse> {
        let request = SpotMetaRequest::default();
        self.post_info(&request).await
    }

    /// Retrieve spot metadata with asset contexts
    ///
    /// Returns both the spot metadata and current asset contexts including
    /// mark price, daily volume, and circulating supply.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let (meta, ctxs) = client.spot_meta_and_asset_ctxs().await?;
    /// for (pair, ctx) in meta.universe.iter().zip(ctxs.iter()) {
    ///     println!("{}: mark={}, vol={}", pair.name, ctx.mark_px, ctx.day_ntl_vlm);
    /// }
    /// ```
    pub async fn spot_meta_and_asset_ctxs(&self) -> Result<(SpotMetaResponse, Vec<SpotAssetCtx>)> {
        let request = SpotMetaAndAssetCtxsRequest::default();
        self.post_info(&request).await
    }

    /// Retrieve token details
    ///
    /// Returns detailed information about a specific token including deployment info,
    /// supply details, and genesis allocations.
    ///
    /// # Arguments
    /// * `token_id` - The token identifier (e.g., "0x1" for USDC)
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let details = client.token_details("0x1").await?;
    /// println!("Token: {} ({})", details.name, details.full_name.unwrap_or_default());
    /// if let Some(supply) = details.total_supply {
    ///     println!("Total supply: {}", supply);
    /// }
    /// ```
    pub async fn token_details(&self, token_id: &str) -> Result<TokenDetails> {
        let request = TokenDetailsRequest::new(token_id);
        self.post_info(&request).await
    }

    /// Retrieve spot deploy state for a user
    ///
    /// Returns the deployment auction state for spot tokens for the specified user.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let state = client.spot_deploy_state("0x1234...").await?;
    /// for token_state in state.tokens {
    ///     println!("Token: {}", token_state.token.name);
    /// }
    /// ```
    pub async fn spot_deploy_state(&self, user: &str) -> Result<SpotDeployState> {
        let request = SpotDeployStateRequest::new(user);
        self.post_info(&request).await
    }

    /// Retrieve spot pair deploy auction status
    ///
    /// Returns the current status of the spot pair deployment auction.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let status = client.spot_pair_deploy_auction_status().await?;
    /// println!("Auction started: {}", status.start_time_seconds);
    /// println!("Current gas: {}", status.current_gas);
    /// ```
    pub async fn spot_pair_deploy_auction_status(&self) -> Result<SpotPairAuctionStatus> {
        let request = SpotPairDeployAuctionStatusRequest::default();
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

        async fn spot_meta(&self) -> Result<SpotMetaResponse> {
            let request = SpotMetaRequest::default();
            self.post_info(&request).await
        }

        async fn spot_meta_and_asset_ctxs(&self) -> Result<(SpotMetaResponse, Vec<SpotAssetCtx>)> {
            let request = SpotMetaAndAssetCtxsRequest::default();
            self.post_info(&request).await
        }

        async fn token_details(&self, token_id: &str) -> Result<TokenDetails> {
            let request = TokenDetailsRequest::new(token_id);
            self.post_info(&request).await
        }

        async fn spot_deploy_state(&self, user: &str) -> Result<SpotDeployState> {
            let request = SpotDeployStateRequest::new(user);
            self.post_info(&request).await
        }

        async fn spot_pair_deploy_auction_status(&self) -> Result<SpotPairAuctionStatus> {
            let request = SpotPairDeployAuctionStatusRequest::default();
            self.post_info(&request).await
        }
    }

    #[tokio::test]
    async fn test_spot_meta() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({"type": "spotMeta"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "tokens": [
                        {
                            "name": "USDC",
                            "szDecimals": 2,
                            "weiDecimals": 6,
                            "index": 0,
                            "tokenId": "0x1",
                            "isCanonical": true,
                            "evmContract": null,
                            "fullName": "USD Coin"
                        },
                        {
                            "name": "PURR",
                            "szDecimals": 0,
                            "weiDecimals": 18,
                            "index": 1,
                            "tokenId": "0x2",
                            "isCanonical": true,
                            "evmContract": null,
                            "fullName": "Purr Token"
                        }
                    ],
                    "universe": [
                        {
                            "name": "PURR/USDC",
                            "tokens": [1, 0],
                            "index": 0,
                            "isCanonical": true
                        }
                    ]
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let meta = client.spot_meta().await.unwrap();

        assert_eq!(meta.tokens.len(), 2);
        assert_eq!(meta.tokens[0].name, "USDC");
        assert_eq!(meta.tokens[0].token_id, "0x1");
        assert_eq!(meta.tokens[0].sz_decimals, 2);
        assert_eq!(meta.tokens[0].wei_decimals, 6);
        assert!(meta.tokens[0].is_canonical);
        assert_eq!(meta.tokens[1].name, "PURR");

        assert_eq!(meta.universe.len(), 1);
        assert_eq!(meta.universe[0].name, "PURR/USDC");
        assert_eq!(meta.universe[0].tokens, vec![1, 0]);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_meta_and_asset_ctxs() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "spotMetaAndAssetCtxs"}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "tokens": [
                            {
                                "name": "USDC",
                                "szDecimals": 2,
                                "weiDecimals": 6,
                                "index": 0,
                                "tokenId": "0x1",
                                "isCanonical": true,
                                "evmContract": null,
                                "fullName": "USD Coin"
                            }
                        ],
                        "universe": [
                            {
                                "name": "PURR/USDC",
                                "tokens": [1, 0],
                                "index": 0,
                                "isCanonical": true
                            }
                        ]
                    },
                    [
                        {
                            "dayNtlVlm": "1000000.0",
                            "markPx": "0.0025",
                            "midPx": "0.0025",
                            "prevDayPx": "0.0024",
                            "circulatingSupply": "1000000000.0"
                        }
                    ]
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let (meta, ctxs) = client.spot_meta_and_asset_ctxs().await.unwrap();

        assert_eq!(meta.tokens.len(), 1);
        assert_eq!(meta.tokens[0].name, "USDC");
        assert_eq!(meta.universe.len(), 1);
        assert_eq!(meta.universe[0].name, "PURR/USDC");

        assert_eq!(ctxs.len(), 1);
        assert_eq!(ctxs[0].day_ntl_vlm, "1000000.0");
        assert_eq!(ctxs[0].mark_px, "0.0025");
        assert_eq!(ctxs[0].mid_px, Some("0.0025".to_string()));
        assert_eq!(ctxs[0].prev_day_px, "0.0024");
        assert_eq!(ctxs[0].circulating_supply, "1000000000.0");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_token_details() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "tokenDetails", "tokenId": "0x2"}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "name": "PURR",
                    "szDecimals": 0,
                    "weiDecimals": 18,
                    "tokenId": "0x2",
                    "isCanonical": true,
                    "evmContract": null,
                    "fullName": "Purr Token",
                    "deployer": "0x1234567890123456789012345678901234567890",
                    "deployGas": "0.001",
                    "deployTime": 1700000000000,
                    "seededUsdc": "10000.0",
                    "totalSupply": "1000000000.0",
                    "maxSupply": "1000000000.0"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let details = client.token_details("0x2").await.unwrap();

        assert_eq!(details.name, "PURR");
        assert_eq!(details.sz_decimals, 0);
        assert_eq!(details.wei_decimals, 18);
        assert_eq!(details.token_id, "0x2");
        assert!(details.is_canonical);
        assert_eq!(details.full_name, Some("Purr Token".to_string()));
        assert_eq!(
            details.deployer,
            Some("0x1234567890123456789012345678901234567890".to_string())
        );
        assert_eq!(details.deploy_gas, Some("0.001".to_string()));
        assert_eq!(details.deploy_time, Some(1700000000000));
        assert_eq!(details.seeded_usdc, Some("10000.0".to_string()));
        assert_eq!(details.total_supply, Some("1000000000.0".to_string()));
        assert_eq!(details.max_supply, Some("1000000000.0".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_token_details_minimal() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "tokenDetails", "tokenId": "0x1"}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "name": "USDC",
                    "szDecimals": 2,
                    "weiDecimals": 6,
                    "tokenId": "0x1",
                    "isCanonical": true
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let details = client.token_details("0x1").await.unwrap();

        assert_eq!(details.name, "USDC");
        assert_eq!(details.sz_decimals, 2);
        assert_eq!(details.wei_decimals, 6);
        assert!(details.deployer.is_none());
        assert!(details.total_supply.is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_deploy_state() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "spotDeployState",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "tokens": [
                        {
                            "token": {
                                "name": "TEST",
                                "szDecimals": 2,
                                "weiDecimals": 18,
                                "index": 100,
                                "tokenId": "0x100",
                                "isCanonical": false,
                                "evmContract": null,
                                "fullName": "Test Token"
                            },
                            "spec": {
                                "name": "TEST",
                                "szDecimals": 2,
                                "weiDecimals": 18,
                                "fullName": "Test Token"
                            },
                            "existingTokenAndShouldRegister": null
                        }
                    ]
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let state = client
            .spot_deploy_state("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(state.tokens.len(), 1);
        assert_eq!(state.tokens[0].token.name, "TEST");
        assert_eq!(state.tokens[0].token.sz_decimals, 2);
        assert_eq!(state.tokens[0].spec.name, "TEST");
        assert!(state.tokens[0].existing_token_and_should_register.is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_deploy_state_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "spotDeployState",
                "user": "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"tokens": []}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let state = client
            .spot_deploy_state("0xabcdefabcdefabcdefabcdefabcdefabcdefabcd")
            .await
            .unwrap();

        assert!(state.tokens.is_empty());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_pair_deploy_auction_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "spotPairDeployAuctionStatus"}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "startTimeSeconds": 1700000000,
                    "durationSeconds": 3600,
                    "startGas": "10.0",
                    "currentGas": "5.0",
                    "endGas": "0.1"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let status = client.spot_pair_deploy_auction_status().await.unwrap();

        assert_eq!(status.start_time_seconds, 1700000000);
        assert_eq!(status.duration_seconds, 3600);
        assert_eq!(status.start_gas, "10.0");
        assert_eq!(status.current_gas, "5.0");
        assert_eq!(status.end_gas, Some("0.1".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_pair_deploy_auction_status_no_end_gas() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "spotPairDeployAuctionStatus"}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "startTimeSeconds": 1700000000,
                    "durationSeconds": 7200,
                    "startGas": "20.0",
                    "currentGas": "15.0"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let status = client.spot_pair_deploy_auction_status().await.unwrap();

        assert_eq!(status.start_time_seconds, 1700000000);
        assert_eq!(status.duration_seconds, 7200);
        assert_eq!(status.start_gas, "20.0");
        assert_eq!(status.current_gas, "15.0");
        assert!(status.end_gas.is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_meta_error_handling() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.spot_meta().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("500"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_token_details_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid token ID"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.token_details("invalid").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid token ID"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_meta_request_serialization() {
        let request = SpotMetaRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"spotMeta"}"#);
    }

    #[tokio::test]
    async fn test_spot_meta_and_asset_ctxs_request_serialization() {
        let request = SpotMetaAndAssetCtxsRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"spotMetaAndAssetCtxs"}"#);
    }

    #[tokio::test]
    async fn test_token_details_request_serialization() {
        let request = TokenDetailsRequest::new("0x123");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"tokenDetails","tokenId":"0x123"}"#);
    }

    #[tokio::test]
    async fn test_spot_deploy_state_request_serialization() {
        let request = SpotDeployStateRequest::new("0xabc");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"spotDeployState","user":"0xabc"}"#);
    }

    #[tokio::test]
    async fn test_spot_pair_deploy_auction_status_request_serialization() {
        let request = SpotPairDeployAuctionStatusRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"spotPairDeployAuctionStatus"}"#);
    }
}
