//! Leverage and margin endpoints for the Hyperliquid Exchange API
//!
//! This module provides methods for updating leverage settings and isolated margin
//! for perpetual positions on the Hyperliquid exchange.

use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::auth::{Signature, Wallet};
use crate::client::Client;
use crate::error::{Error, Result};
use crate::types::{UpdateIsolatedMarginAction, UpdateLeverageAction};

/// Exchange request wrapper with authentication for leverage operations
#[derive(Debug, Clone, Serialize)]
pub struct LeverageExchangeRequest<T> {
    pub action: T,
    pub nonce: u64,
    pub signature: Signature,
    #[serde(rename = "vaultAddress", skip_serializing_if = "Option::is_none")]
    pub vault_address: Option<String>,
}

/// Exchange API response for leverage/margin operations
#[derive(Debug, Clone, Deserialize)]
pub struct LeverageResponse {
    pub status: String,
    pub response: Option<LeverageResponseData>,
}

/// Response data for leverage operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeverageResponseData {
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: LeverageResultData,
}

/// Result data for leverage operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeverageResultData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leverage: Option<LeverageResultInfo>,
}

/// Leverage result info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeverageResultInfo {
    #[serde(rename = "type")]
    pub leverage_type: String,
    pub value: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_usd: Option<String>,
}

/// Exchange API response for isolated margin operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolatedMarginResponse {
    pub status: String,
    pub response: Option<IsolatedMarginResponseData>,
}

/// Response data for isolated margin operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolatedMarginResponseData {
    #[serde(rename = "type")]
    pub response_type: String,
}

impl Client {
    /// Update leverage for a specific asset
    ///
    /// Sets the leverage for a perpetual asset. The leverage mode can be either
    /// cross margin (shared margin across positions) or isolated margin
    /// (separate margin per position).
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `asset` - The asset index to update leverage for
    /// * `is_cross` - If true, use cross margin; if false, use isolated margin
    /// * `leverage` - The leverage value (e.g., 10 for 10x leverage)
    ///
    /// # Returns
    /// The leverage response containing the updated leverage settings
    ///
    /// # Errors
    /// Returns an error if:
    /// - The leverage value is invalid (must be > 0)
    /// - The wallet signing fails
    /// - The API returns an error
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Set 10x cross margin leverage for BTC (asset 0)
    /// let response = client.update_leverage(&wallet, 0, true, 10).await?;
    ///
    /// // Set 5x isolated margin leverage for ETH (asset 1)
    /// let response = client.update_leverage(&wallet, 1, false, 5).await?;
    /// ```
    pub async fn update_leverage(
        &self,
        wallet: &Wallet,
        asset: u32,
        is_cross: bool,
        leverage: u32,
    ) -> Result<LeverageResponseData> {
        self.update_leverage_with_options(wallet, asset, is_cross, leverage, None)
            .await
    }

