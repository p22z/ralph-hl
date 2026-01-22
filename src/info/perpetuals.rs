//! Perpetuals metadata info endpoints
//!
//! This module provides methods for querying perpetuals metadata from the Hyperliquid API.

use crate::client::Client;
use crate::error::Result;
use crate::types::{
    AllPerpMetasRequest, AssetCtx, MetaAndAssetCtxsRequest, MetaRequest, PerpAssetMeta,
    PerpDexInfo, PerpDexsRequest, PerpMetaResponse,
};

impl Client {
    /// Retrieve all perpetual DEXs
    ///
    /// Returns a list of all available perpetual DEX configurations.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let dexs = client.perp_dexs().await?;
    /// for dex in dexs {
    ///     println!("DEX: {} ({})", dex.name, dex.full_name);
    /// }
    /// ```
    pub async fn perp_dexs(&self) -> Result<Vec<PerpDexInfo>> {
        let request = PerpDexsRequest::default();
        let response: Option<Vec<PerpDexInfo>> = self.post_info(&request).await?;
        Ok(response.unwrap_or_default())
    }

    /// Retrieve perpetuals metadata
    ///
    /// Returns metadata for all perpetual assets including size decimals,
    /// max leverage, and margin tables.
    ///
    /// # Arguments
    /// * `dex` - Optional DEX identifier. If None, uses the default perp DEX.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let meta = client.meta(None).await?;
    /// for asset in meta.universe {
    ///     println!("{}: max leverage {}", asset.name, asset.max_leverage);
    /// }
    /// ```
    pub async fn meta(&self, dex: Option<&str>) -> Result<PerpMetaResponse> {
        let request = MetaRequest {
            dex: dex.map(|s| s.to_string()),
            ..Default::default()
        };
        self.post_info(&request).await
    }

    /// Retrieve perpetuals metadata with asset contexts
    ///
    /// Returns both the perpetuals metadata and current asset contexts including
    /// mark price, oracle price, funding rate, open interest, etc.
    ///
    /// # Arguments
    /// * `dex` - Optional DEX identifier. If None, uses the default perp DEX.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let (meta, ctxs) = client.meta_and_asset_ctxs(None).await?;
    /// for (asset, ctx) in meta.universe.iter().zip(ctxs.iter()) {
    ///     println!("{}: mark={}, funding={}", asset.name, ctx.mark_px, ctx.funding);
    /// }
    /// ```
    pub async fn meta_and_asset_ctxs(
        &self,
        dex: Option<&str>,
    ) -> Result<(PerpMetaResponse, Vec<AssetCtx>)> {
        let request = MetaAndAssetCtxsRequest {
            dex: dex.map(|s| s.to_string()),
            ..Default::default()
        };
        self.post_info(&request).await
    }

    /// Retrieve all perpetual metadata across all DEXs
    ///
    /// Returns metadata for all perpetual assets from all available DEXs.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let all_metas = client.all_perp_metas().await?;
    /// for asset in all_metas {
    ///     println!("{}: {} decimals", asset.name, asset.sz_decimals);
    /// }
    /// ```
    pub async fn all_perp_metas(&self) -> Result<Vec<PerpAssetMeta>> {
        let request = AllPerpMetasRequest::default();
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

        async fn perp_dexs(&self) -> Result<Vec<PerpDexInfo>> {
            let request = PerpDexsRequest::default();
            let response: Option<Vec<PerpDexInfo>> = self.post_info(&request).await?;
            Ok(response.unwrap_or_default())
        }

        async fn meta(&self, dex: Option<&str>) -> Result<PerpMetaResponse> {
            let mut request = MetaRequest::default();
            request.dex = dex.map(|s| s.to_string());
            self.post_info(&request).await
        }

        async fn meta_and_asset_ctxs(
            &self,
            dex: Option<&str>,
        ) -> Result<(PerpMetaResponse, Vec<AssetCtx>)> {
            let mut request = MetaAndAssetCtxsRequest::default();
            request.dex = dex.map(|s| s.to_string());
            self.post_info(&request).await
        }

        async fn all_perp_metas(&self) -> Result<Vec<PerpAssetMeta>> {
            let request = AllPerpMetasRequest::default();
            self.post_info(&request).await
        }
    }

