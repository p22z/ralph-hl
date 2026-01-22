//! Wallet and builder approval endpoints for the Hyperliquid Exchange API
//!
//! This module provides methods for approving API wallets (agents) and builder fees
//! on the Hyperliquid exchange.

use serde::{Deserialize, Serialize};

use crate::auth::{Signature, Wallet};
use crate::client::Client;
use crate::error::{Error, Result};

/// Approve agent action - authorize an API wallet to trade on behalf of the user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveAgentAction {
    #[serde(rename = "type")]
    pub action_type: String,
    /// The address of the agent (API wallet) to approve
    #[serde(rename = "hyperliquidChain")]
    pub hyperliquid_chain: String,
    /// Signature chain ID for the action
    #[serde(rename = "signatureChainId")]
    pub signature_chain_id: String,
    /// The agent address to approve
    #[serde(rename = "agentAddress")]
    pub agent_address: String,
    /// Optional name for the agent
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "agentName")]
    pub agent_name: Option<String>,
    /// Nonce for the action
    pub nonce: u64,
}

/// Approve builder fee action - authorize a builder to charge fees on orders
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveBuilderFeeAction {
    #[serde(rename = "type")]
    pub action_type: String,
    /// The address of the builder to approve
    #[serde(rename = "hyperliquidChain")]
    pub hyperliquid_chain: String,
    /// Signature chain ID for the action
    #[serde(rename = "signatureChainId")]
    pub signature_chain_id: String,
    /// The builder address
    pub builder: String,
    /// Maximum fee rate allowed (in basis points as a string, e.g., "0.001" for 0.1%)
    #[serde(rename = "maxFeeRate")]
    pub max_fee_rate: String,
    /// Nonce for the action
    pub nonce: u64,
}

/// Exchange request wrapper with authentication for approval operations
#[derive(Debug, Clone, Serialize)]
pub struct ApprovalExchangeRequest<T> {
    pub action: T,
    pub nonce: u64,
    pub signature: Signature,
}

/// Exchange API response for approval operations
#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalResponse {
    pub status: String,
    pub response: Option<ApprovalResponseData>,
}

/// Response data for approval operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponseData {
    #[serde(rename = "type")]
    pub response_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ApprovalResultData>,
}

