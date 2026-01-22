//! Order modification endpoints for the Hyperliquid Exchange API
//!
//! This module provides methods for modifying existing orders,
//! supporting both single and batch modifications.

use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::auth::{Signature, Wallet};
use crate::client::Client;
use crate::error::{Error, Result};
use crate::types::{
    LimitOrderType, OrderResponse, OrderTypeSpec, OrderWire, TimeInForce, TriggerOrderType,
    TriggerType,
};

/// Individual modify request for the batch modify action
#[derive(Debug, Clone, Serialize)]
pub struct ModifyWire {
    pub oid: u64,
    pub order: OrderWire,
}

/// Modify action for the exchange API
#[derive(Debug, Clone, Serialize)]
pub struct ModifyAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub oid: u64,
    pub order: OrderWire,
}

/// Batch modify action for the exchange API
#[derive(Debug, Clone, Serialize)]
pub struct BatchModifyAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub modifies: Vec<ModifyWire>,
}

/// Exchange request wrapper with authentication for modify operations
#[derive(Debug, Clone, Serialize)]
pub struct ModifyExchangeRequest<T> {
    pub action: T,
    pub nonce: u64,
    pub signature: Signature,
    #[serde(rename = "vaultAddress", skip_serializing_if = "Option::is_none")]
    pub vault_address: Option<String>,
}

/// Exchange API response wrapper for modify operations
#[derive(Debug, Clone, Deserialize)]
pub struct ModifyExchangeApiResponse {
    pub status: String,
    pub response: Option<OrderResponse>,
}

/// Builder for constructing order modifications
#[derive(Debug, Clone)]
pub struct ModifyOrderBuilder {
    oid: u64,
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
    time_in_force: TimeInForce,
    reduce_only: bool,
    client_order_id: Option<String>,
}

impl ModifyOrderBuilder {
    /// Create a new modify order builder
    ///
    /// # Arguments
    /// * `oid` - The order ID of the existing order to modify
    /// * `asset` - The asset index
    /// * `is_buy` - Whether the order is a buy order
    /// * `price` - The new price
    /// * `size` - The new size
    pub fn new(
        oid: u64,
        asset: u32,
        is_buy: bool,
        price: impl Into<String>,
        size: impl Into<String>,
    ) -> Self {
        Self {
            oid,
            asset,
            is_buy,
            price: price.into(),
            size: size.into(),
            time_in_force: TimeInForce::Gtc,
            reduce_only: false,
            client_order_id: None,
        }
    }

    /// Set the time in force (default: GTC)
    pub fn time_in_force(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = tif;
        self
    }

    /// Set reduce only flag
    pub fn reduce_only(mut self, reduce_only: bool) -> Self {
        self.reduce_only = reduce_only;
        self
    }

    /// Set client order ID
    pub fn client_order_id(mut self, cloid: impl Into<String>) -> Self {
        self.client_order_id = Some(cloid.into());
        self
    }

    /// Build the modify wire format
    pub fn build(self) -> ModifyWire {
        ModifyWire {
            oid: self.oid,
            order: OrderWire {
                a: self.asset,
                b: self.is_buy,
                p: self.price,
                s: self.size,
                r: self.reduce_only,
                t: OrderTypeSpec::Limit {
                    limit: LimitOrderType {
                        tif: self.time_in_force,
                    },
                },
                c: self.client_order_id,
            },
        }
    }

    /// Get the order ID
    pub fn oid(&self) -> u64 {
        self.oid
    }
}

/// Builder for constructing trigger order modifications
#[derive(Debug, Clone)]
pub struct ModifyTriggerOrderBuilder {
    oid: u64,
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
    trigger_price: String,
    trigger_type: TriggerType,
    is_market: bool,
    reduce_only: bool,
    client_order_id: Option<String>,
}

