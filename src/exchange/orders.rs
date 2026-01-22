//! Order placement endpoints for the Hyperliquid Exchange API
//!
//! This module provides methods for placing single and batch orders,
//! supporting limit, market, and trigger order types with all TimeInForce options.

use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::auth::{Signature, Wallet};
use crate::client::Client;
use crate::error::{Error, Result};
use crate::types::{
    BuilderFee, LimitOrderType, OrderAction, OrderGrouping, OrderPlacementStatus, OrderResponse,
    OrderTypeSpec, OrderWire, TimeInForce, TriggerOrderType, TriggerType,
};

/// Exchange request wrapper with authentication
#[derive(Debug, Clone, Serialize)]
pub struct ExchangeRequest<T> {
    pub action: T,
    pub nonce: u64,
    pub signature: Signature,
    #[serde(rename = "vaultAddress", skip_serializing_if = "Option::is_none")]
    pub vault_address: Option<String>,
}

/// Exchange API response wrapper
#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeApiResponse {
    pub status: String,
    pub response: Option<OrderResponse>,
}

/// Builder for constructing limit orders
#[derive(Debug, Clone)]
pub struct LimitOrderBuilder {
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
    time_in_force: TimeInForce,
    reduce_only: bool,
    client_order_id: Option<String>,
}