    /// Update leverage for a specific asset with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `asset` - The asset index to update leverage for
    /// * `is_cross` - If true, use cross margin; if false, use isolated margin
    /// * `leverage` - The leverage value (e.g., 10 for 10x leverage)
    /// * `vault_address` - Optional vault address to update leverage on behalf of
    pub async fn update_leverage_with_options(
        &self,
        wallet: &Wallet,
        asset: u32,
        is_cross: bool,
        leverage: u32,
        vault_address: Option<Address>,
    ) -> Result<LeverageResponseData> {
        // Validate leverage
        if leverage == 0 {
            return Err(Error::InvalidParameter(
                "Leverage must be greater than 0".to_string(),
            ));
        }

        // Create the update leverage action
        let action = UpdateLeverageAction {
            action_type: "updateLeverage".to_string(),
            asset,
            is_cross,
            leverage,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = LeverageExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: LeverageResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Update leverage failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Update isolated margin for a specific asset position
    ///
    /// Adds or removes margin from an isolated margin position. The margin delta
    /// is specified in integer units (scaled by 1e6 for USDC).
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `asset` - The asset index of the position
    /// * `is_buy` - If true, the position is long; if false, the position is short
    /// * `ntli` - The margin delta in integer units (positive to add, negative to remove)
    ///
    /// # Returns
    /// The isolated margin response
    ///
    /// # Errors
    /// Returns an error if:
    /// - The wallet signing fails
    /// - The API returns an error
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Add 100 USDC margin to a long BTC position (ntli = 100 * 1e6)
    /// let response = client.update_isolated_margin(&wallet, 0, true, 100_000_000).await?;
    ///
    /// // Remove 50 USDC margin from a short ETH position (ntli = -50 * 1e6)
    /// let response = client.update_isolated_margin(&wallet, 1, false, -50_000_000).await?;
    /// ```
    pub async fn update_isolated_margin(
        &self,
        wallet: &Wallet,
        asset: u32,
        is_buy: bool,
        ntli: i64,
    ) -> Result<IsolatedMarginResponseData> {
        self.update_isolated_margin_with_options(wallet, asset, is_buy, ntli, None)
            .await
    }

    /// Update isolated margin with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `asset` - The asset index of the position
    /// * `is_buy` - If true, the position is long; if false, the position is short
    /// * `ntli` - The margin delta in integer units (positive to add, negative to remove)
    /// * `vault_address` - Optional vault address to update margin on behalf of
    pub async fn update_isolated_margin_with_options(
        &self,
        wallet: &Wallet,
        asset: u32,
        is_buy: bool,
        ntli: i64,
        vault_address: Option<Address>,
    ) -> Result<IsolatedMarginResponseData> {
        // Create the update isolated margin action
        let action = UpdateIsolatedMarginAction {
            action_type: "updateIsolatedMargin".to_string(),
            asset,
            is_buy,
            ntli,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = LeverageExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: IsolatedMarginResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Update isolated margin failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }
}

/// Check if a leverage update was successful
pub fn is_leverage_update_successful(response: &LeverageResponseData) -> bool {
    response.data.leverage.is_some()
}

/// Get the updated leverage info from a successful response
pub fn get_updated_leverage(response: &LeverageResponseData) -> Option<&LeverageResultInfo> {
    response.data.leverage.as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use reqwest::Client as ReqwestClient;
    use serde::de::DeserializeOwned;

    const TEST_PRIVATE_KEY: &str =
        "0x0123456789012345678901234567890123456789012345678901234567890123";

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

        fn exchange_url(&self) -> String {
            format!("{}/exchange", self.base_url)
        }

        async fn post_exchange<T, R>(&self, request: &T) -> Result<R>
        where
            T: Serialize + ?Sized,
            R: DeserializeOwned,
        {
            let url = self.exchange_url();
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

        async fn update_leverage(
            &self,
            wallet: &Wallet,
            asset: u32,
            is_cross: bool,
            leverage: u32,
        ) -> Result<LeverageResponseData> {
            self.update_leverage_with_options(wallet, asset, is_cross, leverage, None)
                .await
        }

        async fn update_leverage_with_options(
            &self,
            wallet: &Wallet,
            asset: u32,
            is_cross: bool,
            leverage: u32,
            vault_address: Option<Address>,
        ) -> Result<LeverageResponseData> {
            if leverage == 0 {
                return Err(Error::InvalidParameter(
                    "Leverage must be greater than 0".to_string(),
                ));
            }

            let action = UpdateLeverageAction {
                action_type: "updateLeverage".to_string(),
                asset,
                is_cross,
                leverage,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

            let request = LeverageExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: vault_address.map(|addr| format!("{addr:?}")),
            };

            let response: LeverageResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Update leverage failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn update_isolated_margin(
            &self,
            wallet: &Wallet,
            asset: u32,
            is_buy: bool,
            ntli: i64,
        ) -> Result<IsolatedMarginResponseData> {
            self.update_isolated_margin_with_options(wallet, asset, is_buy, ntli, None)
                .await
        }

        async fn update_isolated_margin_with_options(
            &self,
            wallet: &Wallet,
            asset: u32,
            is_buy: bool,
            ntli: i64,
            vault_address: Option<Address>,
        ) -> Result<IsolatedMarginResponseData> {
            let action = UpdateIsolatedMarginAction {
                action_type: "updateIsolatedMargin".to_string(),
                asset,
                is_buy,
                ntli,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

            let request = LeverageExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: vault_address.map(|addr| format!("{addr:?}")),
            };

            let response: IsolatedMarginResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Update isolated margin failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }
    }

    // Serialization tests

    #[test]
    fn test_update_leverage_action_serialization() {
        let action = UpdateLeverageAction {
            action_type: "updateLeverage".to_string(),
            asset: 0,
            is_cross: true,
            leverage: 10,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"updateLeverage\""));
        assert!(json.contains("\"asset\":0"));
        assert!(json.contains("\"isCross\":true"));
        assert!(json.contains("\"leverage\":10"));
    }

    #[test]
    fn test_update_leverage_action_isolated() {
        let action = UpdateLeverageAction {
            action_type: "updateLeverage".to_string(),
            asset: 1,
            is_cross: false,
            leverage: 5,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"isCross\":false"));
        assert!(json.contains("\"leverage\":5"));
    }

    #[test]
    fn test_update_isolated_margin_action_serialization() {
        let action = UpdateIsolatedMarginAction {
            action_type: "updateIsolatedMargin".to_string(),
            asset: 0,
            is_buy: true,
            ntli: 100_000_000,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"updateIsolatedMargin\""));
        assert!(json.contains("\"asset\":0"));
        assert!(json.contains("\"isBuy\":true"));
        assert!(json.contains("\"ntli\":100000000"));
    }

    #[test]
    fn test_update_isolated_margin_action_negative() {
        let action = UpdateIsolatedMarginAction {
            action_type: "updateIsolatedMargin".to_string(),
            asset: 1,
            is_buy: false,
            ntli: -50_000_000,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"isBuy\":false"));
        assert!(json.contains("\"ntli\":-50000000"));
    }