impl ModifyTriggerOrderBuilder {
    /// Create a new modify trigger order builder
    ///
    /// # Arguments
    /// * `oid` - The order ID of the existing trigger order to modify
    /// * `asset` - The asset index
    /// * `is_buy` - Whether the order is a buy order
    /// * `price` - The new limit price (execution price)
    /// * `size` - The new size
    /// * `trigger_price` - The new trigger price
    /// * `trigger_type` - The trigger type (stop loss or take profit)
    pub fn new(
        oid: u64,
        asset: u32,
        is_buy: bool,
        price: impl Into<String>,
        size: impl Into<String>,
        trigger_price: impl Into<String>,
        trigger_type: TriggerType,
    ) -> Self {
        Self {
            oid,
            asset,
            is_buy,
            price: price.into(),
            size: size.into(),
            trigger_price: trigger_price.into(),
            trigger_type,
            is_market: false,
            reduce_only: false,
            client_order_id: None,
        }
    }

    /// Create a market trigger order (market execution when triggered)
    pub fn market(mut self) -> Self {
        self.is_market = true;
        self
    }

    /// Set reduce only flag
    pub fn reduce_only(mut self, reduce_only: bool) -> Self {
        self.reduce_only = reduce_only;
        self
    }

    /// Set client order ID
    pub fn client_order_id(mut self, cloid: impl Into<String>) -> Self {
        self.client_order_id = Some(cloid.into());
        self
    }

    /// Build the modify wire format
    pub fn build(self) -> ModifyWire {
        ModifyWire {
            oid: self.oid,
            order: OrderWire {
                a: self.asset,
                b: self.is_buy,
                p: self.price,
                s: self.size,
                r: self.reduce_only,
                t: OrderTypeSpec::Trigger {
                    trigger: TriggerOrderType {
                        is_market: self.is_market,
                        trigger_px: self.trigger_price,
                        tpsl: self.trigger_type,
                    },
                },
                c: self.client_order_id,
            },
        }
    }

    /// Get the order ID
    pub fn oid(&self) -> u64 {
        self.oid
    }
}

