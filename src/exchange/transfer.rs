//! Transfer endpoints for the Hyperliquid Exchange API
//!
//! This module provides methods for transferring assets on the Hyperliquid exchange,
//! including USDC transfers, spot token transfers, L1 withdrawals, and internal
//! spot/perp transfers.

use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::auth::{Signature, Wallet};
use crate::client::Client;
use crate::error::{Error, Result};
use crate::types::{
    SendAssetAction, SpotPerpTransferAction, SpotTransferAction, UsdTransferAction,
    WithdrawAction,
};

/// Exchange request wrapper with authentication for transfer operations
#[derive(Debug, Clone, Serialize)]
pub struct TransferExchangeRequest<T> {
    pub action: T,
    pub nonce: u64,
    pub signature: Signature,
    #[serde(rename = "vaultAddress", skip_serializing_if = "Option::is_none")]
    pub vault_address: Option<String>,
}

/// Exchange API response for transfer operations
#[derive(Debug, Clone, Deserialize)]
pub struct TransferResponse {
    pub status: String,
    pub response: Option<TransferResponseData>,
}

/// Response data for transfer operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResponseData {
    #[serde(rename = "type")]
    pub response_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<TransferResultData>,
}

/// Result data for transfer operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResultData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
}

impl Client {
    /// Transfer USDC to another address
    ///
    /// Transfers USDC from the authenticated wallet to the destination address.
    /// This is an internal transfer within the Hyperliquid L1.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `destination` - The destination address to receive USDC
    /// * `amount` - The amount of USDC to transfer (as a string, e.g., "100.0")
    ///
    /// # Returns
    /// The transfer response containing the transaction hash
    ///
    /// # Errors
    /// Returns an error if:
    /// - The amount is empty or invalid
    /// - The destination address is empty
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
    /// // Transfer 100 USDC to another address
    /// let response = client.usd_transfer(
    ///     &wallet,
    ///     "0x1234567890123456789012345678901234567890",
    ///     "100.0"
    /// ).await?;
    /// ```
    pub async fn usd_transfer(
        &self,
        wallet: &Wallet,
        destination: &str,
        amount: &str,
    ) -> Result<TransferResponseData> {
        self.usd_transfer_with_options(wallet, destination, amount, None)
            .await
    }

