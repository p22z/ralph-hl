//! Schedule cancel (dead man's switch) endpoint for the Hyperliquid Exchange API
//!
//! This module provides the schedule cancel functionality, also known as "dead man's switch".
//! It allows users to set a timestamp at which all their open orders will be automatically
//! cancelled if they haven't sent any other exchange actions.

use std::time::{SystemTime, UNIX_EPOCH};

use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::auth::{Signature, Wallet};
use crate::client::Client;
use crate::error::{Error, Result};
use crate::types::ScheduleCancelAction;

/// Minimum seconds in the future for schedule cancel timestamp
const MIN_SCHEDULE_CANCEL_DELAY_SECONDS: u64 = 5;

/// Exchange request wrapper with authentication for schedule cancel operations
#[derive(Debug, Clone, Serialize)]
pub struct ScheduleCancelExchangeRequest {
    pub action: ScheduleCancelAction,
    pub nonce: u64,
    pub signature: Signature,
    #[serde(rename = "vaultAddress", skip_serializing_if = "Option::is_none")]
    pub vault_address: Option<String>,
}

/// Exchange API response for schedule cancel operations
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleCancelResponse {
    pub status: String,
    pub response: Option<ScheduleCancelResponseData>,
}

/// Response data for schedule cancel operations
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleCancelResponseData {
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: ScheduleCancelResultData,
}

/// Result data for schedule cancel operations
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleCancelResultData {
    #[serde(rename = "scheduledTime")]
    pub scheduled_time: Option<u64>,
}

impl Client {
    /// Schedule automatic cancellation of all open orders at a specific timestamp
    ///
    /// This is also known as a "dead man's switch". If no exchange actions are sent
    /// before the scheduled time, all open orders will be automatically cancelled.
    /// Each new exchange action resets/cancels any previously scheduled cancel.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `timestamp_ms` - Unix timestamp in milliseconds when orders should be cancelled
    ///
    /// # Returns
    /// The schedule cancel response containing the scheduled time if successful
    ///
    /// # Errors
    /// Returns an error if:
    /// - The timestamp is less than 5 seconds in the future
    /// - The wallet signing fails
    /// - The API returns an error
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet};
    /// use std::time::{SystemTime, UNIX_EPOCH};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Schedule cancel 60 seconds from now
    /// let now_ms = SystemTime::now()
    ///     .duration_since(UNIX_EPOCH)
    ///     .unwrap()
    ///     .as_millis() as u64;
    /// let cancel_time = now_ms + 60_000; // 60 seconds from now
    ///
    /// let response = client.schedule_cancel(&wallet, cancel_time).await?;
    /// println!("Scheduled cancel at: {:?}", response.data.scheduled_time);
    /// ```
    pub async fn schedule_cancel(
        &self,
        wallet: &Wallet,
        timestamp_ms: u64,
    ) -> Result<ScheduleCancelResponseData> {
        self.schedule_cancel_with_options(wallet, timestamp_ms, None)
            .await
    }

    /// Schedule automatic cancellation with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `timestamp_ms` - Unix timestamp in milliseconds when orders should be cancelled
    /// * `vault_address` - Optional vault address to schedule cancel on behalf of
    pub async fn schedule_cancel_with_options(
        &self,
        wallet: &Wallet,
        timestamp_ms: u64,
        vault_address: Option<Address>,
    ) -> Result<ScheduleCancelResponseData> {
        // Validate timestamp is at least 5 seconds in the future
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::InvalidParameter(format!("System time error: {}", e)))?
            .as_millis() as u64;

        let min_timestamp = now_ms + (MIN_SCHEDULE_CANCEL_DELAY_SECONDS * 1000);
        if timestamp_ms < min_timestamp {
            return Err(Error::InvalidParameter(format!(
                "Schedule cancel timestamp must be at least {} seconds in the future. \
                 Current time: {} ms, minimum allowed: {} ms, provided: {} ms",
                MIN_SCHEDULE_CANCEL_DELAY_SECONDS, now_ms, min_timestamp, timestamp_ms
            )));
        }

        // Create the schedule cancel action
        let action = ScheduleCancelAction {
            action_type: "scheduleCancel".to_string(),
            time: timestamp_ms,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = ScheduleCancelExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: ScheduleCancelResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Schedule cancel failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }
}