impl LimitOrderBuilder {
    /// Create a new limit order builder
    pub fn new(
        asset: u32,
        is_buy: bool,
        price: impl Into<String>,
        size: impl Into<String>,
    ) -> Self {
        Self {
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

    /// Build the order wire format
    pub fn build(self) -> OrderWire {
        OrderWire {
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
        }
    }
}

/// Builder for constructing trigger orders (stop loss / take profit)
#[derive(Debug, Clone)]
pub struct TriggerOrderBuilder {
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

impl TriggerOrderBuilder {
    /// Create a new trigger order builder
    pub fn new(
        asset: u32,
        is_buy: bool,
        price: impl Into<String>,
        size: impl Into<String>,
        trigger_price: impl Into<String>,
        trigger_type: TriggerType,
    ) -> Self {
        Self {
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

    /// Build the order wire format
    pub fn build(self) -> OrderWire {
        OrderWire {
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
        }
    }
}

impl Client {
    /// Place a single order
    ///
    /// Submits a single order to the exchange. The order can be a limit order,
    /// market order (via IOC time-in-force), or trigger order (stop loss/take profit).
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the order with
    /// * `order` - The order to place (use `LimitOrderBuilder` or `TriggerOrderBuilder`)
    /// * `grouping` - Order grouping for TP/SL pairs
    ///
    /// # Returns
    /// The order response containing the placement status (resting, filled, or error)
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet, LimitOrderBuilder, TimeInForce, OrderGrouping};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Place a limit order
    /// let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1")
    ///     .time_in_force(TimeInForce::Gtc)
    ///     .build();
    ///
    /// let response = client.place_order(&wallet, order, OrderGrouping::Na).await?;
    /// ```
    pub async fn place_order(
        &self,
        wallet: &Wallet,
        order: OrderWire,
        grouping: OrderGrouping,
    ) -> Result<OrderResponse> {
        self.place_order_with_options(wallet, order, grouping, None, None)
            .await
    }

    /// Place a single order with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the order with
    /// * `order` - The order to place
    /// * `grouping` - Order grouping for TP/SL pairs
    /// * `builder` - Optional builder fee configuration
    /// * `vault_address` - Optional vault address to trade on behalf of
    pub async fn place_order_with_options(
        &self,
        wallet: &Wallet,
        order: OrderWire,
        grouping: OrderGrouping,
        builder: Option<BuilderFee>,
        vault_address: Option<Address>,
    ) -> Result<OrderResponse> {
        self.place_batch_orders_with_options(wallet, vec![order], grouping, builder, vault_address)
            .await
    }

    /// Place multiple orders in a single request
    ///
    /// Submits multiple orders atomically. All orders share the same grouping setting.
    /// This is more efficient than placing orders individually.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the orders with
    /// * `orders` - The orders to place
    /// * `grouping` - Order grouping for TP/SL pairs
    ///
    /// # Returns
    /// The order response containing placement statuses for all orders
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet, LimitOrderBuilder, OrderGrouping};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// let orders = vec![
    ///     LimitOrderBuilder::new(0, true, "49000.0", "0.1").build(),
    ///     LimitOrderBuilder::new(0, true, "48000.0", "0.1").build(),
    /// ];
    ///
    /// let response = client.place_batch_orders(&wallet, orders, OrderGrouping::Na).await?;
    /// for status in &response.data.statuses {
    ///     println!("{:?}", status);
    /// }
    /// ```
    pub async fn place_batch_orders(
        &self,
        wallet: &Wallet,
        orders: Vec<OrderWire>,
        grouping: OrderGrouping,
    ) -> Result<OrderResponse> {
        self.place_batch_orders_with_options(wallet, orders, grouping, None, None)
            .await
    }

    /// Place multiple orders with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the orders with
    /// * `orders` - The orders to place
    /// * `grouping` - Order grouping for TP/SL pairs
    /// * `builder` - Optional builder fee configuration
    /// * `vault_address` - Optional vault address to trade on behalf of
    pub async fn place_batch_orders_with_options(
        &self,
        wallet: &Wallet,
        orders: Vec<OrderWire>,
        grouping: OrderGrouping,
        builder: Option<BuilderFee>,
        vault_address: Option<Address>,
    ) -> Result<OrderResponse> {
        if orders.is_empty() {
            return Err(Error::InvalidParameter(
                "Orders list cannot be empty".to_string(),
            ));
        }

        // Create the order action
        let action = OrderAction {
            action_type: "order".to_string(),
            orders,
            grouping,
            builder,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = ExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: ExchangeApiResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Order placement failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }
}

/// Check if an order status indicates success (resting or filled)
pub fn is_order_successful(status: &OrderPlacementStatus) -> bool {
    matches!(
        status,
        OrderPlacementStatus::Resting { .. } | OrderPlacementStatus::Filled { .. }
    )
}

/// Extract order ID from a successful order placement
pub fn get_order_id(status: &OrderPlacementStatus) -> Option<u64> {
    match status {
        OrderPlacementStatus::Resting { resting } => Some(resting.oid),
        OrderPlacementStatus::Filled { filled } => Some(filled.oid),
        OrderPlacementStatus::Error { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RestingOrderStatus;
    use mockito::{Matcher, Server};
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

        async fn place_order(
            &self,
            wallet: &Wallet,
            order: OrderWire,
            grouping: OrderGrouping,
        ) -> Result<OrderResponse> {
            self.place_batch_orders(wallet, vec![order], grouping).await
        }

        async fn place_batch_orders(
            &self,
            wallet: &Wallet,
            orders: Vec<OrderWire>,
            grouping: OrderGrouping,
        ) -> Result<OrderResponse> {
            if orders.is_empty() {
                return Err(Error::InvalidParameter(
                    "Orders list cannot be empty".to_string(),
                ));
            }

            let action = OrderAction {
                action_type: "order".to_string(),
                orders,
                grouping,
                builder: None,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = ExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: None,
            };

            let response: ExchangeApiResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Order placement failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }
    }

    #[test]
    fn test_limit_order_builder() {
        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1")
            .time_in_force(TimeInForce::Gtc)
            .reduce_only(false)
            .client_order_id("my-order-1")
            .build();

        assert_eq!(order.a, 0);
        assert!(order.b);
        assert_eq!(order.p, "50000.0");
        assert_eq!(order.s, "0.1");
        assert!(!order.r);
        assert_eq!(order.c, Some("my-order-1".to_string()));

        match order.t {
            OrderTypeSpec::Limit { limit } => {
                assert_eq!(limit.tif, TimeInForce::Gtc);
            }
            _ => panic!("Expected limit order"),
        }
    }

    #[test]
    fn test_limit_order_builder_ioc() {
        let order = LimitOrderBuilder::new(1, false, "3000.0", "1.5")
            .time_in_force(TimeInForce::Ioc)
            .build();

        assert_eq!(order.a, 1);
        assert!(!order.b);
        assert_eq!(order.p, "3000.0");
        assert_eq!(order.s, "1.5");

        match order.t {
            OrderTypeSpec::Limit { limit } => {
                assert_eq!(limit.tif, TimeInForce::Ioc);
            }
            _ => panic!("Expected limit order"),
        }
    }

    #[test]
    fn test_limit_order_builder_alo() {
        let order = LimitOrderBuilder::new(2, true, "100.0", "10.0")
            .time_in_force(TimeInForce::Alo)
            .build();

        match order.t {
            OrderTypeSpec::Limit { limit } => {
                assert_eq!(limit.tif, TimeInForce::Alo);
            }
            _ => panic!("Expected limit order"),
        }
    }

    #[test]
    fn test_trigger_order_builder_stop_loss() {
        let order =
            TriggerOrderBuilder::new(0, false, "48000.0", "0.1", "49000.0", TriggerType::Sl)
                .reduce_only(true)
                .build();

        assert_eq!(order.a, 0);
        assert!(!order.b);
        assert_eq!(order.p, "48000.0");
        assert_eq!(order.s, "0.1");
        assert!(order.r);

        match order.t {
            OrderTypeSpec::Trigger { trigger } => {
                assert!(!trigger.is_market);
                assert_eq!(trigger.trigger_px, "49000.0");
                assert_eq!(trigger.tpsl, TriggerType::Sl);
            }
            _ => panic!("Expected trigger order"),
        }
    }

    #[test]
    fn test_trigger_order_builder_take_profit_market() {
        let order =
            TriggerOrderBuilder::new(0, false, "52000.0", "0.1", "51000.0", TriggerType::Tp)
                .market()
                .client_order_id("tp-1")
                .build();

        assert_eq!(order.c, Some("tp-1".to_string()));

        match order.t {
            OrderTypeSpec::Trigger { trigger } => {
                assert!(trigger.is_market);
                assert_eq!(trigger.trigger_px, "51000.0");
                assert_eq!(trigger.tpsl, TriggerType::Tp);
            }
            _ => panic!("Expected trigger order"),
        }
    }

    #[test]
    fn test_order_wire_serialization() {
        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1")
            .time_in_force(TimeInForce::Gtc)
            .build();

        let json = serde_json::to_string(&order).unwrap();
        assert!(json.contains("\"a\":0"));
        assert!(json.contains("\"b\":true"));
        assert!(json.contains("\"p\":\"50000.0\""));
        assert!(json.contains("\"s\":\"0.1\""));
        assert!(json.contains("\"r\":false"));
        assert!(json.contains("\"tif\":\"Gtc\""));
    }

    #[test]
    fn test_trigger_order_wire_serialization() {
        let order =
            TriggerOrderBuilder::new(0, false, "48000.0", "0.1", "49000.0", TriggerType::Sl)
                .market()
                .build();

        let json = serde_json::to_string(&order).unwrap();
        assert!(json.contains("\"isMarket\":true"));
        assert!(json.contains("\"triggerPx\":\"49000.0\""));
        assert!(json.contains("\"tpsl\":\"sl\""));
    }

    #[test]
    fn test_order_action_serialization() {
        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1").build();

        let action = OrderAction {
            action_type: "order".to_string(),
            orders: vec![order],
            grouping: OrderGrouping::Na,
            builder: None,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"order\""));
        assert!(json.contains("\"grouping\":\"na\""));
        assert!(json.contains("\"orders\":["));
    }

    #[test]
    fn test_order_action_with_builder_fee() {
        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1").build();

        let action = OrderAction {
            action_type: "order".to_string(),
            orders: vec![order],
            grouping: OrderGrouping::Na,
            builder: Some(BuilderFee {
                b: "0x1234567890123456789012345678901234567890".to_string(),
                f: 100,
            }),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"builder\":"));
        assert!(json.contains("\"b\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"f\":100"));
    }

    #[test]
    fn test_exchange_request_serialization() {
        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1").build();

        let action = OrderAction {
            action_type: "order".to_string(),
            orders: vec![order],
            grouping: OrderGrouping::Na,
            builder: None,
        };

        let request = ExchangeRequest {
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
        assert!(json.contains("\"r\":\"0x1234\""));
        assert!(json.contains("\"s\":\"0x5678\""));
        assert!(json.contains("\"v\":27"));
        assert!(!json.contains("\"vaultAddress\""));
    }

    #[test]
    fn test_exchange_request_with_vault() {
        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1").build();

        let action = OrderAction {
            action_type: "order".to_string(),
            orders: vec![order],
            grouping: OrderGrouping::Na,
            builder: None,
        };

        let request = ExchangeRequest {
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
    fn test_is_order_successful() {
        let resting = OrderPlacementStatus::Resting {
            resting: RestingOrderStatus { oid: 12345 },
        };
        assert!(is_order_successful(&resting));

        let filled = OrderPlacementStatus::Filled {
            filled: crate::types::FilledOrderStatus {
                total_sz: "0.1".to_string(),
                avg_px: "50000.0".to_string(),
                oid: 12345,
            },
        };
        assert!(is_order_successful(&filled));

        let error = OrderPlacementStatus::Error {
            error: "Insufficient margin".to_string(),
        };
        assert!(!is_order_successful(&error));
    }

    #[test]
    fn test_get_order_id() {
        let resting = OrderPlacementStatus::Resting {
            resting: RestingOrderStatus { oid: 12345 },
        };
        assert_eq!(get_order_id(&resting), Some(12345));

        let filled = OrderPlacementStatus::Filled {
            filled: crate::types::FilledOrderStatus {
                total_sz: "0.1".to_string(),
                avg_px: "50000.0".to_string(),
                oid: 67890,
            },
        };
        assert_eq!(get_order_id(&filled), Some(67890));

        let error = OrderPlacementStatus::Error {
            error: "Insufficient margin".to_string(),
        };
        assert_eq!(get_order_id(&error), None);
    }

    #[tokio::test]
    async fn test_place_order_success_resting() {
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
        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1").build();

        let response = client
            .place_order(&wallet, order, OrderGrouping::Na)
            .await
            .unwrap();

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
    async fn test_place_order_success_filled() {
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
                                    "avgPx": "50000.0",
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
        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1")
            .time_in_force(TimeInForce::Ioc)
            .build();

        let response = client
            .place_order(&wallet, order, OrderGrouping::Na)
            .await
            .unwrap();

        match &response.data.statuses[0] {
            OrderPlacementStatus::Filled { filled } => {
                assert_eq!(filled.total_sz, "0.1");
                assert_eq!(filled.avg_px, "50000.0");
                assert_eq!(filled.oid, 12345);
            }
            _ => panic!("Expected filled order status"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_place_order_error_response() {
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
                            "statuses": [{"error": "Insufficient margin"}]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let order = LimitOrderBuilder::new(0, true, "50000.0", "100.0").build();

        let response = client
            .place_order(&wallet, order, OrderGrouping::Na)
            .await
            .unwrap();

        match &response.data.statuses[0] {
            OrderPlacementStatus::Error { error } => {
                assert_eq!(error, "Insufficient margin");
            }
            _ => panic!("Expected error status"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_place_batch_orders_success() {
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

        let orders = vec![
            LimitOrderBuilder::new(0, true, "49000.0", "0.1").build(),
            LimitOrderBuilder::new(0, true, "48000.0", "0.1").build(),
            LimitOrderBuilder::new(0, true, "47000.0", "0.1").build(),
        ];

        let response = client
            .place_batch_orders(&wallet, orders, OrderGrouping::Na)
            .await
            .unwrap();

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
    async fn test_place_batch_orders_partial_success() {
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
                                {"error": "Price too far from oracle"},
                                {"filled": {"totalSz": "0.1", "avgPx": "47000.0", "oid": 12347}}
                            ]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let orders = vec![
            LimitOrderBuilder::new(0, true, "49000.0", "0.1").build(),
            LimitOrderBuilder::new(0, true, "10000.0", "0.1").build(), // Bad price
            LimitOrderBuilder::new(0, true, "47000.0", "0.1")
                .time_in_force(TimeInForce::Ioc)
                .build(),
        ];

        let response = client
            .place_batch_orders(&wallet, orders, OrderGrouping::Na)
            .await
            .unwrap();

        assert_eq!(response.data.statuses.len(), 3);

        // First order: resting
        assert!(matches!(
            &response.data.statuses[0],
            OrderPlacementStatus::Resting { .. }
        ));

        // Second order: error
        match &response.data.statuses[1] {
            OrderPlacementStatus::Error { error } => {
                assert!(error.contains("Price too far from oracle"));
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
    async fn test_place_batch_orders_empty_error() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client
            .place_batch_orders(&wallet, vec![], OrderGrouping::Na)
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
    async fn test_place_order_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1").build();

        let result = client.place_order(&wallet, order, OrderGrouping::Na).await;

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
    async fn test_place_order_unauthorized() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(401)
            .with_body("Unauthorized: invalid signature")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1").build();

        let result = client.place_order(&wallet, order, OrderGrouping::Na).await;

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
    async fn test_place_order_rate_limited() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(429)
            .with_body("Rate limit exceeded")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1").build();

        let result = client.place_order(&wallet, order, OrderGrouping::Na).await;

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
    async fn test_place_order_api_error_status() {
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
        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1").build();

        let result = client.place_order(&wallet, order, OrderGrouping::Na).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Order placement failed"));
            }
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_place_trigger_order() {
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

        let order =
            TriggerOrderBuilder::new(0, false, "48000.0", "0.1", "49000.0", TriggerType::Sl)
                .market()
                .reduce_only(true)
                .build();

        let response = client
            .place_order(&wallet, order, OrderGrouping::Na)
            .await
            .unwrap();

        assert_eq!(response.response_type, "order");
        assert!(matches!(
            &response.data.statuses[0],
            OrderPlacementStatus::Resting { .. }
        ));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_place_order_with_tpsl_grouping() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .match_body(Matcher::PartialJson(serde_json::json!({
                "action": {
                    "grouping": "normalTpsl"
                }
            })))
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
                                {"resting": {"oid": 12346}}
                            ]
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        // Entry order + stop loss pair
        let orders = vec![
            LimitOrderBuilder::new(0, true, "50000.0", "0.1").build(),
            TriggerOrderBuilder::new(0, false, "48000.0", "0.1", "49000.0", TriggerType::Sl)
                .build(),
        ];

        let response = client
            .place_batch_orders(&wallet, orders, OrderGrouping::NormalTpsl)
            .await
            .unwrap();

        assert_eq!(response.data.statuses.len(), 2);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_place_order_with_cloid() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .match_body(Matcher::PartialJson(serde_json::json!({
                "action": {
                    "orders": [{
                        "c": "my-custom-order-id"
                    }]
                }
            })))
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

        let order = LimitOrderBuilder::new(0, true, "50000.0", "0.1")
            .client_order_id("my-custom-order-id")
            .build();

        let response = client
            .place_order(&wallet, order, OrderGrouping::Na)
            .await
            .unwrap();

        assert!(matches!(
            &response.data.statuses[0],
            OrderPlacementStatus::Resting { .. }
        ));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_place_order_reduce_only() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .match_body(Matcher::PartialJson(serde_json::json!({
                "action": {
                    "orders": [{
                        "r": true
                    }]
                }
            })))
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

        let order = LimitOrderBuilder::new(0, false, "50000.0", "0.1")
            .reduce_only(true)
            .build();

        let response = client
            .place_order(&wallet, order, OrderGrouping::Na)
            .await
            .unwrap();

        assert!(matches!(
            &response.data.statuses[0],
            OrderPlacementStatus::Resting { .. }
        ));

        mock.assert_async().await;
    }

    #[test]
    fn test_wallet_signing_produces_signature() {
        // This test verifies that the wallet produces a valid signature structure
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        // Run async test
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = OrderAction {
                action_type: "order".to_string(),
                orders: vec![LimitOrderBuilder::new(0, true, "50000.0", "0.1").build()],
                grouping: OrderGrouping::Na,
                builder: None,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }
}
