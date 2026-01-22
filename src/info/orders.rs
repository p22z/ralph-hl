//! User orders info endpoints
//!
//! This module provides methods for querying user order information from the Hyperliquid API.

use crate::client::Client;
use crate::error::Result;
use crate::types::{
    FrontendOpenOrder, FrontendOpenOrdersRequest, HistoricalOrdersRequest, OpenOrder,
    OpenOrdersRequest, OrderId, OrderStatusRequest, OrderStatusResponse,
};

impl Client {
    /// Retrieve open orders for a user
    ///
    /// Returns all currently open orders for the specified user.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    /// * `dex` - Optional DEX identifier. If None, uses the default perp DEX.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let orders = client.open_orders("0x1234...", None).await?;
    /// for order in orders {
    ///     println!("{}: {} {} @ {}", order.coin, order.side, order.sz, order.limit_px);
    /// }
    /// ```
    pub async fn open_orders(&self, user: &str, dex: Option<&str>) -> Result<Vec<OpenOrder>> {
        let mut request = OpenOrdersRequest::new(user);
        if let Some(d) = dex {
            request = request.with_dex(d);
        }
        self.post_info(&request).await
    }

    /// Retrieve open orders with frontend info for a user
    ///
    /// Returns all currently open orders with additional frontend-specific information
    /// such as reduce_only flag, order type, original size, trigger conditions, and client order ID.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    /// * `dex` - Optional DEX identifier. If None, uses the default perp DEX.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let orders = client.frontend_open_orders("0x1234...", None).await?;
    /// for order in orders {
    ///     println!("{}: {} {} @ {} (type: {:?})",
    ///         order.coin, order.side, order.sz, order.limit_px, order.order_type);
    /// }
    /// ```
    pub async fn frontend_open_orders(
        &self,
        user: &str,
        dex: Option<&str>,
    ) -> Result<Vec<FrontendOpenOrder>> {
        let mut request = FrontendOpenOrdersRequest::new(user);
        if let Some(d) = dex {
            request = request.with_dex(d);
        }
        self.post_info(&request).await
    }

    /// Retrieve historical orders for a user
    ///
    /// Returns the order history for the specified user, including filled, canceled,
    /// and other completed orders.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let orders = client.historical_orders("0x1234...").await?;
    /// for order in orders {
    ///     println!("{}: {} (status: {:?})", order.order.coin, order.order.sz, order.status);
    /// }
    /// ```
    pub async fn historical_orders(&self, user: &str) -> Result<Vec<HistoricalOrder>> {
        let request = HistoricalOrdersRequest::new(user);
        self.post_info(&request).await
    }

    /// Query order status by order ID
    ///
    /// Returns the status of a specific order, either by numeric order ID (oid) or
    /// client order ID (cloid).
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    /// * `oid` - The order ID (either numeric `OrderId::Oid` or string `OrderId::Cloid`)
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    ///
    /// // Query by numeric order ID
    /// let status = client.order_status("0x1234...", OrderId::Oid(12345)).await?;
    /// println!("Status: {}", status.status);
    ///
    /// // Query by client order ID
    /// let status = client.order_status("0x1234...", OrderId::Cloid("0xabcd...".to_string())).await?;
    /// ```
    pub async fn order_status(&self, user: &str, oid: OrderId) -> Result<OrderStatusResponse> {
        let request = OrderStatusRequest::new(user, oid);
        self.post_info(&request).await
    }
}

