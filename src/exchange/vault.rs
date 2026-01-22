//! Vault action endpoints for the Hyperliquid Exchange API
//!
//! This module provides methods for vault deposit and withdrawal actions on
//! the Hyperliquid exchange.

use serde::{Deserialize, Serialize};

use crate::auth::{Signature, Wallet};
use crate::client::Client;
use crate::error::{Error, Result};
use crate::types::{VaultDepositAction, VaultWithdrawAction};

/// Exchange request wrapper with authentication for vault operations
#[derive(Debug, Clone, Serialize)]
pub struct VaultExchangeRequest<T> {
    pub action: T,
    pub nonce: u64,
    pub signature: Signature,
}

/// Exchange API response for vault operations
#[derive(Debug, Clone, Deserialize)]
pub struct VaultResponse {
    pub status: String,
    pub response: Option<VaultResponseData>,
}

/// Response data for vault operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultResponseData {
    #[serde(rename = "type")]
    pub response_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<VaultResultData>,
}

/// Result data for vault operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultResultData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl Client {
    /// Deposit USDC to a vault
    ///
    /// Deposits the specified amount of USDC to the given vault address.
    /// The user will receive vault shares proportional to their deposit.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `vault_address` - The address of the vault to deposit to
    /// * `amount` - The amount of USDC to deposit (as a string, e.g., "100.0")
    ///
    /// # Returns
    /// The vault response containing the transaction details
    ///
    /// # Errors
    /// Returns an error if:
    /// - The vault_address is empty
    /// - The amount is empty or invalid
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
    /// // Deposit 100 USDC to a vault
    /// let response = client.vault_deposit(
    ///     &wallet,
    ///     "0xvault1234567890123456789012345678901234",
    ///     "100.0"
    /// ).await?;
    /// ```
    pub async fn vault_deposit(
        &self,
        wallet: &Wallet,
        vault_address: &str,
        amount: &str,
    ) -> Result<VaultResponseData> {
        // Validate inputs
        if vault_address.is_empty() {
            return Err(Error::InvalidParameter(
                "Vault address cannot be empty".to_string(),
            ));
        }
        if amount.is_empty() {
            return Err(Error::InvalidParameter(
                "Amount cannot be empty".to_string(),
            ));
        }

        // Create the vault deposit action
        let action = VaultDepositAction {
            action_type: "vaultDeposit".to_string(),
            vault_address: vault_address.to_string(),
            amount: amount.to_string(),
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, None).await?;

        // Create the exchange request
        let request = VaultExchangeRequest {
            action,
            nonce,
            signature,
        };

        // Send request and parse response
        let response: VaultResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Vault deposit failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Withdraw USDC from a vault
    ///
    /// Withdraws the specified amount of USDC from the given vault address.
    /// The user's vault shares will be reduced proportionally.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `vault_address` - The address of the vault to withdraw from
    /// * `amount` - The amount of USDC to withdraw (as a string, e.g., "100.0")
    ///
    /// # Returns
    /// The vault response containing the transaction details
    ///
    /// # Errors
    /// Returns an error if:
    /// - The vault_address is empty
    /// - The amount is empty or invalid
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
    /// // Withdraw 50 USDC from a vault
    /// let response = client.vault_withdraw(
    ///     &wallet,
    ///     "0xvault1234567890123456789012345678901234",
    ///     "50.0"
    /// ).await?;
    /// ```
    pub async fn vault_withdraw(
        &self,
        wallet: &Wallet,
        vault_address: &str,
        amount: &str,
    ) -> Result<VaultResponseData> {
        // Validate inputs
        if vault_address.is_empty() {
            return Err(Error::InvalidParameter(
                "Vault address cannot be empty".to_string(),
            ));
        }
        if amount.is_empty() {
            return Err(Error::InvalidParameter(
                "Amount cannot be empty".to_string(),
            ));
        }

        // Create the vault withdraw action
        let action = VaultWithdrawAction {
            action_type: "vaultWithdraw".to_string(),
            vault_address: vault_address.to_string(),
            amount: amount.to_string(),
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, None).await?;

        // Create the exchange request
        let request = VaultExchangeRequest {
            action,
            nonce,
            signature,
        };

        // Send request and parse response
        let response: VaultResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Vault withdraw failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }
}

/// Check if a vault operation was successful
pub fn is_vault_action_successful(response: &VaultResponseData) -> bool {
    // A vault operation is successful if we get a valid response type
    !response.response_type.is_empty()
}