    /// Transfer USDC with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `destination` - The destination address to receive USDC
    /// * `amount` - The amount of USDC to transfer
    /// * `vault_address` - Optional vault address to transfer from
    pub async fn usd_transfer_with_options(
        &self,
        wallet: &Wallet,
        destination: &str,
        amount: &str,
        vault_address: Option<Address>,
    ) -> Result<TransferResponseData> {
        // Validate inputs
        if destination.is_empty() {
            return Err(Error::InvalidParameter(
                "Destination address cannot be empty".to_string(),
            ));
        }
        if amount.is_empty() {
            return Err(Error::InvalidParameter(
                "Amount cannot be empty".to_string(),
            ));
        }

        // Create the USD transfer action
        let action = UsdTransferAction {
            action_type: "usdTransfer".to_string(),
            destination: destination.to_string(),
            amount: amount.to_string(),
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = TransferExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: TransferResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "USD transfer failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Transfer a spot token to another address
    ///
    /// Transfers a spot token from the authenticated wallet to the destination address.
    /// This is an internal transfer within the Hyperliquid L1.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `destination` - The destination address to receive the token
    /// * `token` - The token identifier to transfer
    /// * `amount` - The amount to transfer (as a string, e.g., "100.0")
    ///
    /// # Returns
    /// The transfer response containing the transaction hash
    ///
    /// # Errors
    /// Returns an error if:
    /// - The amount is empty or invalid
    /// - The destination address is empty
    /// - The token is empty
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
    /// // Transfer 100 PURR tokens to another address
    /// let response = client.spot_transfer(
    ///     &wallet,
    ///     "0x1234567890123456789012345678901234567890",
    ///     "PURR",
    ///     "100.0"
    /// ).await?;
    /// ```
    pub async fn spot_transfer(
        &self,
        wallet: &Wallet,
        destination: &str,
        token: &str,
        amount: &str,
    ) -> Result<TransferResponseData> {
        self.spot_transfer_with_options(wallet, destination, token, amount, None)
            .await
    }

    /// Transfer a spot token with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `destination` - The destination address to receive the token
    /// * `token` - The token identifier to transfer
    /// * `amount` - The amount to transfer
    /// * `vault_address` - Optional vault address to transfer from
    pub async fn spot_transfer_with_options(
        &self,
        wallet: &Wallet,
        destination: &str,
        token: &str,
        amount: &str,
        vault_address: Option<Address>,
    ) -> Result<TransferResponseData> {
        // Validate inputs
        if destination.is_empty() {
            return Err(Error::InvalidParameter(
                "Destination address cannot be empty".to_string(),
            ));
        }
        if token.is_empty() {
            return Err(Error::InvalidParameter(
                "Token cannot be empty".to_string(),
            ));
        }
        if amount.is_empty() {
            return Err(Error::InvalidParameter(
                "Amount cannot be empty".to_string(),
            ));
        }

        // Create the spot transfer action
        let action = SpotTransferAction {
            action_type: "spotTransfer".to_string(),
            destination: destination.to_string(),
            token: token.to_string(),
            amount: amount.to_string(),
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = TransferExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: TransferResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Spot transfer failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Send an asset (generalized transfer)
    ///
    /// A generalized transfer function that supports sending USDC, spot tokens,
    /// and other assets. This provides more control over the transfer parameters.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `destination` - The destination address to receive the asset
    /// * `token` - The token identifier (e.g., "USDC", "PURR")
    /// * `amount` - The amount to transfer (as a string, e.g., "100.0")
    /// * `time` - The timestamp for the transfer (used for signature)
    ///
    /// # Returns
    /// The transfer response containing the transaction hash
    ///
    /// # Errors
    /// Returns an error if:
    /// - The amount is empty or invalid
    /// - The destination address is empty
    /// - The token is empty
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
    /// let time = SystemTime::now()
    ///     .duration_since(UNIX_EPOCH)
    ///     .unwrap()
    ///     .as_millis() as u64;
    ///
    /// // Send 100 USDC to another address
    /// let response = client.send_asset(
    ///     &wallet,
    ///     "0x1234567890123456789012345678901234567890",
    ///     "USDC",
    ///     "100.0",
    ///     time,
    /// ).await?;
    /// ```
    pub async fn send_asset(
        &self,
        wallet: &Wallet,
        destination: &str,
        token: &str,
        amount: &str,
        time: u64,
    ) -> Result<TransferResponseData> {
        self.send_asset_with_options(wallet, destination, token, amount, time, None)
            .await
    }

    /// Send an asset with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `destination` - The destination address to receive the asset
    /// * `token` - The token identifier
    /// * `amount` - The amount to transfer
    /// * `time` - The timestamp for the transfer
    /// * `vault_address` - Optional vault address to transfer from
    pub async fn send_asset_with_options(
        &self,
        wallet: &Wallet,
        destination: &str,
        token: &str,
        amount: &str,
        time: u64,
        vault_address: Option<Address>,
    ) -> Result<TransferResponseData> {
        // Validate inputs
        if destination.is_empty() {
            return Err(Error::InvalidParameter(
                "Destination address cannot be empty".to_string(),
            ));
        }
        if token.is_empty() {
            return Err(Error::InvalidParameter(
                "Token cannot be empty".to_string(),
            ));
        }
        if amount.is_empty() {
            return Err(Error::InvalidParameter(
                "Amount cannot be empty".to_string(),
            ));
        }

        // Determine chain IDs based on network
        let (hyperliquid_chain, signature_chain_id) = if self.is_mainnet() {
            ("Mainnet".to_string(), "0xa4b1".to_string()) // Arbitrum mainnet chain ID
        } else {
            ("Testnet".to_string(), "0x66eee".to_string()) // Arbitrum Sepolia testnet
        };

        // Create the send asset action
        let action = SendAssetAction {
            action_type: "sendAsset".to_string(),
            hyperliquid_chain,
            signature_chain_id,
            destination: destination.to_string(),
            token: token.to_string(),
            amount: amount.to_string(),
            time,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = TransferExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: TransferResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Send asset failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Withdraw USDC to Arbitrum L1
    ///
    /// Initiates a withdrawal of USDC from Hyperliquid to an Arbitrum L1 address.
    /// This is a cross-chain transfer and may take time to complete.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `destination` - The Arbitrum L1 address to receive USDC
    /// * `amount` - The amount of USDC to withdraw (as a string, e.g., "100.0")
    ///
    /// # Returns
    /// The withdrawal response
    ///
    /// # Errors
    /// Returns an error if:
    /// - The amount is empty or invalid
    /// - The destination address is empty
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
    /// // Withdraw 100 USDC to Arbitrum L1
    /// let response = client.withdraw(
    ///     &wallet,
    ///     "0x1234567890123456789012345678901234567890",
    ///     "100.0"
    /// ).await?;
    /// ```
    pub async fn withdraw(
        &self,
        wallet: &Wallet,
        destination: &str,
        amount: &str,
    ) -> Result<TransferResponseData> {
        self.withdraw_with_options(wallet, destination, amount, None)
            .await
    }

    /// Withdraw USDC with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `destination` - The Arbitrum L1 address to receive USDC
    /// * `amount` - The amount of USDC to withdraw
    /// * `vault_address` - Optional vault address to withdraw from
    pub async fn withdraw_with_options(
        &self,
        wallet: &Wallet,
        destination: &str,
        amount: &str,
        vault_address: Option<Address>,
    ) -> Result<TransferResponseData> {
        // Validate inputs
        if destination.is_empty() {
            return Err(Error::InvalidParameter(
                "Destination address cannot be empty".to_string(),
            ));
        }
        if amount.is_empty() {
            return Err(Error::InvalidParameter(
                "Amount cannot be empty".to_string(),
            ));
        }

        // Create the withdraw action
        let action = WithdrawAction {
            action_type: "withdraw3".to_string(),
            destination: destination.to_string(),
            amount: amount.to_string(),
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = TransferExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: TransferResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!("Withdraw failed: {}", response.status)));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Transfer between spot and perpetual accounts
    ///
    /// Transfers USDC between the spot trading account and the perpetual trading
    /// account within the same wallet.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `amount` - The amount of USDC to transfer (as a string, e.g., "100.0")
    /// * `to_perp` - If true, transfer from spot to perp; if false, transfer from perp to spot
    ///
    /// # Returns
    /// The transfer response
    ///
    /// # Errors
    /// Returns an error if:
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
    /// // Transfer 100 USDC from spot to perp
    /// let response = client.spot_perp_transfer(&wallet, "100.0", true).await?;
    ///
    /// // Transfer 50 USDC from perp to spot
    /// let response = client.spot_perp_transfer(&wallet, "50.0", false).await?;
    /// ```
    pub async fn spot_perp_transfer(
        &self,
        wallet: &Wallet,
        amount: &str,
        to_perp: bool,
    ) -> Result<TransferResponseData> {
        self.spot_perp_transfer_with_options(wallet, amount, to_perp, None)
            .await
    }

    /// Transfer between spot and perpetual accounts with additional options
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `amount` - The amount of USDC to transfer
    /// * `to_perp` - If true, transfer from spot to perp; if false, transfer from perp to spot
    /// * `vault_address` - Optional vault address to transfer on behalf of
    pub async fn spot_perp_transfer_with_options(
        &self,
        wallet: &Wallet,
        amount: &str,
        to_perp: bool,
        vault_address: Option<Address>,
    ) -> Result<TransferResponseData> {
        // Validate inputs
        if amount.is_empty() {
            return Err(Error::InvalidParameter(
                "Amount cannot be empty".to_string(),
            ));
        }

        // Create the spot-perp transfer action
        let action = SpotPerpTransferAction {
            action_type: "spotUser".to_string(),
            amount: amount.to_string(),
            to_perp,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

        // Create the exchange request
        let request = TransferExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: vault_address.map(|addr| format!("{addr:?}")),
        };

        // Send request and parse response
        let response: TransferResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Spot-perp transfer failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }
}

/// Check if a transfer was successful
pub fn is_transfer_successful(response: &TransferResponseData) -> bool {
    // A transfer is successful if we get a valid response type
    !response.response_type.is_empty()
}

/// Get the transaction hash from a transfer response
pub fn get_transfer_hash(response: &TransferResponseData) -> Option<&str> {
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
        fn new(base_url: &str) -> Self {
            Self {
                http: ReqwestClient::new(),
                base_url: base_url.to_string(),
                is_mainnet: true,
            }
        }

        fn new_testnet(base_url: &str) -> Self {
            Self {
                http: ReqwestClient::new(),
                base_url: base_url.to_string(),
                is_mainnet: false,
            }
        }

        fn exchange_url(&self) -> String {
            format!("{}/exchange", self.base_url)
        }

        fn is_mainnet(&self) -> bool {
            self.is_mainnet
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

        async fn usd_transfer(
            &self,
            wallet: &Wallet,
            destination: &str,
            amount: &str,
        ) -> Result<TransferResponseData> {
            self.usd_transfer_with_options(wallet, destination, amount, None)
                .await
        }

        async fn usd_transfer_with_options(
            &self,
            wallet: &Wallet,
            destination: &str,
            amount: &str,
            vault_address: Option<Address>,
        ) -> Result<TransferResponseData> {
            if destination.is_empty() {
                return Err(Error::InvalidParameter(
                    "Destination address cannot be empty".to_string(),
                ));
            }
            if amount.is_empty() {
                return Err(Error::InvalidParameter(
                    "Amount cannot be empty".to_string(),
                ));
            }

            let action = UsdTransferAction {
                action_type: "usdTransfer".to_string(),
                destination: destination.to_string(),
                amount: amount.to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

            let request = TransferExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: vault_address.map(|addr| format!("{addr:?}")),
            };

            let response: TransferResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "USD transfer failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn spot_transfer(
            &self,
            wallet: &Wallet,
            destination: &str,
            token: &str,
            amount: &str,
        ) -> Result<TransferResponseData> {
            self.spot_transfer_with_options(wallet, destination, token, amount, None)
                .await
        }

        async fn spot_transfer_with_options(
            &self,
            wallet: &Wallet,
            destination: &str,
            token: &str,
            amount: &str,
            vault_address: Option<Address>,
        ) -> Result<TransferResponseData> {
            if destination.is_empty() {
                return Err(Error::InvalidParameter(
                    "Destination address cannot be empty".to_string(),
                ));
            }
            if token.is_empty() {
                return Err(Error::InvalidParameter(
                    "Token cannot be empty".to_string(),
                ));
            }
            if amount.is_empty() {
                return Err(Error::InvalidParameter(
                    "Amount cannot be empty".to_string(),
                ));
            }

            let action = SpotTransferAction {
                action_type: "spotTransfer".to_string(),
                destination: destination.to_string(),
                token: token.to_string(),
                amount: amount.to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

            let request = TransferExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: vault_address.map(|addr| format!("{addr:?}")),
            };

            let response: TransferResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Spot transfer failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn send_asset(
            &self,
            wallet: &Wallet,
            destination: &str,
            token: &str,
            amount: &str,
            time: u64,
        ) -> Result<TransferResponseData> {
            self.send_asset_with_options(wallet, destination, token, amount, time, None)
                .await
        }

        async fn send_asset_with_options(
            &self,
            wallet: &Wallet,
            destination: &str,
            token: &str,
            amount: &str,
            time: u64,
            vault_address: Option<Address>,
        ) -> Result<TransferResponseData> {
            if destination.is_empty() {
                return Err(Error::InvalidParameter(
                    "Destination address cannot be empty".to_string(),
                ));
            }
            if token.is_empty() {
                return Err(Error::InvalidParameter(
                    "Token cannot be empty".to_string(),
                ));
            }
            if amount.is_empty() {
                return Err(Error::InvalidParameter(
                    "Amount cannot be empty".to_string(),
                ));
            }

            let (hyperliquid_chain, signature_chain_id) = if self.is_mainnet() {
                ("Mainnet".to_string(), "0xa4b1".to_string())
            } else {
                ("Testnet".to_string(), "0x66eee".to_string())
            };

            let action = SendAssetAction {
                action_type: "sendAsset".to_string(),
                hyperliquid_chain,
                signature_chain_id,
                destination: destination.to_string(),
                token: token.to_string(),
                amount: amount.to_string(),
                time,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

            let request = TransferExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: vault_address.map(|addr| format!("{addr:?}")),
            };

            let response: TransferResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Send asset failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn withdraw(
            &self,
            wallet: &Wallet,
            destination: &str,
            amount: &str,
        ) -> Result<TransferResponseData> {
            self.withdraw_with_options(wallet, destination, amount, None)
                .await
        }

        async fn withdraw_with_options(
            &self,
            wallet: &Wallet,
            destination: &str,
            amount: &str,
            vault_address: Option<Address>,
        ) -> Result<TransferResponseData> {
            if destination.is_empty() {
                return Err(Error::InvalidParameter(
                    "Destination address cannot be empty".to_string(),
                ));
            }
            if amount.is_empty() {
                return Err(Error::InvalidParameter(
                    "Amount cannot be empty".to_string(),
                ));
            }

            let action = WithdrawAction {
                action_type: "withdraw3".to_string(),
                destination: destination.to_string(),
                amount: amount.to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

            let request = TransferExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: vault_address.map(|addr| format!("{addr:?}")),
            };

            let response: TransferResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!("Withdraw failed: {}", response.status)));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn spot_perp_transfer(
            &self,
            wallet: &Wallet,
            amount: &str,
            to_perp: bool,
        ) -> Result<TransferResponseData> {
            self.spot_perp_transfer_with_options(wallet, amount, to_perp, None)
                .await
        }

        async fn spot_perp_transfer_with_options(
            &self,
            wallet: &Wallet,
            amount: &str,
            to_perp: bool,
            vault_address: Option<Address>,
        ) -> Result<TransferResponseData> {
            if amount.is_empty() {
                return Err(Error::InvalidParameter(
                    "Amount cannot be empty".to_string(),
                ));
            }

            let action = SpotPerpTransferAction {
                action_type: "spotUser".to_string(),
                amount: amount.to_string(),
                to_perp,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, vault_address).await?;

            let request = TransferExchangeRequest {
                action,
                nonce,
                signature,
                vault_address: vault_address.map(|addr| format!("{addr:?}")),
            };

            let response: TransferResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Spot-perp transfer failed: {}",
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
    fn test_usd_transfer_action_serialization() {
        let action = UsdTransferAction {
            action_type: "usdTransfer".to_string(),
            destination: "0x1234567890123456789012345678901234567890".to_string(),
            amount: "100.0".to_string(),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"usdTransfer\""));
        assert!(json.contains("\"destination\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"amount\":\"100.0\""));
    }

    #[test]
    fn test_spot_transfer_action_serialization() {
        let action = SpotTransferAction {
            action_type: "spotTransfer".to_string(),
            destination: "0x1234567890123456789012345678901234567890".to_string(),
            token: "PURR".to_string(),
            amount: "50.5".to_string(),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"spotTransfer\""));
        assert!(json.contains("\"token\":\"PURR\""));
        assert!(json.contains("\"amount\":\"50.5\""));
    }

    #[test]
    fn test_send_asset_action_serialization() {
        let action = SendAssetAction {
            action_type: "sendAsset".to_string(),
            hyperliquid_chain: "Mainnet".to_string(),
            signature_chain_id: "0xa4b1".to_string(),
            destination: "0x1234567890123456789012345678901234567890".to_string(),
            token: "USDC".to_string(),
            amount: "100.0".to_string(),
            time: 1700000000000,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"sendAsset\""));
        assert!(json.contains("\"hyperliquidChain\":\"Mainnet\""));
        assert!(json.contains("\"signatureChainId\":\"0xa4b1\""));
        assert!(json.contains("\"time\":1700000000000"));
    }

    #[test]
    fn test_withdraw_action_serialization() {
        let action = WithdrawAction {
            action_type: "withdraw3".to_string(),
            destination: "0x1234567890123456789012345678901234567890".to_string(),
            amount: "200.0".to_string(),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"withdraw3\""));
        assert!(json.contains("\"destination\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"amount\":\"200.0\""));
    }

    #[test]
    fn test_spot_perp_transfer_action_serialization_to_perp() {
        let action = SpotPerpTransferAction {
            action_type: "spotUser".to_string(),
            amount: "100.0".to_string(),
            to_perp: true,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"spotUser\""));
        assert!(json.contains("\"amount\":\"100.0\""));
        assert!(json.contains("\"toPerp\":true"));
    }

    #[test]
    fn test_spot_perp_transfer_action_serialization_to_spot() {
        let action = SpotPerpTransferAction {
            action_type: "spotUser".to_string(),
            amount: "50.0".to_string(),
            to_perp: false,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"toPerp\":false"));
    }

    #[test]
    fn test_transfer_exchange_request_serialization() {
        let action = UsdTransferAction {
            action_type: "usdTransfer".to_string(),
            destination: "0x1234567890123456789012345678901234567890".to_string(),
            amount: "100.0".to_string(),
        };

        let request = TransferExchangeRequest {
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
    fn test_transfer_exchange_request_with_vault() {
        let action = UsdTransferAction {
            action_type: "usdTransfer".to_string(),
            destination: "0x1234567890123456789012345678901234567890".to_string(),
            amount: "100.0".to_string(),
        };

        let request = TransferExchangeRequest {
            action,
            nonce: 1700000000000,
            signature: Signature {
                r: "0x1234".to_string(),
                s: "0x5678".to_string(),
                v: 27,
            },
            vault_address: Some("0xabcdef1234567890123456789012345678901234".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"vaultAddress\":\"0xabcdef1234567890123456789012345678901234\""));
    }

    // Response deserialization tests

    #[test]
    fn test_transfer_response_deserialization() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "usdTransfer",
                "data": {
                    "hash": "0xabcdef1234567890",
                    "nonce": 12345
                }
            }
        }"#;

        let response: TransferResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "usdTransfer");
        assert!(data.data.is_some());
        let result = data.data.unwrap();
        assert_eq!(result.hash, Some("0xabcdef1234567890".to_string()));
        assert_eq!(result.nonce, Some(12345));
    }

    #[test]
    fn test_transfer_response_minimal() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "spotTransfer"
            }
        }"#;

        let response: TransferResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "spotTransfer");
        assert!(data.data.is_none());
    }

    // Helper function tests

    #[test]
    fn test_is_transfer_successful_true() {
        let response = TransferResponseData {
            response_type: "usdTransfer".to_string(),
            data: Some(TransferResultData {
                hash: Some("0xabcdef".to_string()),
                nonce: Some(123),
            }),
        };
        assert!(is_transfer_successful(&response));
    }

    #[test]
    fn test_is_transfer_successful_no_data() {
        let response = TransferResponseData {
            response_type: "spotTransfer".to_string(),
            data: None,
        };
        assert!(is_transfer_successful(&response));
    }

    #[test]
    fn test_get_transfer_hash_some() {
        let response = TransferResponseData {
            response_type: "usdTransfer".to_string(),
            data: Some(TransferResultData {
                hash: Some("0xabcdef1234".to_string()),
                nonce: None,
            }),
        };
        assert_eq!(get_transfer_hash(&response), Some("0xabcdef1234"));
    }

    #[test]
    fn test_get_transfer_hash_none() {
        let response = TransferResponseData {
            response_type: "usdTransfer".to_string(),
            data: None,
        };
        assert_eq!(get_transfer_hash(&response), None);
    }

    // Validation tests

    #[test]
    fn test_usd_transfer_empty_destination() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.usd_transfer(&wallet, "", "100.0"));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Destination"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_usd_transfer_empty_amount() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.usd_transfer(
            &wallet,
            "0x1234567890123456789012345678901234567890",
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
    fn test_spot_transfer_empty_token() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.spot_transfer(
            &wallet,
            "0x1234567890123456789012345678901234567890",
            "",
            "100.0",
        ));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Token"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    #[test]
    fn test_spot_perp_transfer_empty_amount() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.spot_perp_transfer(&wallet, "", true));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("Amount"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    // Integration tests with mocked HTTP responses

    #[tokio::test]
    async fn test_usd_transfer_success() {
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
                        "type": "usdTransfer",
                        "data": {
                            "hash": "0xabcdef123456",
                            "nonce": 12345
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .usd_transfer(
                &wallet,
                "0x1234567890123456789012345678901234567890",
                "100.0",
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "usdTransfer");
        assert!(is_transfer_successful(&response));
        assert_eq!(get_transfer_hash(&response), Some("0xabcdef123456"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_transfer_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "spotTransfer",
                        "data": {
                            "hash": "0xspottx123"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .spot_transfer(
                &wallet,
                "0x1234567890123456789012345678901234567890",
                "PURR",
                "50.0",
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "spotTransfer");
        assert!(is_transfer_successful(&response));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_send_asset_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "sendAsset",
                        "data": {
                            "hash": "0xsendassettx123"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .send_asset(
                &wallet,
                "0x1234567890123456789012345678901234567890",
                "USDC",
                "100.0",
                1700000000000,
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "sendAsset");
        assert!(is_transfer_successful(&response));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_send_asset_testnet() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "sendAsset"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new_testnet(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, false).unwrap();

        let response = client
            .send_asset(
                &wallet,
                "0x1234567890123456789012345678901234567890",
                "USDC",
                "100.0",
                1700000000000,
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "sendAsset");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_withdraw_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "withdraw3",
                        "data": {
                            "hash": "0xwithdrawtx123"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .withdraw(
                &wallet,
                "0x1234567890123456789012345678901234567890",
                "200.0",
            )
            .await
            .unwrap();

        assert_eq!(response.response_type, "withdraw3");
        assert!(is_transfer_successful(&response));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_perp_transfer_to_perp_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "spotUser"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .spot_perp_transfer(&wallet, "100.0", true)
            .await
            .unwrap();

        assert_eq!(response.response_type, "spotUser");
        assert!(is_transfer_successful(&response));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_perp_transfer_to_spot_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "spotUser"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client
            .spot_perp_transfer(&wallet, "50.0", false)
            .await
            .unwrap();

        assert_eq!(response.response_type, "spotUser");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_usd_transfer_with_vault() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "usdTransfer"
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let vault_address: Address = "0xabcdef1234567890123456789012345678901234"
            .parse()
            .unwrap();

        let response = client
            .usd_transfer_with_options(
                &wallet,
                "0x1234567890123456789012345678901234567890",
                "100.0",
                Some(vault_address),
            )
            .await
            .unwrap();

        assert!(is_transfer_successful(&response));

        mock.assert_async().await;
    }

    // Error handling tests

    #[tokio::test]
    async fn test_usd_transfer_http_error() {
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
            .usd_transfer(
                &wallet,
                "0x1234567890123456789012345678901234567890",
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
    async fn test_usd_transfer_api_error_status() {
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
            .usd_transfer(
                &wallet,
                "0x1234567890123456789012345678901234567890",
                "100.0",
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("USD transfer failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_transfer_http_error() {
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
            .spot_transfer(
                &wallet,
                "0x1234567890123456789012345678901234567890",
                "PURR",
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
    async fn test_withdraw_api_error_status() {
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
            .withdraw(
                &wallet,
                "0x1234567890123456789012345678901234567890",
                "100.0",
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Withdraw failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_spot_perp_transfer_api_error_status() {
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

        let result = client.spot_perp_transfer(&wallet, "100.0", true).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Spot-perp transfer failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    // Wallet signing tests

    #[test]
    fn test_wallet_signing_produces_signature_for_usd_transfer() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = UsdTransferAction {
                action_type: "usdTransfer".to_string(),
                destination: "0x1234567890123456789012345678901234567890".to_string(),
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
    fn test_wallet_signing_produces_signature_for_spot_transfer() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = SpotTransferAction {
                action_type: "spotTransfer".to_string(),
                destination: "0x1234567890123456789012345678901234567890".to_string(),
                token: "PURR".to_string(),
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
    fn test_wallet_signing_produces_signature_for_withdraw() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = WithdrawAction {
                action_type: "withdraw3".to_string(),
                destination: "0x1234567890123456789012345678901234567890".to_string(),
                amount: "200.0".to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_produces_signature_for_spot_perp_transfer() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = SpotPerpTransferAction {
                action_type: "spotUser".to_string(),
                amount: "100.0".to_string(),
                to_perp: true,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }
}
