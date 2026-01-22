//! Staking action endpoints for the Hyperliquid Exchange API
//!
//! This module provides methods for staking actions on the Hyperliquid exchange,
//! including depositing to staking, withdrawing from staking, delegating to validators,
//! and undelegating from validators.

use serde::{Deserialize, Serialize};

use crate::auth::{Signature, Wallet};
use crate::client::Client;
use crate::error::{Error, Result};

/// Staking deposit action - deposit tokens to staking pool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakeDepositAction {
    #[serde(rename = "type")]
    pub action_type: String,
    /// Amount to deposit in wei (as a string)
    pub wei: String,
}

/// Staking withdraw action - withdraw tokens from staking pool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakeWithdrawAction {
    #[serde(rename = "type")]
    pub action_type: String,
    /// Amount to withdraw in wei (as a string)
    pub wei: String,
}

/// Delegate action - delegate staked tokens to a validator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateAction {
    #[serde(rename = "type")]
    pub action_type: String,
    /// Validator address to delegate to
    pub validator: String,
    /// Amount to delegate in wei (as a string)
    pub wei: String,
}

/// Undelegate action - undelegate tokens from a validator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndelegateAction {
    #[serde(rename = "type")]
    pub action_type: String,
    /// Validator address to undelegate from
    pub validator: String,
    /// Amount to undelegate in wei (as a string)
    pub wei: String,
}

/// Exchange request wrapper with authentication for staking operations
#[derive(Debug, Clone, Serialize)]
pub struct StakingExchangeRequest<T> {
    pub action: T,
    pub nonce: u64,
    pub signature: Signature,
}

/// Exchange API response for staking operations
#[derive(Debug, Clone, Deserialize)]
pub struct StakingResponse {
    pub status: String,
    pub response: Option<StakingResponseData>,
}

/// Response data for staking operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingResponseData {
    #[serde(rename = "type")]
    pub response_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<StakingResultData>,
}

