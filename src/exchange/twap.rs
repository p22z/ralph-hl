//! TWAP order endpoints for the Hyperliquid Exchange API
//!
//! This module provides methods for placing and cancelling Time-Weighted Average Price (TWAP)
//! orders. TWAP orders allow users to execute large trades over time to reduce market impact.

use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::auth::{Signature, Wallet};
use crate::client::Client;
use crate::error::{Error, Result};
use crate::types::{CancelTwapAction, TwapOrderAction, TwapOrderWire, TwapResponse};

/// Exchange request wrapper with authentication for TWAP operations
#[derive(Debug, Clone, Serialize)]
pub struct TwapExchangeRequest<T> {
    pub action: T,
    pub nonce: u64,
    pub signature: Signature,
    #[serde(rename = "vaultAddress", skip_serializing_if = "Option::is_none")]
    pub vault_address: Option<String>,
}

/// Exchange API response wrapper for TWAP operations
#[derive(Debug, Clone, Deserialize)]
pub struct TwapExchangeApiResponse {
    pub status: String,
    pub response: Option<TwapResponse>,
}

impl Client {
    /// Place a TWAP order
    ///
    /// TWAP (Time-Weighted Average Price) orders execute over a specified duration
    /// to reduce market impact for large trades.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the order with
    /// * `asset` - The asset index to trade
    /// * `is_buy` - Whether this is a buy order (true) or sell order (false)
    /// * `size` - The total size to trade
    /// * `duration_minutes` - The duration over which to execute the TWAP (in minutes)
    /// * `randomize` - Whether to randomize the execution timing
    ///
    /// # Returns
    /// The TWAP response containing the TWAP order ID if successful
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Place a TWAP buy order for 10.0 of asset 0 over 30 minutes with randomization
    /// let response = client.place_twap_order(&wallet, 0, true, "10.0", 30, true).await?;
    ///
    /// if let hyperliquid_sdk::TwapOrderStatus::Running { running } = response.data.status {
    ///     println!("TWAP order placed with ID: {}", running.twap_id);
    /// }
    /// ```
    pub async fn place_twap_order(
        &self,
        wallet: &Wallet,
        asset: u32,
        is_buy: bool,
        size: impl Into<String>,
        duration_minutes: u32,
        randomize: bool,
    ) -> Result<TwapResponse> {
        self.place_twap_order_with_options(
            wallet,
            asset,
            is_buy,
            size,
            duration_minutes,
            randomize,
            false,
            None,
        )
        .await
    }