/// Result data for approval operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResultData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl Client {
    /// Approve an agent (API wallet) to trade on behalf of the user
    ///
    /// This authorizes an API wallet to perform trading actions on your behalf.
    /// The agent must be approved before it can place orders or perform other
    /// trading actions for your account.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with (main wallet)
    /// * `agent_address` - The address of the agent (API wallet) to approve
    /// * `agent_name` - Optional human-readable name for the agent
    ///
    /// # Returns
    /// The approval response containing the transaction details
    ///
    /// # Errors
    /// Returns an error if:
    /// - The agent_address is empty
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
    /// // Approve an API wallet to trade on your behalf
    /// let response = client.approve_agent(
    ///     &wallet,
    ///     "0xagent1234567890123456789012345678901234",
    ///     Some("My Trading Bot".to_string())
    /// ).await?;
    /// ```
    pub async fn approve_agent(
        &self,
        wallet: &Wallet,
        agent_address: &str,
        agent_name: Option<String>,
    ) -> Result<ApprovalResponseData> {
        // Validate inputs
        if agent_address.is_empty() {
            return Err(Error::InvalidParameter(
                "Agent address cannot be empty".to_string(),
            ));
        }

        // Generate nonce
        let nonce = Wallet::generate_nonce();

        // Determine chain values based on network
        let (hyperliquid_chain, signature_chain_id) = if wallet.is_mainnet() {
            ("Mainnet".to_string(), "0x539".to_string()) // 1337 in hex
        } else {
            ("Testnet".to_string(), "0x66eee".to_string()) // 421614 in hex (Arbitrum Sepolia)
        };

        // Create the approve agent action
        let action = ApproveAgentAction {
            action_type: "approveAgent".to_string(),
            hyperliquid_chain,
            signature_chain_id,
            agent_address: agent_address.to_string(),
            agent_name,
            nonce,
        };

        // Sign the action
        let signature = wallet.sign_l1_action(&action, nonce, None).await?;

        // Create the exchange request
        let request = ApprovalExchangeRequest {
            action,
            nonce,
            signature,
        };

        // Send request and parse response
        let response: ApprovalResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Approve agent failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Approve a builder to charge fees on orders
    ///
    /// This authorizes a builder address to include additional fees when routing
    /// orders through their infrastructure. The max_fee_rate sets the maximum
    /// fee the builder can charge.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `builder` - The builder address to approve
    /// * `max_fee_rate` - Maximum fee rate allowed (as a decimal string, e.g., "0.001" for 0.1%)
    ///
    /// # Returns
    /// The approval response containing the transaction details
    ///
    /// # Errors
    /// Returns an error if:
    /// - The builder address is empty
    /// - The max_fee_rate is empty or invalid
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
    /// // Approve a builder with max 0.1% fee
    /// let response = client.approve_builder_fee(
    ///     &wallet,
    ///     "0xbuilder1234567890123456789012345678901234",
    ///     "0.001"
    /// ).await?;
    /// ```
    pub async fn approve_builder_fee(
        &self,
        wallet: &Wallet,
        builder: &str,
        max_fee_rate: &str,
    ) -> Result<ApprovalResponseData> {
        // Validate inputs
        if builder.is_empty() {
            return Err(Error::InvalidParameter(
                "Builder address cannot be empty".to_string(),
            ));
        }
        if max_fee_rate.is_empty() {
            return Err(Error::InvalidParameter(
                "Max fee rate cannot be empty".to_string(),
            ));
        }

        // Validate max_fee_rate is a valid number
        if max_fee_rate.parse::<f64>().is_err() {
            return Err(Error::InvalidParameter(
                "Max fee rate must be a valid number".to_string(),
            ));
        }

        // Generate nonce
        let nonce = Wallet::generate_nonce();

        // Determine chain values based on network
        let (hyperliquid_chain, signature_chain_id) = if wallet.is_mainnet() {
            ("Mainnet".to_string(), "0x539".to_string()) // 1337 in hex
        } else {
            ("Testnet".to_string(), "0x66eee".to_string()) // 421614 in hex
        };

        // Create the approve builder fee action
        let action = ApproveBuilderFeeAction {
            action_type: "approveBuilderFee".to_string(),
            hyperliquid_chain,
            signature_chain_id,
            builder: builder.to_string(),
            max_fee_rate: max_fee_rate.to_string(),
            nonce,
        };

        // Sign the action
        let signature = wallet.sign_l1_action(&action, nonce, None).await?;

        // Create the exchange request
        let request = ApprovalExchangeRequest {
            action,
            nonce,
            signature,
        };

        // Send request and parse response
        let response: ApprovalResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Approve builder fee failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }
}

/// Check if an approval operation was successful
pub fn is_approval_successful(response: &ApprovalResponseData) -> bool {
    // An approval operation is successful if we get a valid response type
    !response.response_type.is_empty()
}