/// Get the transaction hash from a vault response
pub fn get_vault_hash(response: &VaultResponseData) -> Option<&str> {
    response.data.as_ref().and_then(|d| d.hash.as_deref())
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

        async fn vault_deposit(
            &self,
            wallet: &Wallet,
            vault_address: &str,
            amount: &str,
        ) -> Result<VaultResponseData> {
            if vault_address.is_empty() {
                return Err(Error::InvalidParameter(
                    "Vault address cannot be empty".to_string(),
                ));
            }
            if amount.is_empty() {
                return Err(Error::InvalidParameter(
                    "Amount cannot be empty".to_string(),
                ));
            }

            let action = VaultDepositAction {
                action_type: "vaultDeposit".to_string(),
                vault_address: vault_address.to_string(),
                amount: amount.to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = VaultExchangeRequest {
                action,
                nonce,
                signature,
            };

            let response: VaultResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Vault deposit failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn vault_withdraw(
            &self,
            wallet: &Wallet,
            vault_address: &str,
            amount: &str,
        ) -> Result<VaultResponseData> {
            if vault_address.is_empty() {
                return Err(Error::InvalidParameter(
                    "Vault address cannot be empty".to_string(),
                ));
            }
            if amount.is_empty() {
                return Err(Error::InvalidParameter(
                    "Amount cannot be empty".to_string(),
                ));
            }

            let action = VaultWithdrawAction {
                action_type: "vaultWithdraw".to_string(),
                vault_address: vault_address.to_string(),
                amount: amount.to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = VaultExchangeRequest {
                action,
                nonce,
                signature,
            };

            let response: VaultResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Vault withdraw failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }
    }

    // ============================================================================
    // Serialization Tests
    // ============================================================================

    #[test]
    fn test_vault_deposit_action_serialization() {
        let action = VaultDepositAction {
            action_type: "vaultDeposit".to_string(),
            vault_address: "0xvault1234567890123456789012345678901234".to_string(),
            amount: "100.0".to_string(),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"vaultDeposit\""));
        assert!(json.contains("\"vaultAddress\":\"0xvault1234567890123456789012345678901234\""));
        assert!(json.contains("\"amount\":\"100.0\""));
    }

    #[test]
    fn test_vault_withdraw_action_serialization() {
        let action = VaultWithdrawAction {
            action_type: "vaultWithdraw".to_string(),
            vault_address: "0xvault1234567890123456789012345678901234".to_string(),
            amount: "50.0".to_string(),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"vaultWithdraw\""));
        assert!(json.contains("\"vaultAddress\":\"0xvault1234567890123456789012345678901234\""));
        assert!(json.contains("\"amount\":\"50.0\""));
    }

    #[test]
    fn test_vault_exchange_request_serialization() {
        let action = VaultDepositAction {
            action_type: "vaultDeposit".to_string(),
            vault_address: "0xvault1234567890123456789012345678901234".to_string(),
            amount: "100.0".to_string(),
        };

        let request = VaultExchangeRequest {
            action,
            nonce: 1700000000000,
            signature: Signature {
                r: "0x1234".to_string(),
                s: "0x5678".to_string(),
                v: 27,
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"nonce\":1700000000000"));
        assert!(json.contains("\"signature\":"));
    }

    // ============================================================================
    // Response Deserialization Tests
    // ============================================================================

    #[test]
    fn test_vault_response_deserialization() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "vaultDeposit",
                "data": {
                    "hash": "0xabcdef1234567890"
                }
            }
        }"#;

        let response: VaultResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "vaultDeposit");
        assert!(data.data.is_some());
        let result = data.data.unwrap();
        assert_eq!(result.hash, Some("0xabcdef1234567890".to_string()));
    }

    #[test]
    fn test_vault_response_minimal() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "vaultWithdraw"
            }
        }"#;

        let response: VaultResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "vaultWithdraw");
        assert!(data.data.is_none());
    }

    // ============================================================================
    // Helper Function Tests
    // ============================================================================

    #[test]
    fn test_is_vault_action_successful_true() {
        let response = VaultResponseData {
            response_type: "vaultDeposit".to_string(),
            data: Some(VaultResultData {
                hash: Some("0xabcdef".to_string()),
            }),
        };
        assert!(is_vault_action_successful(&response));
    }

    #[test]
    fn test_is_vault_action_successful_no_data() {
        let response = VaultResponseData {
            response_type: "vaultWithdraw".to_string(),
            data: None,
        };
        assert!(is_vault_action_successful(&response));
    }

    #[test]
    fn test_get_vault_hash_some() {
        let response = VaultResponseData {
            response_type: "vaultDeposit".to_string(),
            data: Some(VaultResultData {
                hash: Some("0xabcdef1234".to_string()),
            }),
        };
        assert_eq!(get_vault_hash(&response), Some("0xabcdef1234"));
    }

    #[test]
    fn test_get_vault_hash_none() {
        let response = VaultResponseData {
            response_type: "vaultDeposit".to_string(),
            data: None,
        };
        assert_eq!(get_vault_hash(&response), None);
    }

    // ============================================================================
    // Validation Tests
    // ============================================================================

    #[test]
    fn test_vault_deposit_empty_vault_address() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.vault_deposit(&wallet, "", "100.0"));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Vault address"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_vault_deposit_empty_amount() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.vault_deposit(
            &wallet,
            "0xvault1234567890123456789012345678901234",
            "",
        ));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Amount"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_vault_withdraw_empty_vault_address() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.vault_withdraw(&wallet, "", "50.0"));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Vault address"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_vault_withdraw_empty_amount() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.vault_withdraw(
            &wallet,
            "0xvault1234567890123456789012345678901234",
            "",
        ));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Amount"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    // ============================================================================
    // Integration Tests with Mocked HTTP Responses
    // ============================================================================

    #[tokio::test]
    async fn test_vault_deposit_success() {
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
                        "type": "vaultDeposit",
                        "data": {
                            "hash": "0xdeposithash123"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .vault_deposit(
                &wallet,
                "0xvault1234567890123456789012345678901234",
                "100.0",
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "vaultDeposit");
        assert!(is_vault_action_successful(&response));
        assert_eq!(get_vault_hash(&response), Some("0xdeposithash123"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_vault_withdraw_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "vaultWithdraw",
                        "data": {
                            "hash": "0xwithdrawhash456"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .vault_withdraw(
                &wallet,
                "0xvault1234567890123456789012345678901234",
                "50.0",
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "vaultWithdraw");
        assert!(is_vault_action_successful(&response));
        assert_eq!(get_vault_hash(&response), Some("0xwithdrawhash456"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_vault_deposit_with_decimal_amount() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "vaultDeposit"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .vault_deposit(
                &wallet,
                "0xvault1234567890123456789012345678901234",
                "123.456789",
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "vaultDeposit");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_vault_withdraw_with_large_amount() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "vaultWithdraw"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .vault_withdraw(
                &wallet,
                "0xvault1234567890123456789012345678901234",
                "1000000.0",
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "vaultWithdraw");

        mock.assert_async().await;
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[tokio::test]
    async fn test_vault_deposit_http_error() {
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
            .vault_deposit(
                &wallet,
                "0xvault1234567890123456789012345678901234",
                "100.0",
            )
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
    async fn test_vault_deposit_api_error_status() {
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
            .vault_deposit(
                &wallet,
                "0xvault1234567890123456789012345678901234",
                "100.0",
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Vault deposit failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_vault_withdraw_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client
            .vault_withdraw(
                &wallet,
                "0xvault1234567890123456789012345678901234",
                "50.0",
            )
            .await;

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
    async fn test_vault_withdraw_api_error_status() {
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
            .vault_withdraw(
                &wallet,
                "0xvault1234567890123456789012345678901234",
                "50.0",
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Vault withdraw failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_vault_deposit_no_response_data() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "ok", "response": null}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client
            .vault_deposit(
                &wallet,
                "0xvault1234567890123456789012345678901234",
                "100.0",
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("No response data"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    // ============================================================================
    // Wallet Signing Tests
    // ============================================================================

    #[test]
    fn test_wallet_signing_produces_signature_for_vault_deposit() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = VaultDepositAction {
                action_type: "vaultDeposit".to_string(),
                vault_address: "0xvault1234567890123456789012345678901234".to_string(),
                amount: "100.0".to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_produces_signature_for_vault_withdraw() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = VaultWithdrawAction {
                action_type: "vaultWithdraw".to_string(),
                vault_address: "0xvault1234567890123456789012345678901234".to_string(),
                amount: "50.0".to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_with_different_vault_addresses() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action1 = VaultDepositAction {
                action_type: "vaultDeposit".to_string(),
                vault_address: "0xvault1111111111111111111111111111111111".to_string(),
                amount: "100.0".to_string(),
            };

            let action2 = VaultDepositAction {
                action_type: "vaultDeposit".to_string(),
                vault_address: "0xvault2222222222222222222222222222222222".to_string(),
                amount: "100.0".to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let sig1 = wallet.sign_l1_action(&action1, nonce, None).await.unwrap();
            let sig2 = wallet.sign_l1_action(&action2, nonce, None).await.unwrap();

            // Signatures should be different for different vault addresses
            assert_ne!(sig1.r, sig2.r);
        });
    }
}