/// Result data for staking operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingResultData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl Client {
    /// Deposit tokens to staking
    ///
    /// Deposits the specified amount of tokens (in wei) to the staking pool.
    /// This is the first step before delegating to a validator.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `wei_amount` - The amount to deposit in wei (as a string, e.g., "1000000000000000000" for 1 token)
    ///
    /// # Returns
    /// The staking response containing the transaction details
    ///
    /// # Errors
    /// Returns an error if:
    /// - The wei_amount is empty or invalid
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
    /// // Deposit 1 token (in wei) to staking
    /// let response = client.stake_deposit(&wallet, "1000000000000000000").await?;
    /// ```
    pub async fn stake_deposit(
        &self,
        wallet: &Wallet,
        wei_amount: &str,
    ) -> Result<StakingResponseData> {
        // Validate inputs
        if wei_amount.is_empty() {
            return Err(Error::InvalidParameter(
                "Wei amount cannot be empty".to_string(),
            ));
        }

        // Create the stake deposit action
        let action = StakeDepositAction {
            action_type: "cDeposit".to_string(),
            wei: wei_amount.to_string(),
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, None).await?;

        // Create the exchange request
        let request = StakingExchangeRequest {
            action,
            nonce,
            signature,
        };

        // Send request and parse response
        let response: StakingResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Stake deposit failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Withdraw tokens from staking
    ///
    /// Withdraws the specified amount of tokens (in wei) from the staking pool.
    /// Tokens must be undelegated before they can be withdrawn.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `wei_amount` - The amount to withdraw in wei (as a string, e.g., "1000000000000000000" for 1 token)
    ///
    /// # Returns
    /// The staking response containing the transaction details
    ///
    /// # Errors
    /// Returns an error if:
    /// - The wei_amount is empty or invalid
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
    /// // Withdraw 1 token (in wei) from staking
    /// let response = client.stake_withdraw(&wallet, "1000000000000000000").await?;
    /// ```
    pub async fn stake_withdraw(
        &self,
        wallet: &Wallet,
        wei_amount: &str,
    ) -> Result<StakingResponseData> {
        // Validate inputs
        if wei_amount.is_empty() {
            return Err(Error::InvalidParameter(
                "Wei amount cannot be empty".to_string(),
            ));
        }

        // Create the stake withdraw action
        let action = StakeWithdrawAction {
            action_type: "cWithdraw".to_string(),
            wei: wei_amount.to_string(),
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, None).await?;

        // Create the exchange request
        let request = StakingExchangeRequest {
            action,
            nonce,
            signature,
        };

        // Send request and parse response
        let response: StakingResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Stake withdraw failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Delegate staked tokens to a validator
    ///
    /// Delegates the specified amount of staked tokens to a validator.
    /// Tokens must first be deposited to staking before they can be delegated.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `validator` - The validator address to delegate to
    /// * `wei_amount` - The amount to delegate in wei (as a string, e.g., "1000000000000000000" for 1 token)
    ///
    /// # Returns
    /// The staking response containing the transaction details
    ///
    /// # Errors
    /// Returns an error if:
    /// - The validator address is empty
    /// - The wei_amount is empty or invalid
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
    /// // Delegate 1 token (in wei) to a validator
    /// let response = client.delegate(
    ///     &wallet,
    ///     "0xvalidator1234567890123456789012345678901234",
    ///     "1000000000000000000"
    /// ).await?;
    /// ```
    pub async fn delegate(
        &self,
        wallet: &Wallet,
        validator: &str,
        wei_amount: &str,
    ) -> Result<StakingResponseData> {
        // Validate inputs
        if validator.is_empty() {
            return Err(Error::InvalidParameter(
                "Validator address cannot be empty".to_string(),
            ));
        }
        if wei_amount.is_empty() {
            return Err(Error::InvalidParameter(
                "Wei amount cannot be empty".to_string(),
            ));
        }

        // Create the delegate action
        let action = DelegateAction {
            action_type: "cDelegate".to_string(),
            validator: validator.to_string(),
            wei: wei_amount.to_string(),
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, None).await?;

        // Create the exchange request
        let request = StakingExchangeRequest {
            action,
            nonce,
            signature,
        };

        // Send request and parse response
        let response: StakingResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!("Delegate failed: {}", response.status)));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Undelegate tokens from a validator
    ///
    /// Undelegates the specified amount of tokens from a validator.
    /// After undelegating, there may be a waiting period before tokens can be withdrawn.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `validator` - The validator address to undelegate from
    /// * `wei_amount` - The amount to undelegate in wei (as a string, e.g., "1000000000000000000" for 1 token)
    ///
    /// # Returns
    /// The staking response containing the transaction details
    ///
    /// # Errors
    /// Returns an error if:
    /// - The validator address is empty
    /// - The wei_amount is empty or invalid
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
    /// // Undelegate 1 token (in wei) from a validator
    /// let response = client.undelegate(
    ///     &wallet,
    ///     "0xvalidator1234567890123456789012345678901234",
    ///     "1000000000000000000"
    /// ).await?;
    /// ```
    pub async fn undelegate(
        &self,
        wallet: &Wallet,
        validator: &str,
        wei_amount: &str,
    ) -> Result<StakingResponseData> {
        // Validate inputs
        if validator.is_empty() {
            return Err(Error::InvalidParameter(
                "Validator address cannot be empty".to_string(),
            ));
        }
        if wei_amount.is_empty() {
            return Err(Error::InvalidParameter(
                "Wei amount cannot be empty".to_string(),
            ));
        }

        // Create the undelegate action
        let action = UndelegateAction {
            action_type: "cUndelegate".to_string(),
            validator: validator.to_string(),
            wei: wei_amount.to_string(),
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, None).await?;

        // Create the exchange request
        let request = StakingExchangeRequest {
            action,
            nonce,
            signature,
        };

        // Send request and parse response
        let response: StakingResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Undelegate failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }
}

/// Check if a staking operation was successful
pub fn is_staking_successful(response: &StakingResponseData) -> bool {
    // A staking operation is successful if we get a valid response type
    !response.response_type.is_empty()
}

