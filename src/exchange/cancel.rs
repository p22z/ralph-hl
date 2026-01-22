//! Order cancellation endpoints for the Hyperliquid Exchange API
//!
//! This module provides methods for cancelling single and batch orders,
//! supporting cancellation by order ID (oid) or client order ID (cloid).

use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::auth::{Signature, Wallet};
use crate::client::Client;
use crate::error::{Error, Result};
use crate::types::{CancelAction, CancelByCloidAction, CancelByCloidWire, CancelResponse, CancelWire};

/// Exchange request wrapper with authentication
#[derive(Debug, Clone, Serialize)]
pub struct CancelExchangeRequest<T> {
    pub action: T,
    pub nonce: u64,
    pub signature: Signature,
    #[serde(rename = "vaultAddress", skip_serializing_if = "Option::is_none")]
    pub vault_address: Option<String>,
}

/// Exchange API response wrapper for cancel operations
#[derive(Debug, Clone, Deserialize)]
pub struct CancelExchangeApiResponse {
    pub status: String,
    pub response: Option<CancelResponse>,
}

impl Client {
    /// Cancel a single order by order ID
    ///
    /// Cancels an open order using its numeric order ID (oid).
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the cancellation with
    /// * `asset` - The asset index of the order to cancel
    /// * `oid` - The order ID to cancel
    ///
    /// # Returns
    /// The cancel response containing the status of the cancellation
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Cancel order with ID 12345 on asset 0 (BTC)
    /// let response = client.cancel_order(&wallet, 0, 12345).await?;
    /// ```
    pub async fn cancel_order(
        &self,
        wallet: &Wallet,
        asset: u32,
        oid: u64,
    ) -> Result<CancelResponse> {
        self.cancel_order_with_options(wallet, asset, oid, None).await
    }

    /// Cancel a single order by order ID with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the cancellation with
    /// * `asset` - The asset index of the order to cancel
    /// * `oid` - The order ID to cancel
    /// * `vault_address` - Optional vault address to cancel on behalf of
    pub async fn cancel_order_with_options(
        &self,
        wallet: &Wallet,
        asset: u32,
        oid: u64,
        vault_address: Option<Address>,
    ) -> Result<CancelResponse> {
        self.cancel_batch_orders_with_options(wallet, vec![(asset, oid)], vault_address)
            .await
    }

    /// Cancel a single order by client order ID (cloid)
    ///
    /// Cancels an open order using its client-provided order ID.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the cancellation with
    /// * `asset` - The asset index of the order to cancel
    /// * `cloid` - The client order ID to cancel
    ///
    /// # Returns
    /// The cancel response containing the status of the cancellation
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Cancel order with client ID "my-order-1" on asset 0 (BTC)
    /// let response = client.cancel_order_by_cloid(&wallet, 0, "my-order-1").await?;
    /// ```
    pub async fn cancel_order_by_cloid(
        &self,
        wallet: &Wallet,
        asset: u32,
        cloid: impl Into<String>,
    ) -> Result<CancelResponse> {
        self.cancel_order_by_cloid_with_options(wallet, asset, cloid, None)
            .await
    }

    /// Cancel a single order by client order ID with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the cancellation with
    /// * `asset` - The asset index of the order to cancel
    /// * `cloid` - The client order ID to cancel
    /// * `vault_address` - Optional vault address to cancel on behalf of
    pub async fn cancel_order_by_cloid_with_options(
        &self,
        wallet: &Wallet,
        asset: u32,
        cloid: impl Into<String>,
        vault_address: Option<Address>,
    ) -> Result<CancelResponse> {
        self.cancel_batch_orders_by_cloid_with_options(
            wallet,
            vec![(asset, cloid.into())],
            vault_address,
        )
        .await
    }