impl Client {
    /// Modify a single order
    ///
    /// Modifies an existing order's price and/or size. The order type cannot be changed
    /// (e.g., a limit order cannot be changed to a trigger order).
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the modification with
    /// * `modify` - The modification to apply (use `ModifyOrderBuilder` or `ModifyTriggerOrderBuilder`)
    ///
    /// # Returns
    /// The order response containing the status of the modification
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet, ModifyOrderBuilder, TimeInForce};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Modify an existing order's price
    /// let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1")
    ///     .time_in_force(TimeInForce::Gtc)
    ///     .build();
    ///
    /// let response = client.modify_order(&wallet, modify).await?;
    /// ```
    pub async fn modify_order(&self, wallet: &Wallet, modify: ModifyWire) -> Result<OrderResponse> {
        self.modify_order_with_options(wallet, modify, None).await
    }

    /// Modify a single order with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the modification with
    /// * `modify` - The modification to apply
    /// * `vault_address` - Optional vault address to modify on behalf of
    pub async fn modify_order_with_options(
        &self,
        wallet: &Wallet,
        modify: ModifyWire,
        vault_address: Option<Address>,
    ) -> Result<OrderResponse> {
        // Create the modify action
        let action = ModifyAction {
            action_type: "modify".to_string(),
            oid: modify.oid,
            order: modify.order,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = ModifyExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: ModifyExchangeApiResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Order modification failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Modify multiple orders in a single request
    ///
    /// Modifies multiple orders atomically. This is more efficient than modifying
    /// orders individually.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the modifications with
    /// * `modifies` - The modifications to apply
    ///
    /// # Returns
    /// The order response containing statuses for all modifications
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet, ModifyOrderBuilder};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// let modifies = vec![
    ///     ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build(),
    ///     ModifyOrderBuilder::new(12346, 0, true, "50500.0", "0.1").build(),
    /// ];
    ///
    /// let response = client.modify_batch_orders(&wallet, modifies).await?;
    /// for status in &response.data.statuses {
    ///     println!("{:?}", status);
    /// }
    /// ```
    pub async fn modify_batch_orders(
        &self,
        wallet: &Wallet,
        modifies: Vec<ModifyWire>,
    ) -> Result<OrderResponse> {
        self.modify_batch_orders_with_options(wallet, modifies, None)
            .await
    }

    /// Modify multiple orders with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the modifications with
    /// * `modifies` - The modifications to apply
    /// * `vault_address` - Optional vault address to modify on behalf of
    pub async fn modify_batch_orders_with_options(
        &self,
        wallet: &Wallet,
        modifies: Vec<ModifyWire>,
        vault_address: Option<Address>,
    ) -> Result<OrderResponse> {
        if modifies.is_empty() {
            return Err(Error::InvalidParameter(
                "Modifies list cannot be empty".to_string(),
            ));
        }

        // Create the batch modify action
        let action = BatchModifyAction {
            action_type: "batchModify".to_string(),
            modifies,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = ModifyExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: ModifyExchangeApiResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Order modification failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OrderPlacementStatus;
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

        async fn modify_order(&self, wallet: &Wallet, modify: ModifyWire) -> Result<OrderResponse> {
            let action = ModifyAction {
                action_type: "modify".to_string(),
                oid: modify.oid,
                order: modify.order,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = ModifyExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: None,
            };

            let response: ModifyExchangeApiResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Order modification failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn modify_batch_orders(
            &self,
            wallet: &Wallet,
            modifies: Vec<ModifyWire>,
        ) -> Result<OrderResponse> {
            if modifies.is_empty() {
                return Err(Error::InvalidParameter(
                    "Modifies list cannot be empty".to_string(),
                ));
            }

            let action = BatchModifyAction {
                action_type: "batchModify".to_string(),
                modifies,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = ModifyExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: None,
            };

            let response: ModifyExchangeApiResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Order modification failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }
    }

    #[test]
    fn test_modify_order_builder() {
        let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1")
            .time_in_force(TimeInForce::Gtc)
            .reduce_only(false)
            .client_order_id("modified-order-1")
            .build();

        assert_eq!(modify.oid, 12345);
        assert_eq!(modify.order.a, 0);
        assert!(modify.order.b);
        assert_eq!(modify.order.p, "51000.0");
        assert_eq!(modify.order.s, "0.1");
        assert!(!modify.order.r);
        assert_eq!(modify.order.c, Some("modified-order-1".to_string()));

        match modify.order.t {
            OrderTypeSpec::Limit { limit } => {
                assert_eq!(limit.tif, TimeInForce::Gtc);
            }
            _ => panic!("Expected limit order"),
        }
    }

    #[test]
    fn test_modify_order_builder_ioc() {
        let modify = ModifyOrderBuilder::new(12345, 1, false, "3000.0", "1.5")
            .time_in_force(TimeInForce::Ioc)
            .build();

        assert_eq!(modify.oid, 12345);
        assert_eq!(modify.order.a, 1);
        assert!(!modify.order.b);
        assert_eq!(modify.order.p, "3000.0");
        assert_eq!(modify.order.s, "1.5");

        match modify.order.t {
            OrderTypeSpec::Limit { limit } => {
                assert_eq!(limit.tif, TimeInForce::Ioc);
            }
            _ => panic!("Expected limit order"),
        }
    }

    #[test]
    fn test_modify_trigger_order_builder() {
        let modify =
            ModifyTriggerOrderBuilder::new(12345, 0, false, "48000.0", "0.1", "49000.0", TriggerType::Sl)
                .reduce_only(true)
                .build();

        assert_eq!(modify.oid, 12345);
        assert_eq!(modify.order.a, 0);
        assert!(!modify.order.b);
        assert_eq!(modify.order.p, "48000.0");
        assert_eq!(modify.order.s, "0.1");
        assert!(modify.order.r);

        match modify.order.t {
            OrderTypeSpec::Trigger { trigger } => {
                assert!(!trigger.is_market);
                assert_eq!(trigger.trigger_px, "49000.0");
                assert_eq!(trigger.tpsl, TriggerType::Sl);
            }
            _ => panic!("Expected trigger order"),
        }
    }

    #[test]
    fn test_modify_trigger_order_builder_market() {
        let modify =
            ModifyTriggerOrderBuilder::new(12345, 0, false, "52000.0", "0.1", "51000.0", TriggerType::Tp)
                .market()
                .client_order_id("tp-modified-1")
                .build();

        assert_eq!(modify.order.c, Some("tp-modified-1".to_string()));

        match modify.order.t {
            OrderTypeSpec::Trigger { trigger } => {
                assert!(trigger.is_market);
                assert_eq!(trigger.trigger_px, "51000.0");
                assert_eq!(trigger.tpsl, TriggerType::Tp);
            }
            _ => panic!("Expected trigger order"),
        }
    }

    #[test]
    fn test_modify_wire_serialization() {
        let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1")
            .time_in_force(TimeInForce::Gtc)
            .build();

        let json = serde_json::to_string(&modify).unwrap();
        assert!(json.contains("\"oid\":12345"));
        assert!(json.contains("\"a\":0"));
        assert!(json.contains("\"b\":true"));
        assert!(json.contains("\"p\":\"51000.0\""));
        assert!(json.contains("\"s\":\"0.1\""));
        assert!(json.contains("\"tif\":\"Gtc\""));
    }

    #[test]
    fn test_modify_action_serialization() {
        let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build();

        let action = ModifyAction {
            action_type: "modify".to_string(),
            oid: modify.oid,
            order: modify.order,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"modify\""));
        assert!(json.contains("\"oid\":12345"));
    }

    #[test]
    fn test_batch_modify_action_serialization() {
        let modifies = vec![
            ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build(),
            ModifyOrderBuilder::new(12346, 0, true, "50500.0", "0.2").build(),
        ];

        let action = BatchModifyAction {
            action_type: "batchModify".to_string(),
            modifies,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"batchModify\""));
        assert!(json.contains("\"modifies\":["));
        assert!(json.contains("\"oid\":12345"));
        assert!(json.contains("\"oid\":12346"));
    }

    #[test]
    fn test_modify_exchange_request_serialization() {
        let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build();

        let action = ModifyAction {
            action_type: "modify".to_string(),
            oid: modify.oid,
            order: modify.order,
        };

        let request = ModifyExchangeRequest {
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
    fn test_modify_exchange_request_with_vault() {
        let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build();

        let action = ModifyAction {
            action_type: "modify".to_string(),
            oid: modify.oid,
            order: modify.order,
        };

        let request = ModifyExchangeRequest {
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

    #[tokio::test]
    async fn test_modify_order_success() {
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
                        "type": "order",
                        "data": {
                            "statuses": [{"resting": {"oid": 12345}}]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build();

        let response = client.modify_order(&wallet, modify).await.unwrap();

        assert_eq!(response.response_type, "order");
        assert_eq!(response.data.statuses.len(), 1);

        match &response.data.statuses[0] {
            OrderPlacementStatus::Resting { resting } => {
                assert_eq!(resting.oid, 12345);
            }
            _ => panic!("Expected resting order status"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_modify_order_filled() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "order",
                        "data": {
                            "statuses": [{
                                "filled": {
                                    "totalSz": "0.1",
                                    "avgPx": "51000.0",
                                    "oid": 12345
                                }
                            }]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1")
            .time_in_force(TimeInForce::Ioc)
            .build();

        let response = client.modify_order(&wallet, modify).await.unwrap();

        match &response.data.statuses[0] {
            OrderPlacementStatus::Filled { filled } => {
                assert_eq!(filled.total_sz, "0.1");
                assert_eq!(filled.avg_px, "51000.0");
                assert_eq!(filled.oid, 12345);
            }
            _ => panic!("Expected filled order status"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_modify_order_error_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "order",
                        "data": {
                            "statuses": [{"error": "Order not found"}]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let modify = ModifyOrderBuilder::new(99999, 0, true, "51000.0", "0.1").build();

        let response = client.modify_order(&wallet, modify).await.unwrap();

        match &response.data.statuses[0] {
            OrderPlacementStatus::Error { error } => {
                assert_eq!(error, "Order not found");
            }
            _ => panic!("Expected error status"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_modify_batch_orders_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "order",
                        "data": {
                            "statuses": [
                                {"resting": {"oid": 12345}},
                                {"resting": {"oid": 12346}},
                                {"resting": {"oid": 12347}}
                            ]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let modifies = vec![
            ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build(),
            ModifyOrderBuilder::new(12346, 0, true, "50500.0", "0.1").build(),
            ModifyOrderBuilder::new(12347, 0, true, "50000.0", "0.1").build(),
        ];

        let response = client.modify_batch_orders(&wallet, modifies).await.unwrap();

        assert_eq!(response.data.statuses.len(), 3);

        for (i, status) in response.data.statuses.iter().enumerate() {
            match status {
                OrderPlacementStatus::Resting { resting } => {
                    assert_eq!(resting.oid, 12345 + i as u64);
                }
                _ => panic!("Expected resting order status"),
            }
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_modify_batch_orders_partial_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "order",
                        "data": {
                            "statuses": [
                                {"resting": {"oid": 12345}},
                                {"error": "Order not found"},
                                {"filled": {"totalSz": "0.1", "avgPx": "50000.0", "oid": 12347}}
                            ]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let modifies = vec![
            ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build(),
            ModifyOrderBuilder::new(99999, 0, true, "50500.0", "0.1").build(),
            ModifyOrderBuilder::new(12347, 0, true, "50000.0", "0.1")
                .time_in_force(TimeInForce::Ioc)
                .build(),
        ];

        let response = client.modify_batch_orders(&wallet, modifies).await.unwrap();

        assert_eq!(response.data.statuses.len(), 3);

        // First order: resting
        assert!(matches!(
            &response.data.statuses[0],
            OrderPlacementStatus::Resting { .. }
        ));

        // Second order: error
        match &response.data.statuses[1] {
            OrderPlacementStatus::Error { error } => {
                assert!(error.contains("Order not found"));
            }
            _ => panic!("Expected error status"),
        }

        // Third order: filled
        assert!(matches!(
            &response.data.statuses[2],
            OrderPlacementStatus::Filled { .. }
        ));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_modify_batch_orders_empty_error() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.modify_batch_orders(&wallet, vec![]).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[tokio::test]
    async fn test_modify_order_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build();

        let result = client.modify_order(&wallet, modify).await;

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
    async fn test_modify_order_unauthorized() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(401)
            .with_body("Unauthorized: invalid signature")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build();

        let result = client.modify_order(&wallet, modify).await;

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
    async fn test_modify_order_rate_limited() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(429)
            .with_body("Rate limit exceeded")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build();

        let result = client.modify_order(&wallet, modify).await;

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
    async fn test_modify_order_api_error_status() {
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
        let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build();

        let result = client.modify_order(&wallet, modify).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Order modification failed"));
            }
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_modify_trigger_order() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "order",
                        "data": {
                            "statuses": [{"resting": {"oid": 12345}}]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let modify =
            ModifyTriggerOrderBuilder::new(12345, 0, false, "48000.0", "0.1", "49000.0", TriggerType::Sl)
                .market()
                .reduce_only(true)
                .build();

        let response = client.modify_order(&wallet, modify).await.unwrap();

        assert_eq!(response.response_type, "order");
        assert!(matches!(
            &response.data.statuses[0],
            OrderPlacementStatus::Resting { .. }
        ));

        mock.assert_async().await;
    }

    #[test]
    fn test_wallet_signing_produces_signature_for_modify() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let modify = ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build();

            let action = ModifyAction {
                action_type: "modify".to_string(),
                oid: modify.oid,
                order: modify.order,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_produces_signature_for_batch_modify() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let modifies = vec![
                ModifyOrderBuilder::new(12345, 0, true, "51000.0", "0.1").build(),
                ModifyOrderBuilder::new(12346, 0, true, "50500.0", "0.2").build(),
            ];

            let action = BatchModifyAction {
                action_type: "batchModify".to_string(),
                modifies,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }
}