    #[tokio::test]
    async fn test_perp_dexs() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({"type": "perpDexs"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "name": "HL",
                        "fullName": "Hyperliquid",
                        "deployer": "0x0000000000000000000000000000000000000000",
                        "oracleUpdater": null,
                        "feeRecipient": null,
                        "assetToStreamingOiCap": [],
                        "assetToFundingMultiplier": []
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let dexs = client.perp_dexs().await.unwrap();

        assert_eq!(dexs.len(), 1);
        assert_eq!(dexs[0].name, "HL");
        assert_eq!(dexs[0].full_name, "Hyperliquid");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_perp_dexs_null_returns_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({"type": "perpDexs"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("null")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let dexs = client.perp_dexs().await.unwrap();

        assert!(dexs.is_empty());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_meta_without_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({"type": "meta"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "universe": [
                        {
                            "name": "BTC",
                            "szDecimals": 4,
                            "maxLeverage": 50
                        },
                        {
                            "name": "ETH",
                            "szDecimals": 3,
                            "maxLeverage": 50
                        }
                    ],
                    "marginTables": []
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let meta = client.meta(None).await.unwrap();

        assert_eq!(meta.universe.len(), 2);
        assert_eq!(meta.universe[0].name, "BTC");
        assert_eq!(meta.universe[0].sz_decimals, 4);
        assert_eq!(meta.universe[0].max_leverage, 50);
        assert_eq!(meta.universe[1].name, "ETH");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_meta_with_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "meta", "dex": "testdex"}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "universe": [
                        {
                            "name": "SOL",
                            "szDecimals": 2,
                            "maxLeverage": 20
                        }
                    ],
                    "marginTables": []
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let meta = client.meta(Some("testdex")).await.unwrap();

        assert_eq!(meta.universe.len(), 1);
        assert_eq!(meta.universe[0].name, "SOL");
        assert_eq!(meta.universe[0].max_leverage, 20);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_meta_and_asset_ctxs() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "metaAndAssetCtxs"}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "universe": [
                            {
                                "name": "BTC",
                                "szDecimals": 4,
                                "maxLeverage": 50
                            }
                        ],
                        "marginTables": []
                    },
                    [
                        {
                            "dayNtlVlm": "1000000000.0",
                            "funding": "0.0001",
                            "impactPxs": ["50000.0", "50010.0"],
                            "markPx": "50005.0",
                            "midPx": "50005.0",
                            "openInterest": "100000.0",
                            "oraclePx": "50000.0",
                            "premium": "0.0001",
                            "prevDayPx": "49000.0"
                        }
                    ]
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let (meta, ctxs) = client.meta_and_asset_ctxs(None).await.unwrap();

        assert_eq!(meta.universe.len(), 1);
        assert_eq!(meta.universe[0].name, "BTC");
        assert_eq!(ctxs.len(), 1);
        assert_eq!(ctxs[0].mark_px, "50005.0");
        assert_eq!(ctxs[0].funding, "0.0001");
        assert_eq!(ctxs[0].open_interest, "100000.0");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_meta_and_asset_ctxs_with_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                serde_json::json!({"type": "metaAndAssetCtxs", "dex": "customdex"}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "universe": [
                            {
                                "name": "DOGE",
                                "szDecimals": 0,
                                "maxLeverage": 10
                            }
                        ],
                        "marginTables": []
                    },
                    [
                        {
                            "dayNtlVlm": "500000.0",
                            "funding": "0.00005",
                            "impactPxs": ["0.1", "0.11"],
                            "markPx": "0.105",
                            "midPx": "0.105",
                            "openInterest": "10000.0",
                            "oraclePx": "0.1",
                            "premium": "0.05",
                            "prevDayPx": "0.09"
                        }
                    ]
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let (meta, ctxs) = client.meta_and_asset_ctxs(Some("customdex")).await.unwrap();

        assert_eq!(meta.universe.len(), 1);
        assert_eq!(meta.universe[0].name, "DOGE");
        assert_eq!(ctxs[0].mark_px, "0.105");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_all_perp_metas() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({"type": "allPerpMetas"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "name": "BTC",
                        "szDecimals": 4,
                        "maxLeverage": 50
                    },
                    {
                        "name": "ETH",
                        "szDecimals": 3,
                        "maxLeverage": 50
                    },
                    {
                        "name": "SOL",
                        "szDecimals": 2,
                        "maxLeverage": 20,
                        "onlyIsolated": true
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let metas = client.all_perp_metas().await.unwrap();

        assert_eq!(metas.len(), 3);
        assert_eq!(metas[0].name, "BTC");
        assert_eq!(metas[0].sz_decimals, 4);
        assert_eq!(metas[1].name, "ETH");
        assert_eq!(metas[2].name, "SOL");
        assert_eq!(metas[2].only_isolated, Some(true));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_perp_dexs_error_handling() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.perp_dexs().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("500"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_meta_api_error_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid dex parameter"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result: Result<PerpMetaResponse> = client.meta(Some("invaliddex")).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid dex parameter"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_perp_dexs_request_serialization() {
        let request = PerpDexsRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"perpDexs"}"#);
    }

    #[tokio::test]
    async fn test_meta_request_serialization_without_dex() {
        let request = MetaRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"meta"}"#);
    }

    #[tokio::test]
    async fn test_meta_request_serialization_with_dex() {
        let mut request = MetaRequest::default();
        request.dex = Some("testdex".to_string());
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"meta","dex":"testdex"}"#);
    }

    #[tokio::test]
    async fn test_meta_and_asset_ctxs_request_serialization() {
        let request = MetaAndAssetCtxsRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"metaAndAssetCtxs"}"#);
    }

    #[tokio::test]
    async fn test_all_perp_metas_request_serialization() {
        let request = AllPerpMetasRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"allPerpMetas"}"#);
    }
}