/// Get the transaction hash from a staking response
pub fn get_staking_hash(response: &StakingResponseData) -> Option<&str> {
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

        async fn stake_deposit(
            &self,
            wallet: &Wallet,
            wei_amount: &str,
        ) -> Result<StakingResponseData> {
            if wei_amount.is_empty() {
                return Err(Error::InvalidParameter(
                    "Wei amount cannot be empty".to_string(),
                ));
            }

            let action = StakeDepositAction {
                action_type: "cDeposit".to_string(),
                wei: wei_amount.to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = StakingExchangeRequest {
                action,
                nonce,
                signature,
            };

            let response: StakingResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Stake deposit failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn stake_withdraw(
            &self,
            wallet: &Wallet,
            wei_amount: &str,
        ) -> Result<StakingResponseData> {
            if wei_amount.is_empty() {
                return Err(Error::InvalidParameter(
                    "Wei amount cannot be empty".to_string(),
                ));
            }

            let action = StakeWithdrawAction {
                action_type: "cWithdraw".to_string(),
                wei: wei_amount.to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = StakingExchangeRequest {
                action,
                nonce,
                signature,
            };

            let response: StakingResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Stake withdraw failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn delegate(
            &self,
            wallet: &Wallet,
            validator: &str,
            wei_amount: &str,
        ) -> Result<StakingResponseData> {
            if validator.is_empty() {
                return Err(Error::InvalidParameter(
                    "Validator address cannot be empty".to_string(),
                ));
            }
            if wei_amount.is_empty() {
                return Err(Error::InvalidParameter(
                    "Wei amount cannot be empty".to_string(),
                ));
            }

            let action = DelegateAction {
                action_type: "cDelegate".to_string(),
                validator: validator.to_string(),
                wei: wei_amount.to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = StakingExchangeRequest {
                action,
                nonce,
                signature,
            };

            let response: StakingResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!("Delegate failed: {}", response.status)));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn undelegate(
            &self,
            wallet: &Wallet,
            validator: &str,
            wei_amount: &str,
        ) -> Result<StakingResponseData> {
            if validator.is_empty() {
                return Err(Error::InvalidParameter(
                    "Validator address cannot be empty".to_string(),
                ));
            }
            if wei_amount.is_empty() {
                return Err(Error::InvalidParameter(
                    "Wei amount cannot be empty".to_string(),
                ));
            }

            let action = UndelegateAction {
                action_type: "cUndelegate".to_string(),
                validator: validator.to_string(),
                wei: wei_amount.to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = StakingExchangeRequest {
                action,
                nonce,
                signature,
            };

            let response: StakingResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Undelegate failed: {}",
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
    fn test_stake_deposit_action_serialization() {
        let action = StakeDepositAction {
            action_type: "cDeposit".to_string(),
            wei: "1000000000000000000".to_string(),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"cDeposit\""));
        assert!(json.contains("\"wei\":\"1000000000000000000\""));
    }

    #[test]
    fn test_stake_withdraw_action_serialization() {
        let action = StakeWithdrawAction {
            action_type: "cWithdraw".to_string(),
            wei: "500000000000000000".to_string(),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"cWithdraw\""));
        assert!(json.contains("\"wei\":\"500000000000000000\""));
    }

    #[test]
    fn test_delegate_action_serialization() {
        let action = DelegateAction {
            action_type: "cDelegate".to_string(),
            validator: "0x1234567890123456789012345678901234567890".to_string(),
            wei: "1000000000000000000".to_string(),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"cDelegate\""));
        assert!(json.contains("\"validator\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"wei\":\"1000000000000000000\""));
    }

    #[test]
    fn test_undelegate_action_serialization() {
        let action = UndelegateAction {
            action_type: "cUndelegate".to_string(),
            validator: "0xabcdef1234567890123456789012345678901234".to_string(),
            wei: "250000000000000000".to_string(),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"cUndelegate\""));
        assert!(json.contains("\"validator\":\"0xabcdef1234567890123456789012345678901234\""));
        assert!(json.contains("\"wei\":\"250000000000000000\""));
    }

    #[test]
    fn test_staking_exchange_request_serialization() {
        let action = StakeDepositAction {
            action_type: "cDeposit".to_string(),
            wei: "1000000000000000000".to_string(),
        };

        let request = StakingExchangeRequest {
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
    fn test_staking_response_deserialization() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "cDeposit",
                "data": {
                    "hash": "0xabcdef1234567890"
                }
            }
        }"#;

        let response: StakingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "cDeposit");
        assert!(data.data.is_some());
        let result = data.data.unwrap();
        assert_eq!(result.hash, Some("0xabcdef1234567890".to_string()));
    }

    #[test]
    fn test_staking_response_minimal() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "cDelegate"
            }
        }"#;

        let response: StakingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "cDelegate");
        assert!(data.data.is_none());
    }

    // ============================================================================
    // Helper Function Tests
    // ============================================================================

    #[test]
    fn test_is_staking_successful_true() {
        let response = StakingResponseData {
            response_type: "cDeposit".to_string(),
            data: Some(StakingResultData {
                hash: Some("0xabcdef".to_string()),
            }),
        };
        assert!(is_staking_successful(&response));
    }

    #[test]
    fn test_is_staking_successful_no_data() {
        let response = StakingResponseData {
            response_type: "cWithdraw".to_string(),
            data: None,
        };
        assert!(is_staking_successful(&response));
    }

    #[test]
    fn test_get_staking_hash_some() {
        let response = StakingResponseData {
            response_type: "cDeposit".to_string(),
            data: Some(StakingResultData {
                hash: Some("0xabcdef1234".to_string()),
            }),
        };
        assert_eq!(get_staking_hash(&response), Some("0xabcdef1234"));
    }

    #[test]
    fn test_get_staking_hash_none() {
        let response = StakingResponseData {
            response_type: "cDeposit".to_string(),
            data: None,
        };
        assert_eq!(get_staking_hash(&response), None);
    }

    // ============================================================================
    // Validation Tests
    // ============================================================================

    #[test]
    fn test_stake_deposit_empty_wei() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.stake_deposit(&wallet, ""));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Wei amount"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_stake_withdraw_empty_wei() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.stake_withdraw(&wallet, ""));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Wei amount"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_delegate_empty_validator() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.delegate(&wallet, "", "1000000000000000000"));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Validator"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_delegate_empty_wei() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result =
            rt.block_on(client.delegate(&wallet, "0x1234567890123456789012345678901234567890", ""));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Wei amount"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_undelegate_empty_validator() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.undelegate(&wallet, "", "1000000000000000000"));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Validator"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_undelegate_empty_wei() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.undelegate(
            &wallet,
            "0x1234567890123456789012345678901234567890",
            "",
        ));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Wei amount"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    // ============================================================================
    // Integration Tests with Mocked HTTP Responses
    // ============================================================================

    #[tokio::test]
    async fn test_stake_deposit_success() {
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
                        "type": "cDeposit",
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
            .stake_deposit(&wallet, "1000000000000000000")
            .await
            .unwrap();

        assert_eq!(response.response_type, "cDeposit");
        assert!(is_staking_successful(&response));
        assert_eq!(get_staking_hash(&response), Some("0xdeposithash123"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_stake_withdraw_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "cWithdraw",
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
            .stake_withdraw(&wallet, "500000000000000000")
            .await
            .unwrap();

        assert_eq!(response.response_type, "cWithdraw");
        assert!(is_staking_successful(&response));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_delegate_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "cDelegate",
                        "data": {
                            "hash": "0xdelegatehash789"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .delegate(
                &wallet,
                "0xvalidator1234567890123456789012345678901234",
                "1000000000000000000",
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "cDelegate");
        assert!(is_staking_successful(&response));
        assert_eq!(get_staking_hash(&response), Some("0xdelegatehash789"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_undelegate_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "cUndelegate",
                        "data": {
                            "hash": "0xundelegatehash012"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .undelegate(
                &wallet,
                "0xvalidator1234567890123456789012345678901234",
                "500000000000000000",
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "cUndelegate");
        assert!(is_staking_successful(&response));
        assert_eq!(get_staking_hash(&response), Some("0xundelegatehash012"));

        mock.assert_async().await;
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[tokio::test]
    async fn test_stake_deposit_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.stake_deposit(&wallet, "1000000000000000000").await;

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
    async fn test_stake_deposit_api_error_status() {
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

        let result = client.stake_deposit(&wallet, "1000000000000000000").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Stake deposit failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_stake_withdraw_api_error_status() {
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

        let result = client.stake_withdraw(&wallet, "500000000000000000").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Stake withdraw failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_delegate_api_error_status() {
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
            .delegate(
                &wallet,
                "0xvalidator1234567890123456789012345678901234",
                "1000000000000000000",
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Delegate failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_undelegate_api_error_status() {
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
            .undelegate(
                &wallet,
                "0xvalidator1234567890123456789012345678901234",
                "500000000000000000",
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Undelegate failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_delegate_http_unauthorized() {
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
            .delegate(
                &wallet,
                "0xvalidator1234567890123456789012345678901234",
                "1000000000000000000",
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

    // ============================================================================
    // Wallet Signing Tests
    // ============================================================================

    #[test]
    fn test_wallet_signing_produces_signature_for_stake_deposit() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = StakeDepositAction {
                action_type: "cDeposit".to_string(),
                wei: "1000000000000000000".to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_produces_signature_for_stake_withdraw() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = StakeWithdrawAction {
                action_type: "cWithdraw".to_string(),
                wei: "500000000000000000".to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_produces_signature_for_delegate() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = DelegateAction {
                action_type: "cDelegate".to_string(),
                validator: "0x1234567890123456789012345678901234567890".to_string(),
                wei: "1000000000000000000".to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_produces_signature_for_undelegate() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = UndelegateAction {
                action_type: "cUndelegate".to_string(),
                validator: "0x1234567890123456789012345678901234567890".to_string(),
                wei: "500000000000000000".to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }
}