/// Check if a schedule cancel was successful
pub fn is_schedule_cancel_successful(response: &ScheduleCancelResponseData) -> bool {
    response.data.scheduled_time.is_some()
}

/// Get the scheduled time from a successful response
pub fn get_scheduled_time(response: &ScheduleCancelResponseData) -> Option<u64> {
    response.data.scheduled_time
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

        async fn schedule_cancel(
            &self,
            wallet: &Wallet,
            timestamp_ms: u64,
        ) -> Result<ScheduleCancelResponseData> {
            self.schedule_cancel_with_options(wallet, timestamp_ms, None)
                .await
        }

        async fn schedule_cancel_with_options(
            &self,
            wallet: &Wallet,
            timestamp_ms: u64,
            vault_address: Option<Address>,
        ) -> Result<ScheduleCancelResponseData> {
            // Validate timestamp is at least 5 seconds in the future
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| Error::InvalidParameter(format!("System time error: {}", e)))?
                .as_millis() as u64;

            let min_timestamp = now_ms + (MIN_SCHEDULE_CANCEL_DELAY_SECONDS * 1000);
            if timestamp_ms < min_timestamp {
                return Err(Error::InvalidParameter(format!(
                    "Schedule cancel timestamp must be at least {} seconds in the future. \
                     Current time: {} ms, minimum allowed: {} ms, provided: {} ms",
                    MIN_SCHEDULE_CANCEL_DELAY_SECONDS, now_ms, min_timestamp, timestamp_ms
                )));
            }

            let action = ScheduleCancelAction {
                action_type: "scheduleCancel".to_string(),
                time: timestamp_ms,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

            let request = ScheduleCancelExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: vault_address.map(|addr| format!("{addr:?}")),
            };

            let response: ScheduleCancelResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Schedule cancel failed: {}",
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
    fn test_schedule_cancel_action_serialization() {
        let action = ScheduleCancelAction {
            action_type: "scheduleCancel".to_string(),
            time: 1700000000000,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"scheduleCancel\""));
        assert!(json.contains("\"time\":1700000000000"));
    }

    #[test]
    fn test_schedule_cancel_request_serialization() {
        let action = ScheduleCancelAction {
            action_type: "scheduleCancel".to_string(),
            time: 1700000000000,
        };

        let request = ScheduleCancelExchangeRequest {
            action,
            nonce: 1700000000001,
            signature: Signature {
                r: "0x1234".to_string(),
                s: "0x5678".to_string(),
                v: 27,
            },
            vault_address: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"nonce\":1700000000001"));
        assert!(json.contains("\"signature\":"));
        assert!(!json.contains("\"vaultAddress\""));
    }

    #[test]
    fn test_schedule_cancel_request_with_vault() {
        let action = ScheduleCancelAction {
            action_type: "scheduleCancel".to_string(),
            time: 1700000000000,
        };

        let request = ScheduleCancelExchangeRequest {
            action,
            nonce: 1700000000001,
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
    fn test_schedule_cancel_response_deserialization() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "scheduleCancel",
                "data": {
                    "scheduledTime": 1700000000000
                }
            }
        }"#;

        let response: ScheduleCancelResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "scheduleCancel");
        assert_eq!(data.data.scheduled_time, Some(1700000000000));
    }

    #[test]
    fn test_schedule_cancel_response_no_scheduled_time() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "scheduleCancel",
                "data": {
                    "scheduledTime": null
                }
            }
        }"#;

        let response: ScheduleCancelResponse = serde_json::from_str(json).unwrap();
        let data = response.response.unwrap();
        assert_eq!(data.data.scheduled_time, None);
    }

    // Helper function tests

    #[test]
    fn test_is_schedule_cancel_successful_true() {
        let response = ScheduleCancelResponseData {
            response_type: "scheduleCancel".to_string(),
            data: ScheduleCancelResultData {
                scheduled_time: Some(1700000000000),
            },
        };
        assert!(is_schedule_cancel_successful(&response));
    }

    #[test]
    fn test_is_schedule_cancel_successful_false() {
        let response = ScheduleCancelResponseData {
            response_type: "scheduleCancel".to_string(),
            data: ScheduleCancelResultData {
                scheduled_time: None,
            },
        };
        assert!(!is_schedule_cancel_successful(&response));
    }

    #[test]
    fn test_get_scheduled_time_some() {
        let response = ScheduleCancelResponseData {
            response_type: "scheduleCancel".to_string(),
            data: ScheduleCancelResultData {
                scheduled_time: Some(1700000000000),
            },
        };
        assert_eq!(get_scheduled_time(&response), Some(1700000000000));
    }

    #[test]
    fn test_get_scheduled_time_none() {
        let response = ScheduleCancelResponseData {
            response_type: "scheduleCancel".to_string(),
            data: ScheduleCancelResultData {
                scheduled_time: None,
            },
        };
        assert_eq!(get_scheduled_time(&response), None);
    }

    // Validation tests

    #[test]
    fn test_timestamp_validation_past() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        // Timestamp in the past
        let past_timestamp = 1000;

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.schedule_cancel(&wallet, past_timestamp));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("at least 5 seconds in the future"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_timestamp_validation_too_soon() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        // Timestamp only 1 second in the future
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let too_soon_timestamp = now_ms + 1000;

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.schedule_cancel(&wallet, too_soon_timestamp));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("at least 5 seconds in the future"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    // Integration tests with mocked HTTP responses

    #[tokio::test]
    async fn test_schedule_cancel_success() {
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
                        "type": "scheduleCancel",
                        "data": {
                            "scheduledTime": 1700000060000
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        // Set timestamp 60 seconds in the future
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let future_timestamp = now_ms + 60_000;

        let response = client
            .schedule_cancel(&wallet, future_timestamp)
            .await
            .unwrap();

        assert_eq!(response.response_type, "scheduleCancel");
        assert!(is_schedule_cancel_successful(&response));
        assert_eq!(response.data.scheduled_time, Some(1700000060000));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_schedule_cancel_with_vault() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "scheduleCancel",
                        "data": {
                            "scheduledTime": 1700000060000
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

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let future_timestamp = now_ms + 60_000;

        let response = client
            .schedule_cancel_with_options(&wallet, future_timestamp, Some(vault_address))
            .await
            .unwrap();

        assert!(is_schedule_cancel_successful(&response));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_schedule_cancel_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let future_timestamp = now_ms + 60_000;

        let result = client.schedule_cancel(&wallet, future_timestamp).await;

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
    async fn test_schedule_cancel_api_error_status() {
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

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let future_timestamp = now_ms + 60_000;

        let result = client.schedule_cancel(&wallet, future_timestamp).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Schedule cancel failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_schedule_cancel_unauthorized() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(401)
            .with_body("Unauthorized: invalid signature")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let future_timestamp = now_ms + 60_000;

        let result = client.schedule_cancel(&wallet, future_timestamp).await;

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
    async fn test_schedule_cancel_rate_limited() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(429)
            .with_body("Rate limit exceeded")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let future_timestamp = now_ms + 60_000;

        let result = client.schedule_cancel(&wallet, future_timestamp).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("429"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    // Wallet signing tests

    #[test]
    fn test_wallet_signing_produces_signature() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = ScheduleCancelAction {
                action_type: "scheduleCancel".to_string(),
                time: 1700000000000,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }
}