    /// Cancel multiple orders in a single request
    ///
    /// Cancels multiple orders atomically using their order IDs.
    /// This is more efficient than cancelling orders individually.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the cancellations with
    /// * `cancels` - A list of (asset, oid) pairs to cancel
    ///
    /// # Returns
    /// The cancel response containing statuses for all cancellations
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Cancel multiple orders
    /// let cancels = vec![
    ///     (0, 12345), // Asset 0, order ID 12345
    ///     (0, 12346), // Asset 0, order ID 12346
    ///     (1, 67890), // Asset 1, order ID 67890
    /// ];
    ///
    /// let response = client.cancel_batch_orders(&wallet, cancels).await?;
    /// for status in &response.data.statuses {
    ///     println!("{}", status);
    /// }
    /// ```
    pub async fn cancel_batch_orders(
        &self,
        wallet: &Wallet,
        cancels: Vec<(u32, u64)>,
    ) -> Result<CancelResponse> {
        self.cancel_batch_orders_with_options(wallet, cancels, None)
            .await
    }

    /// Cancel multiple orders with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the cancellations with
    /// * `cancels` - A list of (asset, oid) pairs to cancel
    /// * `vault_address` - Optional vault address to cancel on behalf of
    pub async fn cancel_batch_orders_with_options(
        &self,
        wallet: &Wallet,
        cancels: Vec<(u32, u64)>,
        vault_address: Option<Address>,
    ) -> Result<CancelResponse> {
        if cancels.is_empty() {
            return Err(Error::InvalidParameter(
                "Cancels list cannot be empty".to_string(),
            ));
        }

        // Create the cancel wires
        let cancel_wires: Vec<CancelWire> = cancels
            .into_iter()
            .map(|(asset, oid)| CancelWire { a: asset, o: oid })
            .collect();

        // Create the cancel action
        let action = CancelAction {
            action_type: "cancel".to_string(),
            cancels: cancel_wires,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = CancelExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: CancelExchangeApiResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Order cancellation failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Cancel multiple orders by client order ID in a single request
    ///
    /// Cancels multiple orders atomically using their client-provided order IDs.
    /// This is more efficient than cancelling orders individually.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the cancellations with
    /// * `cancels` - A list of (asset, cloid) pairs to cancel
    ///
    /// # Returns
    /// The cancel response containing statuses for all cancellations
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Cancel multiple orders by client ID
    /// let cancels = vec![
    ///     (0, "order-1".to_string()),
    ///     (0, "order-2".to_string()),
    ///     (1, "order-3".to_string()),
    /// ];
    ///
    /// let response = client.cancel_batch_orders_by_cloid(&wallet, cancels).await?;
    /// ```
    pub async fn cancel_batch_orders_by_cloid(
        &self,
        wallet: &Wallet,
        cancels: Vec<(u32, String)>,
    ) -> Result<CancelResponse> {
        self.cancel_batch_orders_by_cloid_with_options(wallet, cancels, None)
            .await
    }

    /// Cancel multiple orders by client order ID with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the cancellations with
    /// * `cancels` - A list of (asset, cloid) pairs to cancel
    /// * `vault_address` - Optional vault address to cancel on behalf of
    pub async fn cancel_batch_orders_by_cloid_with_options(
        &self,
        wallet: &Wallet,
        cancels: Vec<(u32, String)>,
        vault_address: Option<Address>,
    ) -> Result<CancelResponse> {
        if cancels.is_empty() {
            return Err(Error::InvalidParameter(
                "Cancels list cannot be empty".to_string(),
            ));
        }

        // Create the cancel by cloid wires
        let cancel_wires: Vec<CancelByCloidWire> = cancels
            .into_iter()
            .map(|(asset, cloid)| CancelByCloidWire { asset, cloid })
            .collect();

        // Create the cancel by cloid action
        let action = CancelByCloidAction {
            action_type: "cancelByCloid".to_string(),
            cancels: cancel_wires,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = CancelExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: CancelExchangeApiResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Order cancellation failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }
}

/// Check if a cancel status indicates success
pub fn is_cancel_successful(status: &str) -> bool {
    status == "success"
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

        async fn cancel_order(
            &self,
            wallet: &Wallet,
            asset: u32,
            oid: u64,
        ) -> Result<CancelResponse> {
            self.cancel_batch_orders(wallet, vec![(asset, oid)]).await
        }

        async fn cancel_order_by_cloid(
            &self,
            wallet: &Wallet,
            asset: u32,
            cloid: impl Into<String>,
        ) -> Result<CancelResponse> {
            self.cancel_batch_orders_by_cloid(wallet, vec![(asset, cloid.into())])
                .await
        }

        async fn cancel_batch_orders(
            &self,
            wallet: &Wallet,
            cancels: Vec<(u32, u64)>,
        ) -> Result<CancelResponse> {
            if cancels.is_empty() {
                return Err(Error::InvalidParameter(
                    "Cancels list cannot be empty".to_string(),
                ));
            }

            let cancel_wires: Vec<CancelWire> = cancels
                .into_iter()
                .map(|(asset, oid)| CancelWire { a: asset, o: oid })
                .collect();

            let action = CancelAction {
                action_type: "cancel".to_string(),
                cancels: cancel_wires,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = CancelExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: None,
            };

            let response: CancelExchangeApiResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Order cancellation failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn cancel_batch_orders_by_cloid(
            &self,
            wallet: &Wallet,
            cancels: Vec<(u32, String)>,
        ) -> Result<CancelResponse> {
            if cancels.is_empty() {
                return Err(Error::InvalidParameter(
                    "Cancels list cannot be empty".to_string(),
                ));
            }

            let cancel_wires: Vec<CancelByCloidWire> = cancels
                .into_iter()
                .map(|(asset, cloid)| CancelByCloidWire { asset, cloid })
                .collect();

            let action = CancelByCloidAction {
                action_type: "cancelByCloid".to_string(),
                cancels: cancel_wires,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = CancelExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: None,
            };

            let response: CancelExchangeApiResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Order cancellation failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }
    }

    #[test]
    fn test_cancel_wire_serialization() {
        let cancel = CancelWire { a: 0, o: 12345 };
        let json = serde_json::to_string(&cancel).unwrap();
        assert!(json.contains("\"a\":0"));
        assert!(json.contains("\"o\":12345"));
    }

    #[test]
    fn test_cancel_by_cloid_wire_serialization() {
        let cancel = CancelByCloidWire {
            asset: 0,
            cloid: "my-order-1".to_string(),
        };
        let json = serde_json::to_string(&cancel).unwrap();
        assert!(json.contains("\"asset\":0"));
        assert!(json.contains("\"cloid\":\"my-order-1\""));
    }

    #[test]
    fn test_cancel_action_serialization() {
        let action = CancelAction {
            action_type: "cancel".to_string(),
            cancels: vec![CancelWire { a: 0, o: 12345 }],
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"cancel\""));
        assert!(json.contains("\"cancels\":["));
    }

    #[test]
    fn test_cancel_by_cloid_action_serialization() {
        let action = CancelByCloidAction {
            action_type: "cancelByCloid".to_string(),
            cancels: vec![CancelByCloidWire {
                asset: 0,
                cloid: "my-order-1".to_string(),
            }],
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"cancelByCloid\""));
        assert!(json.contains("\"cancels\":["));
    }

    #[test]
    fn test_cancel_exchange_request_serialization() {
        let action = CancelAction {
            action_type: "cancel".to_string(),
            cancels: vec![CancelWire { a: 0, o: 12345 }],
        };

        let request = CancelExchangeRequest {
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
    fn test_cancel_exchange_request_with_vault() {
        let action = CancelAction {
            action_type: "cancel".to_string(),
            cancels: vec![CancelWire { a: 0, o: 12345 }],
        };

        let request = CancelExchangeRequest {
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

    #[test]
    fn test_is_cancel_successful() {
        assert!(is_cancel_successful("success"));
        assert!(!is_cancel_successful("error: Order not found"));
        assert!(!is_cancel_successful("already_cancelled"));
    }

    #[tokio::test]
    async fn test_cancel_order_success() {
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
                        "type": "cancel",
                        "data": {
                            "statuses": ["success"]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client.cancel_order(&wallet, 0, 12345).await.unwrap();

        assert_eq!(response.response_type, "cancel");
        assert_eq!(response.data.statuses.len(), 1);
        assert_eq!(response.data.statuses[0], "success");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_cancel_order_not_found() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "cancel",
                        "data": {
                            "statuses": ["error: Order not found"]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client.cancel_order(&wallet, 0, 99999).await.unwrap();

        assert_eq!(response.data.statuses[0], "error: Order not found");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_cancel_order_by_cloid_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "cancel",
                        "data": {
                            "statuses": ["success"]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .cancel_order_by_cloid(&wallet, 0, "my-order-1")
            .await
            .unwrap();

        assert_eq!(response.response_type, "cancel");
        assert_eq!(response.data.statuses[0], "success");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_cancel_batch_orders_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "cancel",
                        "data": {
                            "statuses": ["success", "success", "success"]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let cancels = vec![(0, 12345), (0, 12346), (1, 67890)];

        let response = client.cancel_batch_orders(&wallet, cancels).await.unwrap();

        assert_eq!(response.data.statuses.len(), 3);
        assert!(response.data.statuses.iter().all(|s| s == "success"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_cancel_batch_orders_partial_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "cancel",
                        "data": {
                            "statuses": ["success", "error: Order not found", "success"]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let cancels = vec![(0, 12345), (0, 99999), (1, 67890)];

        let response = client.cancel_batch_orders(&wallet, cancels).await.unwrap();

        assert_eq!(response.data.statuses.len(), 3);
        assert_eq!(response.data.statuses[0], "success");
        assert_eq!(response.data.statuses[1], "error: Order not found");
        assert_eq!(response.data.statuses[2], "success");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_cancel_batch_orders_empty_error() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.cancel_batch_orders(&wallet, vec![]).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[tokio::test]
    async fn test_cancel_batch_orders_by_cloid_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "cancel",
                        "data": {
                            "statuses": ["success", "success"]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let cancels = vec![
            (0, "order-1".to_string()),
            (0, "order-2".to_string()),
        ];

        let response = client
            .cancel_batch_orders_by_cloid(&wallet, cancels)
            .await
            .unwrap();

        assert_eq!(response.data.statuses.len(), 2);
        assert!(response.data.statuses.iter().all(|s| s == "success"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_cancel_batch_orders_by_cloid_empty_error() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client
            .cancel_batch_orders_by_cloid(&wallet, vec![])
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[tokio::test]
    async fn test_cancel_order_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.cancel_order(&wallet, 0, 12345).await;

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
    async fn test_cancel_order_unauthorized() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(401)
            .with_body("Unauthorized: invalid signature")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.cancel_order(&wallet, 0, 12345).await;

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
    async fn test_cancel_order_rate_limited() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(429)
            .with_body("Rate limit exceeded")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.cancel_order(&wallet, 0, 12345).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("429"));
            }
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_cancel_order_api_error_status() {
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

        let result = client.cancel_order(&wallet, 0, 12345).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Order cancellation failed"));
            }
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[test]
    fn test_wallet_signing_produces_signature_for_cancel() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = CancelAction {
                action_type: "cancel".to_string(),
                cancels: vec![CancelWire { a: 0, o: 12345 }],
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_produces_signature_for_cancel_by_cloid() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = CancelByCloidAction {
                action_type: "cancelByCloid".to_string(),
                cancels: vec![CancelByCloidWire {
                    asset: 0,
                    cloid: "my-order-1".to_string(),
                }],
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }
}
