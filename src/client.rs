//! HTTP client wrapper for Hyperliquid API

use reqwest::Client as ReqwestClient;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::{Error, Result};

/// Base URLs for Hyperliquid API
pub const MAINNET_URL: &str = "https://api.hyperliquid.xyz";
pub const TESTNET_URL: &str = "https://api.hyperliquid-testnet.xyz";

/// Network configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Network {
    /// Mainnet environment
    #[default]
    Mainnet,
    /// Testnet environment
    Testnet,
}

impl Network {
    /// Get the base URL for this network
    pub fn base_url(&self) -> &'static str {
        match self {
            Network::Mainnet => MAINNET_URL,
            Network::Testnet => TESTNET_URL,
        }
    }
}

/// Hyperliquid API client
#[derive(Debug, Clone)]
pub struct Client {
    http: ReqwestClient,
    network: Network,
}

impl Client {
    /// Create a new client for the specified network
    pub fn new(network: Network) -> Result<Self> {
        let http = ReqwestClient::builder().build().map_err(Error::Http)?;

        Ok(Self { http, network })
    }

    /// Create a new client for mainnet
    pub fn mainnet() -> Result<Self> {
        Self::new(Network::Mainnet)
    }

    /// Create a new client for testnet
    pub fn testnet() -> Result<Self> {
        Self::new(Network::Testnet)
    }

    /// Get the base URL for the current network
    pub fn base_url(&self) -> &str {
        self.network.base_url()
    }

    /// Get the info endpoint URL
    pub fn info_url(&self) -> String {
        format!("{}/info", self.base_url())
    }

    /// Get the exchange endpoint URL
    pub fn exchange_url(&self) -> String {
        format!("{}/exchange", self.base_url())
    }

    /// Get the underlying HTTP client
    pub fn http(&self) -> &ReqwestClient {
        &self.http
    }

    /// Get the current network
    pub fn network(&self) -> Network {
        self.network
    }