    /// Place a TWAP order with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the order with
    /// * `asset` - The asset index to trade
    /// * `is_buy` - Whether this is a buy order (true) or sell order (false)
    /// * `size` - The total size to trade
    /// * `duration_minutes` - The duration over which to execute the TWAP (in minutes)
    /// * `randomize` - Whether to randomize the execution timing
    /// * `reduce_only` - If true, only reduce an existing position
    /// * `vault_address` - Optional vault address to trade on behalf of
    pub async fn place_twap_order_with_options(
        &self,
        wallet: &Wallet,
        asset: u32,
        is_buy: bool,
        size: impl Into<String>,
        duration_minutes: u32,
        randomize: bool,
        reduce_only: bool,
        vault_address: Option<Address>,
    ) -> Result<TwapResponse> {
        // Validate duration
        if duration_minutes == 0 {
            return Err(Error::InvalidParameter(
                "TWAP duration must be greater than 0".to_string(),
            ));
        }

        // Create the TWAP order action
        let action = TwapOrderAction {
            action_type: "twapOrder".to_string(),
            twap: TwapOrderWire {
                a: asset,
                b: is_buy,
                s: size.into(),
                r: reduce_only,
                m: duration_minutes,
                t: randomize,
            },
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = TwapExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: TwapExchangeApiResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "TWAP order placement failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Cancel a TWAP order
    ///
    /// Cancels an active TWAP order by its ID.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the cancellation with
    /// * `asset` - The asset index of the TWAP order
    /// * `twap_id` - The TWAP order ID to cancel
    ///
    /// # Returns
    /// The TWAP response confirming the cancellation
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Cancel TWAP order with ID 12345 on asset 0
    /// let response = client.cancel_twap_order(&wallet, 0, 12345).await?;
    /// ```
    pub async fn cancel_twap_order(
        &self,
        wallet: &Wallet,
        asset: u32,
        twap_id: u64,
    ) -> Result<TwapResponse> {
        self.cancel_twap_order_with_options(wallet, asset, twap_id, None)
            .await
    }

    /// Cancel a TWAP order with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the cancellation with
    /// * `asset` - The asset index of the TWAP order
    /// * `twap_id` - The TWAP order ID to cancel
    /// * `vault_address` - Optional vault address to cancel on behalf of
    pub async fn cancel_twap_order_with_options(
        &self,
        wallet: &Wallet,
        asset: u32,
        twap_id: u64,
        vault_address: Option<Address>,
    ) -> Result<TwapResponse> {
        // Create the cancel TWAP action
        let action = CancelTwapAction {
            action_type: "twapCancel".to_string(),
            a: asset,
            t: twap_id,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = TwapExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: TwapExchangeApiResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "TWAP order cancellation failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }
}

/// Check if a TWAP order status indicates it's running
pub fn is_twap_running(response: &TwapResponse) -> bool {
    matches!(
        response.data.status,
        crate::types::TwapOrderStatus::Running { .. }
    )
}

/// Extract the TWAP ID from a successful response
pub fn get_twap_id(response: &TwapResponse) -> Option<u64> {
    match &response.data.status {
        crate::types::TwapOrderStatus::Running { running } => Some(running.twap_id),
        crate::types::TwapOrderStatus::Error { .. } => None,
    }
}

/// Extract the error message from a failed TWAP response
pub fn get_twap_error(response: &TwapResponse) -> Option<&str> {
    match &response.data.status {
        crate::types::TwapOrderStatus::Running { .. } => None,
        crate::types::TwapOrderStatus::Error { error } => Some(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TwapOrderStatus;
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

        async fn place_twap_order(
            &self,
            wallet: &Wallet,
            asset: u32,
            is_buy: bool,
            size: impl Into<String>,
            duration_minutes: u32,
            randomize: bool,
        ) -> Result<TwapResponse> {
            self.place_twap_order_with_options(
                wallet,
                asset,
                is_buy,
                size,
                duration_minutes,
                randomize,
                false,
                None,
            )
            .await
        }

        async fn place_twap_order_with_options(
            &self,
            wallet: &Wallet,
            asset: u32,
            is_buy: bool,
            size: impl Into<String>,
            duration_minutes: u32,
            randomize: bool,
            reduce_only: bool,
            vault_address: Option<Address>,
        ) -> Result<TwapResponse> {
            if duration_minutes == 0 {
                return Err(Error::InvalidParameter(
                    "TWAP duration must be greater than 0".to_string(),
                ));
            }

            let action = TwapOrderAction {
                action_type: "twapOrder".to_string(),
                twap: TwapOrderWire {
                    a: asset,
                    b: is_buy,
                    s: size.into(),
                    r: reduce_only,
                    m: duration_minutes,
                    t: randomize,
                },
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

            let request = TwapExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: vault_address.map(|addr| format!("{addr:?}")),
            };

            let response: TwapExchangeApiResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "TWAP order placement failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn cancel_twap_order(
            &self,
            wallet: &Wallet,
            asset: u32,
            twap_id: u64,
        ) -> Result<TwapResponse> {
            self.cancel_twap_order_with_options(wallet, asset, twap_id, None)
                .await
        }

        async fn cancel_twap_order_with_options(
            &self,
            wallet: &Wallet,
            asset: u32,
            twap_id: u64,
            vault_address: Option<Address>,
        ) -> Result<TwapResponse> {
            let action = CancelTwapAction {
                action_type: "twapCancel".to_string(),
                a: asset,
                t: twap_id,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

            let request = TwapExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: vault_address.map(|addr| format!("{addr:?}")),
            };

            let response: TwapExchangeApiResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "TWAP order cancellation failed: {}",
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
    fn test_twap_order_wire_serialization() {
        let twap = TwapOrderWire {
            a: 0,
            b: true,
            s: "10.5".to_string(),
            r: false,
            m: 30,
            t: true,
        };
        let json = serde_json::to_string(&twap).unwrap();
        assert!(json.contains("\"a\":0"));
        assert!(json.contains("\"b\":true"));
        assert!(json.contains("\"s\":\"10.5\""));
        assert!(json.contains("\"r\":false"));
        assert!(json.contains("\"m\":30"));
        assert!(json.contains("\"t\":true"));
    }

    #[test]
    fn test_twap_order_action_serialization() {
        let action = TwapOrderAction {
            action_type: "twapOrder".to_string(),
            twap: TwapOrderWire {
                a: 1,
                b: false,
                s: "5.0".to_string(),
                r: true,
                m: 60,
                t: false,
            },
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"twapOrder\""));
        assert!(json.contains("\"twap\":"));
    }

    #[test]
    fn test_cancel_twap_action_serialization() {
        let action = CancelTwapAction {
            action_type: "twapCancel".to_string(),
            a: 0,
            t: 12345,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"twapCancel\""));
        assert!(json.contains("\"a\":0"));
        assert!(json.contains("\"t\":12345"));
    }

    #[test]
    fn test_twap_exchange_request_serialization() {
        let action = TwapOrderAction {
            action_type: "twapOrder".to_string(),
            twap: TwapOrderWire {
                a: 0,
                b: true,
                s: "10.0".to_string(),
                r: false,
                m: 30,
                t: true,
            },
        };

        let request = TwapExchangeRequest {
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
    fn test_twap_exchange_request_with_vault() {
        let action = TwapOrderAction {
            action_type: "twapOrder".to_string(),
            twap: TwapOrderWire {
                a: 0,
                b: true,
                s: "10.0".to_string(),
                r: false,
                m: 30,
                t: true,
            },
        };

        let request = TwapExchangeRequest {
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

    // Helper function tests

    #[test]
    fn test_is_twap_running_success() {
        let response = TwapResponse {
            response_type: "twapOrder".to_string(),
            data: crate::types::TwapResponseData {
                status: TwapOrderStatus::Running {
                    running: crate::types::TwapRunningStatus { twap_id: 12345 },
                },
            },
        };

        assert!(is_twap_running(&response));
    }

    #[test]
    fn test_is_twap_running_error() {
        let response = TwapResponse {
            response_type: "twapOrder".to_string(),
            data: crate::types::TwapResponseData {
                status: TwapOrderStatus::Error {
                    error: "Insufficient margin".to_string(),
                },
            },
        };

        assert!(!is_twap_running(&response));
    }

    #[test]
    fn test_get_twap_id_success() {
        let response = TwapResponse {
            response_type: "twapOrder".to_string(),
            data: crate::types::TwapResponseData {
                status: TwapOrderStatus::Running {
                    running: crate::types::TwapRunningStatus { twap_id: 12345 },
                },
            },
        };

        assert_eq!(get_twap_id(&response), Some(12345));
    }

    #[test]
    fn test_get_twap_id_error() {
        let response = TwapResponse {
            response_type: "twapOrder".to_string(),
            data: crate::types::TwapResponseData {
                status: TwapOrderStatus::Error {
                    error: "Insufficient margin".to_string(),
                },
            },
        };

        assert_eq!(get_twap_id(&response), None);
    }

    #[test]
    fn test_get_twap_error_success() {
        let response = TwapResponse {
            response_type: "twapOrder".to_string(),
            data: crate::types::TwapResponseData {
                status: TwapOrderStatus::Running {
                    running: crate::types::TwapRunningStatus { twap_id: 12345 },
                },
            },
        };

        assert_eq!(get_twap_error(&response), None);
    }

    #[test]
    fn test_get_twap_error_failure() {
        let response = TwapResponse {
            response_type: "twapOrder".to_string(),
            data: crate::types::TwapResponseData {
                status: TwapOrderStatus::Error {
                    error: "Insufficient margin".to_string(),
                },
            },
        };

        assert_eq!(get_twap_error(&response), Some("Insufficient margin"));
    }

    // Integration tests with mocked HTTP responses

    #[tokio::test]
    async fn test_place_twap_order_success() {
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
                        "type": "twapOrder",
                        "data": {
                            "status": {
                                "running": {
                                    "twapId": 12345
                                }
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
            .place_twap_order(&wallet, 0, true, "10.0", 30, true)
            .await
            .unwrap();

        assert_eq!(response.response_type, "twapOrder");
        assert!(is_twap_running(&response));
        assert_eq!(get_twap_id(&response), Some(12345));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_place_twap_order_with_reduce_only() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "twapOrder",
                        "data": {
                            "status": {
                                "running": {
                                    "twapId": 67890
                                }
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
            .place_twap_order_with_options(&wallet, 0, false, "5.0", 60, false, true, None)
            .await
            .unwrap();

        assert_eq!(get_twap_id(&response), Some(67890));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_place_twap_order_error_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "twapOrder",
                        "data": {
                            "status": {
                                "error": "Insufficient margin"
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
            .place_twap_order(&wallet, 0, true, "100.0", 30, true)
            .await
            .unwrap();

        assert!(!is_twap_running(&response));
        assert_eq!(get_twap_error(&response), Some("Insufficient margin"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_place_twap_order_zero_duration_error() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client
            .place_twap_order(&wallet, 0, true, "10.0", 0, true)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("duration"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[tokio::test]
    async fn test_cancel_twap_order_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "twapCancel",
                        "data": {
                            "status": {
                                "running": {
                                    "twapId": 12345
                                }
                            }
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client.cancel_twap_order(&wallet, 0, 12345).await.unwrap();

        assert_eq!(response.response_type, "twapCancel");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_cancel_twap_order_not_found() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "twapCancel",
                        "data": {
                            "status": {
                                "error": "TWAP order not found"
                            }
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client.cancel_twap_order(&wallet, 0, 99999).await.unwrap();

        assert_eq!(get_twap_error(&response), Some("TWAP order not found"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_twap_order_http_error() {
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
            .place_twap_order(&wallet, 0, true, "10.0", 30, true)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("500"));
            }
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_twap_order_api_error_status() {
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
            .place_twap_order(&wallet, 0, true, "10.0", 30, true)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("TWAP order placement failed"));
            }
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_cancel_twap_order_unauthorized() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(401)
            .with_body("Unauthorized: invalid signature")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.cancel_twap_order(&wallet, 0, 12345).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("401"));
            }
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_twap_order_rate_limited() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(429)
            .with_body("Rate limit exceeded")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client
            .place_twap_order(&wallet, 0, true, "10.0", 30, true)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("429"));
            }
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    // Wallet signing tests

    #[test]
    fn test_wallet_signing_produces_signature_for_twap_order() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = TwapOrderAction {
                action_type: "twapOrder".to_string(),
                twap: TwapOrderWire {
                    a: 0,
                    b: true,
                    s: "10.0".to_string(),
                    r: false,
                    m: 30,
                    t: true,
                },
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_produces_signature_for_cancel_twap() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = CancelTwapAction {
                action_type: "twapCancel".to_string(),
                a: 0,
                t: 12345,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    // Response deserialization tests

    #[test]
    fn test_twap_response_deserialization_running() {
        let json = r#"{
            "type": "twapOrder",
            "data": {
                "status": {
                    "running": {
                        "twapId": 12345
                    }
                }
            }
        }"#;

        let response: TwapResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.response_type, "twapOrder");
        assert!(is_twap_running(&response));
        assert_eq!(get_twap_id(&response), Some(12345));
    }

    #[test]
    fn test_twap_response_deserialization_error() {
        let json = r#"{
            "type": "twapOrder",
            "data": {
                "status": {
                    "error": "Insufficient margin"
                }
            }
        }"#;

        let response: TwapResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.response_type, "twapOrder");
        assert!(!is_twap_running(&response));
        assert_eq!(get_twap_error(&response), Some("Insufficient margin"));
    }

    #[test]
    fn test_twap_api_response_deserialization() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "twapOrder",
                "data": {
                    "status": {
                        "running": {
                            "twapId": 67890
                        }
                    }
                }
            }
        }"#;

        let response: TwapExchangeApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let twap_response = response.response.unwrap();
        assert_eq!(get_twap_id(&twap_response), Some(67890));
    }
}