    #[test]
    fn test_leverage_exchange_request_serialization() {
        let action = UpdateLeverageAction {
            action_type: "updateLeverage".to_string(),
            asset: 0,
            is_cross: true,
            leverage: 10,
        };

        let request = LeverageExchangeRequest {
            action,
            nonce: 1700000000000,
            signature: Signature {
                r: "0x1234".to_string(),
                s: "0x5678".to_string(),
                v: 27,
            },
            vault_address: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"nonce\":1700000000000"));
        assert!(json.contains("\"signature\":"));
        assert!(!json.contains("\"vaultAddress\""));
    }

    #[test]
    fn test_leverage_exchange_request_with_vault() {
        let action = UpdateLeverageAction {
            action_type: "updateLeverage".to_string(),
            asset: 0,
            is_cross: true,
            leverage: 10,
        };

        let request = LeverageExchangeRequest {
            action,
            nonce: 1700000000000,
            signature: Signature {
                r: "0x1234".to_string(),
                s: "0x5678".to_string(),
                v: 27,
            },
            vault_address: Some("0x1234567890123456789012345678901234567890".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"vaultAddress\":\"0x1234567890123456789012345678901234567890\""));
    }

    // Response deserialization tests

    #[test]
    fn test_leverage_response_deserialization() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "updateLeverage",
                "data": {
                    "leverage": {
                        "type": "cross",
                        "value": 10,
                        "rawUsd": "1000.0"
                    }
                }
            }
        }"#;

        let response: LeverageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "updateLeverage");
        assert!(data.data.leverage.is_some());
        let leverage = data.data.leverage.unwrap();
        assert_eq!(leverage.leverage_type, "cross");
        assert_eq!(leverage.value, 10);
        assert_eq!(leverage.raw_usd, Some("1000.0".to_string()));
    }

    #[test]
    fn test_leverage_response_isolated() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "updateLeverage",
                "data": {
                    "leverage": {
                        "type": "isolated",
                        "value": 5
                    }
                }
            }
        }"#;

        let response: LeverageResponse = serde_json::from_str(json).unwrap();
        let data = response.response.unwrap();
        let leverage = data.data.leverage.unwrap();
        assert_eq!(leverage.leverage_type, "isolated");
        assert_eq!(leverage.value, 5);
        assert_eq!(leverage.raw_usd, None);
    }

    #[test]
    fn test_isolated_margin_response_deserialization() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "updateIsolatedMargin"
            }
        }"#;

        let response: IsolatedMarginResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "updateIsolatedMargin");
    }

    // Helper function tests

    #[test]
    fn test_is_leverage_update_successful_true() {
        let response = LeverageResponseData {
            response_type: "updateLeverage".to_string(),
            data: LeverageResultData {
                leverage: Some(LeverageResultInfo {
                    leverage_type: "cross".to_string(),
                    value: 10,
                    raw_usd: None,
                }),
            },
        };
        assert!(is_leverage_update_successful(&response));
    }

    #[test]
    fn test_is_leverage_update_successful_false() {
        let response = LeverageResponseData {
            response_type: "updateLeverage".to_string(),
            data: LeverageResultData { leverage: None },
        };
        assert!(!is_leverage_update_successful(&response));
    }

    #[test]
    fn test_get_updated_leverage_some() {
        let response = LeverageResponseData {
            response_type: "updateLeverage".to_string(),
            data: LeverageResultData {
                leverage: Some(LeverageResultInfo {
                    leverage_type: "cross".to_string(),
                    value: 10,
                    raw_usd: Some("5000.0".to_string()),
                }),
            },
        };
        let leverage = get_updated_leverage(&response).unwrap();
        assert_eq!(leverage.value, 10);
        assert_eq!(leverage.leverage_type, "cross");
    }

    #[test]
    fn test_get_updated_leverage_none() {
        let response = LeverageResponseData {
            response_type: "updateLeverage".to_string(),
            data: LeverageResultData { leverage: None },
        };
        assert!(get_updated_leverage(&response).is_none());
    }

    // Validation tests

    #[test]
    fn test_leverage_validation_zero() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.update_leverage(&wallet, 0, true, 0));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("greater than 0"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    // Integration tests with mocked HTTP responses

    #[tokio::test]
    async fn test_update_leverage_cross_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "updateLeverage",
                        "data": {
                            "leverage": {
                                "type": "cross",
                                "value": 10,
                                "rawUsd": "5000.0"
                            }
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .update_leverage(&wallet, 0, true, 10)
            .await
            .unwrap();

        assert_eq!(response.response_type, "updateLeverage");
        assert!(is_leverage_update_successful(&response));
        let leverage = get_updated_leverage(&response).unwrap();
        assert_eq!(leverage.leverage_type, "cross");
        assert_eq!(leverage.value, 10);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_update_leverage_isolated_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "updateLeverage",
                        "data": {
                            "leverage": {
                                "type": "isolated",
                                "value": 5
                            }
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .update_leverage(&wallet, 1, false, 5)
            .await
            .unwrap();

        let leverage = get_updated_leverage(&response).unwrap();
        assert_eq!(leverage.leverage_type, "isolated");
        assert_eq!(leverage.value, 5);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_update_leverage_with_vault() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "updateLeverage",
                        "data": {
                            "leverage": {
                                "type": "cross",
                                "value": 20
                            }
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let vault_address: Address = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();

        let response = client
            .update_leverage_with_options(&wallet, 0, true, 20, Some(vault_address))
            .await
            .unwrap();

        assert!(is_leverage_update_successful(&response));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_update_isolated_margin_add_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "updateIsolatedMargin"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .update_isolated_margin(&wallet, 0, true, 100_000_000)
            .await
            .unwrap();

        assert_eq!(response.response_type, "updateIsolatedMargin");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_update_isolated_margin_remove_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "updateIsolatedMargin"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .update_isolated_margin(&wallet, 1, false, -50_000_000)
            .await
            .unwrap();

        assert_eq!(response.response_type, "updateIsolatedMargin");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_update_isolated_margin_with_vault() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "updateIsolatedMargin"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let vault_address: Address = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();

        let response = client
            .update_isolated_margin_with_options(&wallet, 0, true, 100_000_000, Some(vault_address))
            .await
            .unwrap();

        assert_eq!(response.response_type, "updateIsolatedMargin");

        mock.assert_async().await;
    }

    // Error handling tests

    #[tokio::test]
    async fn test_update_leverage_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.update_leverage(&wallet, 0, true, 10).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("500"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_update_leverage_api_error_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "err", "response": null}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.update_leverage(&wallet, 0, true, 10).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Update leverage failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_update_leverage_unauthorized() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(401)
            .with_body("Unauthorized: invalid signature")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.update_leverage(&wallet, 0, true, 10).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("401"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_update_isolated_margin_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client
            .update_isolated_margin(&wallet, 0, true, 100_000_000)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("500"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_update_isolated_margin_api_error_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "err", "response": null}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client
            .update_isolated_margin(&wallet, 0, true, 100_000_000)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Update isolated margin failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    // Wallet signing tests

    #[test]
    fn test_wallet_signing_produces_signature_for_leverage() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = UpdateLeverageAction {
                action_type: "updateLeverage".to_string(),
                asset: 0,
                is_cross: true,
                leverage: 10,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_produces_signature_for_isolated_margin() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = UpdateIsolatedMarginAction {
                action_type: "updateIsolatedMargin".to_string(),
                asset: 0,
                is_buy: true,
                ntli: 100_000_000,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }
}