    /// Send a POST request to the info endpoint and deserialize the response
    ///
    /// This is used for all read-only queries like market data and user state.
    ///
    /// # Arguments
    /// * `request` - The request body to send (must implement Serialize)
    ///
    /// # Returns
    /// The deserialized response of type `R`
    pub async fn post_info<T, R>(&self, request: &T) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let url = self.info_url();
        self.post(&url, request).await
    }

    /// Send a POST request to the exchange endpoint and deserialize the response
    ///
    /// This is used for authenticated trading actions like placing orders.
    /// Note: The request must include proper authentication (signature, nonce, etc.)
    ///
    /// # Arguments
    /// * `request` - The request body to send (must implement Serialize)
    ///
    /// # Returns
    /// The deserialized response of type `R`
    pub async fn post_exchange<T, R>(&self, request: &T) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let url = self.exchange_url();
        self.post(&url, request).await
    }

    /// Internal helper to send a POST request and handle the response
    async fn post<T, R>(&self, url: &str, request: &T) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let response = self
            .http
            .post(url)
            .json(request)
            .send()
            .await
            .map_err(Error::Http)?;

        let status = response.status();
        let body = response.text().await.map_err(Error::Http)?;

        // Check for HTTP errors
        if !status.is_success() {
            return Err(Error::Api(format!("HTTP {} - {}", status.as_u16(), body)));
        }

        // Try to parse as the expected response type
        serde_json::from_str(&body).map_err(|e| {
            // If parsing fails, check if the API returned an error message
            if let Ok(error_obj) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(error_msg) = error_obj.get("error").and_then(|v| v.as_str()) {
                    return Error::Api(error_msg.to_string());
                }
            }
            Error::Json(e)
        })
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::mainnet().expect("Failed to create default client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_urls() {
        assert_eq!(Network::Mainnet.base_url(), MAINNET_URL);
        assert_eq!(Network::Testnet.base_url(), TESTNET_URL);
    }

    #[test]
    fn test_client_creation() {
        let client = Client::mainnet().unwrap();
        assert_eq!(client.network(), Network::Mainnet);
        assert_eq!(client.base_url(), MAINNET_URL);

        let client = Client::testnet().unwrap();
        assert_eq!(client.network(), Network::Testnet);
        assert_eq!(client.base_url(), TESTNET_URL);
    }

    #[test]
    fn test_endpoint_urls() {
        let client = Client::mainnet().unwrap();
        assert_eq!(client.info_url(), "https://api.hyperliquid.xyz/info");
        assert_eq!(
            client.exchange_url(),
            "https://api.hyperliquid.xyz/exchange"
        );
    }

    #[test]
    fn test_default_network() {
        assert_eq!(Network::default(), Network::Mainnet);
    }

    #[test]
    fn test_client_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<Client>();
        assert_sync::<Client>();
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use mockito::{Matcher, Server};
    use serde::{Deserialize, Serialize};

    /// Test client that can be configured with a custom base URL
    #[derive(Debug, Clone)]
    struct TestClient {
        http: ReqwestClient,
        base_url: String,
    }

    impl TestClient {
        fn new(base_url: &str) -> Result<Self> {
            let http = ReqwestClient::builder().build().map_err(Error::Http)?;
            Ok(Self {
                http,
                base_url: base_url.to_string(),
            })
        }

        fn info_url(&self) -> String {
            format!("{}/info", self.base_url)
        }

        fn exchange_url(&self) -> String {
            format!("{}/exchange", self.base_url)
        }

        async fn post<T, R>(&self, url: &str, request: &T) -> Result<R>
        where
            T: Serialize + ?Sized,
            R: DeserializeOwned,
        {
            let response = self
                .http
                .post(url)
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

        async fn post_info<T, R>(&self, request: &T) -> Result<R>
        where
            T: Serialize + ?Sized,
            R: DeserializeOwned,
        {
            let url = self.info_url();
            self.post(&url, request).await
        }

        async fn post_exchange<T, R>(&self, request: &T) -> Result<R>
        where
            T: Serialize + ?Sized,
            R: DeserializeOwned,
        {
            let url = self.exchange_url();
            self.post(&url, request).await
        }
    }

    #[derive(Debug, Serialize)]
    struct TestInfoRequest {
        #[serde(rename = "type")]
        request_type: String,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestMidsResponse {
        #[serde(rename = "BTC")]
        btc: String,
        #[serde(rename = "ETH")]
        eth: String,
    }

    #[derive(Debug, Serialize)]
    struct TestExchangeRequest {
        action: TestAction,
        nonce: u64,
        signature: TestSignature,
    }

    #[derive(Debug, Serialize)]
    struct TestAction {
        #[serde(rename = "type")]
        action_type: String,
    }

    #[derive(Debug, Serialize)]
    struct TestSignature {
        r: String,
        s: String,
        v: u8,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestExchangeResponse {
        status: String,
    }

    #[tokio::test]
    async fn test_post_info_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({"type": "allMids"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"BTC": "50000.0", "ETH": "3000.0"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url()).unwrap();
        let request = TestInfoRequest {
            request_type: "allMids".to_string(),
        };

        let result: TestMidsResponse = client.post_info(&request).await.unwrap();
        assert_eq!(result.btc, "50000.0");
        assert_eq!(result.eth, "3000.0");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_info_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url()).unwrap();
        let request = TestInfoRequest {
            request_type: "allMids".to_string(),
        };

        let result: Result<TestMidsResponse> = client.post_info(&request).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("500"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_info_api_error_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid request type"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url()).unwrap();
        let request = TestInfoRequest {
            request_type: "invalid".to_string(),
        };

        let result: Result<TestMidsResponse> = client.post_info(&request).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid request type"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_info_malformed_json() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("not valid json")
            .create_async()
            .await;

        let client = TestClient::new(&server.url()).unwrap();
        let request = TestInfoRequest {
            request_type: "allMids".to_string(),
        };

        let result: Result<TestMidsResponse> = client.post_info(&request).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Json(_)));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_exchange_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "ok"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url()).unwrap();
        let request = TestExchangeRequest {
            action: TestAction {
                action_type: "order".to_string(),
            },
            nonce: 1234567890,
            signature: TestSignature {
                r: "0x1234".to_string(),
                s: "0x5678".to_string(),
                v: 27,
            },
        };

        let result: TestExchangeResponse = client.post_exchange(&request).await.unwrap();
        assert_eq!(result.status, "ok");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_exchange_unauthorized() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(401)
            .with_body("Unauthorized: invalid signature")
            .create_async()
            .await;

        let client = TestClient::new(&server.url()).unwrap();
        let request = TestExchangeRequest {
            action: TestAction {
                action_type: "order".to_string(),
            },
            nonce: 1234567890,
            signature: TestSignature {
                r: "0xinvalid".to_string(),
                s: "0xinvalid".to_string(),
                v: 27,
            },
        };

        let result: Result<TestExchangeResponse> = client.post_exchange(&request).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("401"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_exchange_rate_limited() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(429)
            .with_body("Rate limit exceeded")
            .create_async()
            .await;

        let client = TestClient::new(&server.url()).unwrap();
        let request = TestExchangeRequest {
            action: TestAction {
                action_type: "order".to_string(),
            },
            nonce: 1234567890,
            signature: TestSignature {
                r: "0x1234".to_string(),
                s: "0x5678".to_string(),
                v: 27,
            },
        };

        let result: Result<TestExchangeResponse> = client.post_exchange(&request).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("429"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_info_with_complex_response() {
        #[derive(Debug, Deserialize)]
        struct L2BookResponse {
            coin: String,
            levels: (Vec<Level>, Vec<Level>),
        }

        #[derive(Debug, Deserialize)]
        struct Level {
            px: String,
            sz: String,
            n: u32,
        }

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "coin": "BTC",
                    "levels": [
                        [{"px": "50000.0", "sz": "1.0", "n": 5}],
                        [{"px": "49999.0", "sz": "2.0", "n": 10}]
                    ]
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url()).unwrap();

        #[derive(Serialize)]
        struct L2Request {
            #[serde(rename = "type")]
            request_type: String,
            coin: String,
        }

        let request = L2Request {
            request_type: "l2Book".to_string(),
            coin: "BTC".to_string(),
        };

        let result: L2BookResponse = client.post_info(&request).await.unwrap();
        assert_eq!(result.coin, "BTC");
        assert_eq!(result.levels.0.len(), 1);
        assert_eq!(result.levels.0[0].px, "50000.0");
        assert_eq!(result.levels.1[0].px, "49999.0");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_exchange_with_order_response() {
        #[derive(Debug, Deserialize)]
        struct OrderResponse {
            status: String,
            response: OrderResponseData,
        }

        #[derive(Debug, Deserialize)]
        struct OrderResponseData {
            #[serde(rename = "type")]
            response_type: String,
        }

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
                        "data": {"statuses": [{"resting": {"oid": 12345}}]}
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url()).unwrap();
        let request = TestExchangeRequest {
            action: TestAction {
                action_type: "order".to_string(),
            },
            nonce: 1234567890,
            signature: TestSignature {
                r: "0x1234".to_string(),
                s: "0x5678".to_string(),
                v: 27,
            },
        };

        let result: OrderResponse = client.post_exchange(&request).await.unwrap();
        assert_eq!(result.status, "ok");
        assert_eq!(result.response.response_type, "order");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_info_empty_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[]")
            .create_async()
            .await;

        let client = TestClient::new(&server.url()).unwrap();
        let request = TestInfoRequest {
            request_type: "openOrders".to_string(),
        };

        let result: Vec<serde_json::Value> = client.post_info(&request).await.unwrap();
        assert!(result.is_empty());

        mock.assert_async().await;
    }
}