// Re-export HistoricalOrder for use in this module
use crate::types::HistoricalOrder;

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

        async fn open_orders(&self, user: &str, dex: Option<&str>) -> Result<Vec<OpenOrder>> {
            let mut request = OpenOrdersRequest::new(user);
            if let Some(d) = dex {
                request = request.with_dex(d);
            }
            self.post_info(&request).await
        }

        async fn frontend_open_orders(
            &self,
            user: &str,
            dex: Option<&str>,
        ) -> Result<Vec<FrontendOpenOrder>> {
            let mut request = FrontendOpenOrdersRequest::new(user);
            if let Some(d) = dex {
                request = request.with_dex(d);
            }
            self.post_info(&request).await
        }

        async fn historical_orders(&self, user: &str) -> Result<Vec<HistoricalOrder>> {
            let request = HistoricalOrdersRequest::new(user);
            self.post_info(&request).await
        }

        async fn order_status(&self, user: &str, oid: OrderId) -> Result<OrderStatusResponse> {
            let request = OrderStatusRequest::new(user, oid);
            self.post_info(&request).await
        }
    }

    // ==================== Open Orders Tests ====================

    #[tokio::test]
    async fn test_open_orders_without_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "openOrders",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "coin": "BTC",
                        "limitPx": "50000.0",
                        "oid": 12345,
                        "side": "B",
                        "sz": "0.1",
                        "timestamp": 1700000000000
                    },
                    {
                        "coin": "ETH",
                        "limitPx": "3000.0",
                        "oid": 12346,
                        "side": "A",
                        "sz": "1.0",
                        "timestamp": 1700000001000
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let orders = client
            .open_orders("0x1234567890123456789012345678901234567890", None)
            .await
            .unwrap();

        assert_eq!(orders.len(), 2);
        assert_eq!(orders[0].coin, "BTC");
        assert_eq!(orders[0].limit_px, "50000.0");
        assert_eq!(orders[0].oid, 12345);
        assert_eq!(orders[0].side, Side::Buy);
        assert_eq!(orders[0].sz, "0.1");
        assert_eq!(orders[1].coin, "ETH");
        assert_eq!(orders[1].side, Side::Sell);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_open_orders_with_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "openOrders",
                "user": "0x1234567890123456789012345678901234567890",
                "dex": "customdex"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let orders = client
            .open_orders(
                "0x1234567890123456789012345678901234567890",
                Some("customdex"),
            )
            .await
            .unwrap();

        assert_eq!(orders.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_open_orders_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "openOrders",
                "user": "0xdeadbeef12345678901234567890123456789012"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let orders = client
            .open_orders("0xdeadbeef12345678901234567890123456789012", None)
            .await
            .unwrap();

        assert_eq!(orders.len(), 0);

        mock.assert_async().await;
    }

    // ==================== Frontend Open Orders Tests ====================

    #[tokio::test]
    async fn test_frontend_open_orders_without_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "frontendOpenOrders",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "coin": "BTC",
                        "limitPx": "50000.0",
                        "oid": 12345,
                        "side": "B",
                        "sz": "0.1",
                        "timestamp": 1700000000000,
                        "reduceOnly": false,
                        "orderType": "Limit",
                        "origSz": "0.1",
                        "cloid": "0xabc123"
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let orders = client
            .frontend_open_orders("0x1234567890123456789012345678901234567890", None)
            .await
            .unwrap();

        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].coin, "BTC");
        assert_eq!(orders[0].limit_px, "50000.0");
        assert_eq!(orders[0].oid, 12345);
        assert_eq!(orders[0].reduce_only, Some(false));
        assert_eq!(orders[0].order_type, Some("Limit".to_string()));
        assert_eq!(orders[0].orig_sz, Some("0.1".to_string()));
        assert_eq!(orders[0].cloid, Some("0xabc123".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_frontend_open_orders_with_trigger() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "frontendOpenOrders",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "coin": "ETH",
                        "limitPx": "2800.0",
                        "oid": 12347,
                        "side": "B",
                        "sz": "2.0",
                        "timestamp": 1700000000000,
                        "reduceOnly": true,
                        "orderType": "Stop Market",
                        "origSz": "2.0",
                        "triggerPx": "2750.0",
                        "triggerCondition": "tp"
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let orders = client
            .frontend_open_orders("0x1234567890123456789012345678901234567890", None)
            .await
            .unwrap();

        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].coin, "ETH");
        assert_eq!(orders[0].reduce_only, Some(true));
        assert_eq!(orders[0].order_type, Some("Stop Market".to_string()));
        assert_eq!(orders[0].trigger_px, Some("2750.0".to_string()));
        assert_eq!(orders[0].trigger_condition, Some("tp".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_frontend_open_orders_with_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "frontendOpenOrders",
                "user": "0x1234567890123456789012345678901234567890",
                "dex": "testdex"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let orders = client
            .frontend_open_orders(
                "0x1234567890123456789012345678901234567890",
                Some("testdex"),
            )
            .await
            .unwrap();

        assert_eq!(orders.len(), 0);

        mock.assert_async().await;
    }

    // ==================== Historical Orders Tests ====================

    #[tokio::test]
    async fn test_historical_orders() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "historicalOrders",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "order": {
                            "coin": "BTC",
                            "limitPx": "50000.0",
                            "oid": 12345,
                            "side": "B",
                            "sz": "0.1",
                            "timestamp": 1700000000000,
                            "origSz": "0.1"
                        },
                        "status": "filled",
                        "statusTimestamp": 1700000001000
                    },
                    {
                        "order": {
                            "coin": "ETH",
                            "limitPx": "3000.0",
                            "oid": 12346,
                            "side": "A",
                            "sz": "0.0",
                            "timestamp": 1700000002000,
                            "origSz": "1.0"
                        },
                        "status": "canceled",
                        "statusTimestamp": 1700000003000
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let orders = client
            .historical_orders("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(orders.len(), 2);
        assert_eq!(orders[0].order.coin, "BTC");
        assert_eq!(orders[0].order.oid, 12345);
        assert_eq!(orders[0].status, "filled");
        assert_eq!(orders[0].status_timestamp, 1700000001000);
        assert_eq!(orders[1].order.coin, "ETH");
        assert_eq!(orders[1].status, "canceled");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_historical_orders_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "historicalOrders",
                "user": "0xdeadbeef12345678901234567890123456789012"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let orders = client
            .historical_orders("0xdeadbeef12345678901234567890123456789012")
            .await
            .unwrap();

        assert_eq!(orders.len(), 0);

        mock.assert_async().await;
    }

    // ==================== Order Status Tests ====================

    #[tokio::test]
    async fn test_order_status_by_oid() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "orderStatus",
                "user": "0x1234567890123456789012345678901234567890",
                "oid": 12345
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "order",
                    "order": {
                        "order": {
                            "coin": "BTC",
                            "limitPx": "50000.0",
                            "oid": 12345,
                            "side": "B",
                            "sz": "0.1",
                            "timestamp": 1700000000000
                        },
                        "status": "open",
                        "statusTimestamp": 1700000000000
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let status = client
            .order_status(
                "0x1234567890123456789012345678901234567890",
                OrderId::Oid(12345),
            )
            .await
            .unwrap();

        assert_eq!(status.status, "order");
        assert!(status.order.is_some());
        let order_info = status.order.unwrap();
        assert_eq!(order_info.order.coin, "BTC");
        assert_eq!(order_info.order.oid, 12345);
        assert_eq!(order_info.status, crate::types::OrderStatus::Open);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_order_status_by_cloid() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "orderStatus",
                "user": "0x1234567890123456789012345678901234567890",
                "oid": "0xabcdef1234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "order",
                    "order": {
                        "order": {
                            "coin": "ETH",
                            "limitPx": "3000.0",
                            "oid": 12346,
                            "side": "A",
                            "sz": "1.0",
                            "timestamp": 1700000000000
                        },
                        "status": "filled",
                        "statusTimestamp": 1700000001000
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let status = client
            .order_status(
                "0x1234567890123456789012345678901234567890",
                OrderId::Cloid("0xabcdef1234567890".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(status.status, "order");
        assert!(status.order.is_some());
        let order_info = status.order.unwrap();
        assert_eq!(order_info.order.coin, "ETH");
        assert_eq!(order_info.status, crate::types::OrderStatus::Filled);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_order_status_not_found() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "orderStatus",
                "user": "0x1234567890123456789012345678901234567890",
                "oid": 99999
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "unknownOid"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let status = client
            .order_status(
                "0x1234567890123456789012345678901234567890",
                OrderId::Oid(99999),
            )
            .await
            .unwrap();

        assert_eq!(status.status, "unknownOid");
        assert!(status.order.is_none());

        mock.assert_async().await;
    }

    // ==================== Error Handling Tests ====================

    #[tokio::test]
    async fn test_open_orders_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .open_orders("0x1234567890123456789012345678901234567890", None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("500"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_frontend_open_orders_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid user address"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result: Result<Vec<FrontendOpenOrder>> =
            client.frontend_open_orders("invalid-address", None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid user address"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_historical_orders_rate_limited() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(429)
            .with_body("Too Many Requests")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .historical_orders("0x1234567890123456789012345678901234567890")
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("429"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_order_status_unauthorized() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .order_status(
                "0x1234567890123456789012345678901234567890",
                OrderId::Oid(12345),
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("401"));

        mock.assert_async().await;
    }

    // ==================== Request Serialization Tests ====================

    #[tokio::test]
    async fn test_open_orders_request_serialization_without_dex() {
        let request = OpenOrdersRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"openOrders","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_open_orders_request_serialization_with_dex() {
        let request = OpenOrdersRequest::new("0x1234567890123456789012345678901234567890")
            .with_dex("testdex");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"openOrders\""));
        assert!(json.contains("\"user\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"dex\":\"testdex\""));
    }

    #[tokio::test]
    async fn test_frontend_open_orders_request_serialization() {
        let request = FrontendOpenOrdersRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"frontendOpenOrders","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_historical_orders_request_serialization() {
        let request = HistoricalOrdersRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"historicalOrders","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_order_status_request_serialization_with_oid() {
        let request = OrderStatusRequest::new(
            "0x1234567890123456789012345678901234567890",
            OrderId::Oid(12345),
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"orderStatus\""));
        assert!(json.contains("\"user\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"oid\":12345"));
    }

    #[tokio::test]
    async fn test_order_status_request_serialization_with_cloid() {
        let request = OrderStatusRequest::new(
            "0x1234567890123456789012345678901234567890",
            OrderId::Cloid("0xabcdef".to_string()),
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"orderStatus\""));
        assert!(json.contains("\"user\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"oid\":\"0xabcdef\""));
    }
}