/// Get the transaction hash from an approval response
pub fn get_approval_hash(response: &ApprovalResponseData) -> Option<&str> {
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
        is_mainnet: bool,
    }

    impl TestClient {
        fn new(base_url: &str, is_mainnet: bool) -> Self {
            Self {
                http: ReqwestClient::new(),
                base_url: base_url.to_string(),
                is_mainnet,
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

        async fn approve_agent(
            &self,
            wallet: &Wallet,
            agent_address: &str,
            agent_name: Option<String>,
        ) -> Result<ApprovalResponseData> {
            if agent_address.is_empty() {
                return Err(Error::InvalidParameter(
                    "Agent address cannot be empty".to_string(),
                ));
            }

            let nonce = Wallet::generate_nonce();

            let (hyperliquid_chain, signature_chain_id) = if self.is_mainnet {
                ("Mainnet".to_string(), "0x539".to_string())
            } else {
                ("Testnet".to_string(), "0x66eee".to_string())
            };

            let action = ApproveAgentAction {
                action_type: "approveAgent".to_string(),
                hyperliquid_chain,
                signature_chain_id,
                agent_address: agent_address.to_string(),
                agent_name,
                nonce,
            };

            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = ApprovalExchangeRequest {
                action,
                nonce,
                signature,
            };

            let response: ApprovalResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Approve agent failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn approve_builder_fee(
            &self,
            wallet: &Wallet,
            builder: &str,
            max_fee_rate: &str,
        ) -> Result<ApprovalResponseData> {
            if builder.is_empty() {
                return Err(Error::InvalidParameter(
                    "Builder address cannot be empty".to_string(),
                ));
            }
            if max_fee_rate.is_empty() {
                return Err(Error::InvalidParameter(
                    "Max fee rate cannot be empty".to_string(),
                ));
            }
            if max_fee_rate.parse::<f64>().is_err() {
                return Err(Error::InvalidParameter(
                    "Max fee rate must be a valid number".to_string(),
                ));
            }

            let nonce = Wallet::generate_nonce();

            let (hyperliquid_chain, signature_chain_id) = if self.is_mainnet {
                ("Mainnet".to_string(), "0x539".to_string())
            } else {
                ("Testnet".to_string(), "0x66eee".to_string())
            };

            let action = ApproveBuilderFeeAction {
                action_type: "approveBuilderFee".to_string(),
                hyperliquid_chain,
                signature_chain_id,
                builder: builder.to_string(),
                max_fee_rate: max_fee_rate.to_string(),
                nonce,
            };

            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = ApprovalExchangeRequest {
                action,
                nonce,
                signature,
            };

            let response: ApprovalResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Approve builder fee failed: {}",
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
    fn test_approve_agent_action_serialization() {
        let action = ApproveAgentAction {
            action_type: "approveAgent".to_string(),
            hyperliquid_chain: "Mainnet".to_string(),
            signature_chain_id: "0x539".to_string(),
            agent_address: "0x1234567890123456789012345678901234567890".to_string(),
            agent_name: Some("My Bot".to_string()),
            nonce: 1700000000000,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"approveAgent\""));
        assert!(json.contains("\"hyperliquidChain\":\"Mainnet\""));
        assert!(json.contains("\"signatureChainId\":\"0x539\""));
        assert!(json.contains("\"agentAddress\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"agentName\":\"My Bot\""));
        assert!(json.contains("\"nonce\":1700000000000"));
    }

    #[test]
    fn test_approve_agent_action_without_name() {
        let action = ApproveAgentAction {
            action_type: "approveAgent".to_string(),
            hyperliquid_chain: "Testnet".to_string(),
            signature_chain_id: "0x66eee".to_string(),
            agent_address: "0xabcdef1234567890123456789012345678901234".to_string(),
            agent_name: None,
            nonce: 1700000000000,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"approveAgent\""));
        assert!(json.contains("\"hyperliquidChain\":\"Testnet\""));
        assert!(!json.contains("agentName")); // Should be omitted
    }

    #[test]
    fn test_approve_builder_fee_action_serialization() {
        let action = ApproveBuilderFeeAction {
            action_type: "approveBuilderFee".to_string(),
            hyperliquid_chain: "Mainnet".to_string(),
            signature_chain_id: "0x539".to_string(),
            builder: "0x1234567890123456789012345678901234567890".to_string(),
            max_fee_rate: "0.001".to_string(),
            nonce: 1700000000000,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"approveBuilderFee\""));
        assert!(json.contains("\"hyperliquidChain\":\"Mainnet\""));
        assert!(json.contains("\"signatureChainId\":\"0x539\""));
        assert!(json.contains("\"builder\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"maxFeeRate\":\"0.001\""));
        assert!(json.contains("\"nonce\":1700000000000"));
    }

    #[test]
    fn test_approval_exchange_request_serialization() {
        let action = ApproveAgentAction {
            action_type: "approveAgent".to_string(),
            hyperliquid_chain: "Mainnet".to_string(),
            signature_chain_id: "0x539".to_string(),
            agent_address: "0x1234567890123456789012345678901234567890".to_string(),
            agent_name: None,
            nonce: 1700000000000,
        };

        let request = ApprovalExchangeRequest {
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
    fn test_approval_response_deserialization() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "approveAgent",
                "data": {
                    "hash": "0xabcdef1234567890"
                }
            }
        }"#;

        let response: ApprovalResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "approveAgent");
        assert!(data.data.is_some());
        let result = data.data.unwrap();
        assert_eq!(result.hash, Some("0xabcdef1234567890".to_string()));
    }

    #[test]
    fn test_approval_response_minimal() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "approveBuilderFee"
            }
        }"#;

        let response: ApprovalResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "approveBuilderFee");
        assert!(data.data.is_none());
    }

    // ============================================================================
    // Helper Function Tests
    // ============================================================================

    #[test]
    fn test_is_approval_successful_true() {
        let response = ApprovalResponseData {
            response_type: "approveAgent".to_string(),
            data: Some(ApprovalResultData {
                hash: Some("0xabcdef".to_string()),
            }),
        };
        assert!(is_approval_successful(&response));
    }

    #[test]
    fn test_is_approval_successful_no_data() {
        let response = ApprovalResponseData {
            response_type: "approveBuilderFee".to_string(),
            data: None,
        };
        assert!(is_approval_successful(&response));
    }

    #[test]
    fn test_get_approval_hash_some() {
        let response = ApprovalResponseData {
            response_type: "approveAgent".to_string(),
            data: Some(ApprovalResultData {
                hash: Some("0xabcdef1234".to_string()),
            }),
        };
        assert_eq!(get_approval_hash(&response), Some("0xabcdef1234"));
    }

    #[test]
    fn test_get_approval_hash_none() {
        let response = ApprovalResponseData {
            response_type: "approveAgent".to_string(),
            data: None,
        };
        assert_eq!(get_approval_hash(&response), None);
    }

    // ============================================================================
    // Validation Tests
    // ============================================================================

    #[test]
    fn test_approve_agent_empty_address() {
        let client = TestClient::new("http://localhost:12345", true);
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.approve_agent(&wallet, "", None));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Agent address"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_approve_builder_fee_empty_builder() {
        let client = TestClient::new("http://localhost:12345", true);
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.approve_builder_fee(&wallet, "", "0.001"));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Builder address"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_approve_builder_fee_empty_rate() {
        let client = TestClient::new("http://localhost:12345", true);
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.approve_builder_fee(
            &wallet,
            "0x1234567890123456789012345678901234567890",
            "",
        ));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Max fee rate"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_approve_builder_fee_invalid_rate() {
        let client = TestClient::new("http://localhost:12345", true);
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.approve_builder_fee(
            &wallet,
            "0x1234567890123456789012345678901234567890",
            "not-a-number",
        ));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("valid number"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    // ============================================================================
    // Integration Tests with Mocked HTTP Responses
    // ============================================================================

    #[tokio::test]
    async fn test_approve_agent_success() {
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
                        "type": "approveAgent",
                        "data": {
                            "hash": "0xagenthash123"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url(), true);
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .approve_agent(
                &wallet,
                "0xagent1234567890123456789012345678901234",
                Some("My Bot".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "approveAgent");
        assert!(is_approval_successful(&response));
        assert_eq!(get_approval_hash(&response), Some("0xagenthash123"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_approve_agent_without_name() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "approveAgent",
                        "data": {
                            "hash": "0xagenthash456"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url(), true);
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .approve_agent(&wallet, "0xagent1234567890123456789012345678901234", None)
            .await
            .unwrap();

        assert_eq!(response.response_type, "approveAgent");
        assert!(is_approval_successful(&response));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_approve_builder_fee_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "approveBuilderFee",
                        "data": {
                            "hash": "0xbuilderhash789"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url(), true);
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .approve_builder_fee(
                &wallet,
                "0xbuilder1234567890123456789012345678901234",
                "0.001",
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "approveBuilderFee");
        assert!(is_approval_successful(&response));
        assert_eq!(get_approval_hash(&response), Some("0xbuilderhash789"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_approve_agent_testnet() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "approveAgent",
                        "data": {
                            "hash": "0xtestnethash"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url(), false);
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, false).unwrap();

        let response = client
            .approve_agent(
                &wallet,
                "0xagent1234567890123456789012345678901234",
                Some("Testnet Bot".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "approveAgent");
        assert!(is_approval_successful(&response));

        mock.assert_async().await;
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[tokio::test]
    async fn test_approve_agent_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url(), true);
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client
            .approve_agent(&wallet, "0xagent1234567890123456789012345678901234", None)
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
    async fn test_approve_agent_api_error_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "err", "response": null}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url(), true);
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client
            .approve_agent(&wallet, "0xagent1234567890123456789012345678901234", None)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Approve agent failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_approve_builder_fee_api_error_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "err", "response": null}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url(), true);
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client
            .approve_builder_fee(
                &wallet,
                "0xbuilder1234567890123456789012345678901234",
                "0.001",
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Approve builder fee failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_approve_builder_fee_http_unauthorized() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let client = TestClient::new(&server.url(), true);
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client
            .approve_builder_fee(
                &wallet,
                "0xbuilder1234567890123456789012345678901234",
                "0.001",
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
    fn test_wallet_signing_produces_signature_for_approve_agent() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = ApproveAgentAction {
                action_type: "approveAgent".to_string(),
                hyperliquid_chain: "Mainnet".to_string(),
                signature_chain_id: "0x539".to_string(),
                agent_address: "0x1234567890123456789012345678901234567890".to_string(),
                agent_name: Some("Test Bot".to_string()),
                nonce: 1700000000000,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_produces_signature_for_approve_builder_fee() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = ApproveBuilderFeeAction {
                action_type: "approveBuilderFee".to_string(),
                hyperliquid_chain: "Mainnet".to_string(),
                signature_chain_id: "0x539".to_string(),
                builder: "0x1234567890123456789012345678901234567890".to_string(),
                max_fee_rate: "0.001".to_string(),
                nonce: 1700000000000,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_approve_agent_mainnet_vs_testnet_different_chain_values() {
        // Mainnet action
        let mainnet_action = ApproveAgentAction {
            action_type: "approveAgent".to_string(),
            hyperliquid_chain: "Mainnet".to_string(),
            signature_chain_id: "0x539".to_string(),
            agent_address: "0x1234567890123456789012345678901234567890".to_string(),
            agent_name: None,
            nonce: 1700000000000,
        };

        // Testnet action
        let testnet_action = ApproveAgentAction {
            action_type: "approveAgent".to_string(),
            hyperliquid_chain: "Testnet".to_string(),
            signature_chain_id: "0x66eee".to_string(),
            agent_address: "0x1234567890123456789012345678901234567890".to_string(),
            agent_name: None,
            nonce: 1700000000000,
        };

        let mainnet_json = serde_json::to_string(&mainnet_action).unwrap();
        let testnet_json = serde_json::to_string(&testnet_action).unwrap();

        // Verify they have different chain values
        assert!(mainnet_json.contains("\"hyperliquidChain\":\"Mainnet\""));
        assert!(mainnet_json.contains("\"signatureChainId\":\"0x539\""));
        assert!(testnet_json.contains("\"hyperliquidChain\":\"Testnet\""));
        assert!(testnet_json.contains("\"signatureChainId\":\"0x66eee\""));
    }
}
