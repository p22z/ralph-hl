//! Vault and miscellaneous info endpoints
//!
//! This module provides methods for querying vault information and miscellaneous
//! data from the Hyperliquid API.

use crate::client::Client;
use crate::error::Result;
use crate::types::{
    ActiveAssetData, ActiveAssetDataRequest, BuilderFeeApprovalRequest, Hip3StateRequest,
    PerpDeployAuctionStatus, PerpDeployAuctionStatusRequest, PerpDexLimits, PerpDexLimitsRequest,
    PerpDexStatusRequest, PerpsAtOpenInterestCapRequest, ReferralRequest, UserVaultsRequest,
    VaultDetailsRequest,
};

// ============================================================================
// Response Types
// ============================================================================

/// Vault details response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultDetails {
    /// Vault name
    pub name: String,
    /// Vault description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Vault portfolio value (TVL)
    pub portfolio_vlm: String,
    /// Vault leader's address
    pub leader: String,
    /// Leader's commission rate
    pub leader_commission: String,
    /// Leader's share of the vault
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader_fraction: Option<String>,
    /// Max capacity of the vault
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_capacity: Option<String>,
    /// Whether vault is closed to new deposits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_closed: Option<bool>,
    /// Vault creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,
    /// All-time PnL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_time_pnl: Option<String>,
    /// Number of followers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follower_count: Option<u64>,
}

/// User vault deposit information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserVaultDeposit {
    /// Vault address
    pub vault_address: String,
    /// Amount deposited by user
    pub equity_share: String,
    /// User's share of the vault as a fraction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equity_fraction: Option<String>,
    /// Current value of deposit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Pending withdrawal amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_withdrawal: Option<String>,
}

/// Builder fee approval information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuilderFeeApprovalInfo {
    /// Whether approval exists
    pub approved: bool,
    /// Maximum fee rate approved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_rate: Option<String>,
    /// Nonce of the approval
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
}

/// Referral information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferralInfo {
    /// User's referral code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referral_code: Option<String>,
    /// Referrer's address (who referred this user)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referred_by: Option<String>,
    /// Referral discount rate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cumulative_referral_discount: Option<String>,
    /// Total volume referred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cumulative_referral_vlm: Option<String>,
    /// Number of referrals
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referral_count: Option<u64>,
}

/// HIP-3 abstraction state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hip3State {
    /// Whether HIP-3 is enabled for this user
    pub enabled: bool,
    /// Current HIP-3 balance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<String>,
}

/// Perp DEX status information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerpDexStatus {
    /// DEX name
    pub name: String,
    /// Whether the DEX is active
    pub is_active: bool,
    /// Number of assets listed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_assets: Option<u32>,
    /// Total open interest
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_open_interest: Option<String>,
    /// 24h volume
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_volume: Option<String>,
}

impl Client {
    /// Retrieve vault details
    ///
    /// Returns detailed information about a specific vault including
    /// its configuration, leader information, and performance metrics.
    ///
    /// # Arguments
    /// * `vault_address` - The vault's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let details = client.vault_details("0xvault...").await?;
    /// println!("Vault: {}, TVL: {}", details.name, details.portfolio_vlm);
    /// ```
    pub async fn vault_details(&self, vault_address: &str) -> Result<VaultDetails> {
        let request = VaultDetailsRequest::new(vault_address);
        self.post_info(&request).await
    }

    /// Retrieve user's vault deposits
    ///
    /// Returns a list of all vaults the user has deposited into,
    /// including their equity share and current value.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let vaults = client.user_vaults("0x1234...").await?;
    /// for vault in vaults {
    ///     println!("Vault: {}, Share: {}", vault.vault_address, vault.equity_share);
    /// }
    /// ```
    pub async fn user_vaults(&self, user: &str) -> Result<Vec<UserVaultDeposit>> {
        let request = UserVaultsRequest::new(user);
        self.post_info(&request).await
    }

    /// Check builder fee approval status
    ///
    /// Returns whether a user has approved a specific builder for fee collection
    /// and the maximum approved fee rate.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    /// * `builder` - The builder's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let approval = client.builder_fee_approval("0xuser...", "0xbuilder...").await?;
    /// if approval.approved {
    ///     println!("Max fee rate: {:?}", approval.max_fee_rate);
    /// }
    /// ```
    pub async fn builder_fee_approval(
        &self,
        user: &str,
        builder: &str,
    ) -> Result<BuilderFeeApprovalInfo> {
        let request = BuilderFeeApprovalRequest::new(user, builder);
        self.post_info(&request).await
    }

    /// Retrieve referral information for a user
    ///
    /// Returns the user's referral code, who referred them, and
    /// cumulative referral statistics.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let referral = client.referral("0x1234...").await?;
    /// if let Some(code) = referral.referral_code {
    ///     println!("Referral code: {}", code);
    /// }
    /// ```
    pub async fn referral(&self, user: &str) -> Result<ReferralInfo> {
        let request = ReferralRequest::new(user);
        self.post_info(&request).await
    }

    /// Retrieve HIP-3 abstraction state for a user
    ///
    /// Returns whether the user has HIP-3 enabled and their
    /// current HIP-3 balance.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let hip3 = client.hip3_state("0x1234...").await?;
    /// println!("HIP-3 enabled: {}", hip3.enabled);
    /// ```
    pub async fn hip3_state(&self, user: &str) -> Result<Hip3State> {
        let request = Hip3StateRequest::new(user);
        self.post_info(&request).await
    }

    /// Retrieve user's active asset data for a specific coin
    ///
    /// Returns leverage information, max trade sizes, and available
    /// margin for a specific asset.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    /// * `coin` - The asset symbol (e.g., "BTC")
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let data = client.active_asset_data("0x1234...", "BTC").await?;
    /// println!("Max long: {}, Max short: {}", data.max_trade_szs.0, data.max_trade_szs.1);
    /// ```
    pub async fn active_asset_data(&self, user: &str, coin: &str) -> Result<ActiveAssetData> {
        let request = ActiveAssetDataRequest::new(user, coin);
        self.post_info(&request).await
    }

    /// Retrieve perps at open interest cap
    ///
    /// Returns a list of perpetual assets that have reached their
    /// open interest cap, optionally filtered by DEX.
    ///
    /// # Arguments
    /// * `dex` - Optional DEX filter
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let capped = client.perps_at_open_interest_cap(None).await?;
    /// for coin in capped {
    ///     println!("Capped: {}", coin);
    /// }
    /// ```
    pub async fn perps_at_open_interest_cap(&self, dex: Option<&str>) -> Result<Vec<String>> {
        let mut request = PerpsAtOpenInterestCapRequest::default();
        if let Some(d) = dex {
            request.dex = Some(d.to_string());
        }
        self.post_info(&request).await
    }

    /// Retrieve perp DEX limits
    ///
    /// Returns the limits and caps for a specific perpetual DEX.
    ///
    /// # Arguments
    /// * `dex` - The DEX identifier
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let limits = client.perp_dex_limits("perp").await?;
    /// println!("Total OI cap: {}", limits.total_oi_cap);
    /// ```
    pub async fn perp_dex_limits(&self, dex: &str) -> Result<PerpDexLimits> {
        let request = PerpDexLimitsRequest::new(dex);
        self.post_info(&request).await
    }

    /// Retrieve perp DEX status
    ///
    /// Returns the current status and statistics for a specific perpetual DEX.
    ///
    /// # Arguments
    /// * `dex` - The DEX identifier
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let status = client.perp_dex_status("perp").await?;
    /// println!("DEX: {}, Active: {}", status.name, status.is_active);
    /// ```
    pub async fn perp_dex_status(&self, dex: &str) -> Result<PerpDexStatus> {
        let request = PerpDexStatusRequest::new(dex);
        self.post_info(&request).await
    }

    /// Retrieve perp deploy auction status
    ///
    /// Returns information about the current perpetual deployment auction
    /// including start time, duration, and gas prices.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let auction = client.perp_deploy_auction_status().await?;
    /// println!("Current gas: {}", auction.current_gas);
    /// ```
    pub async fn perp_deploy_auction_status(&self) -> Result<PerpDeployAuctionStatus> {
        let request = PerpDeployAuctionStatusRequest::default();
        self.post_info(&request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
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

        async fn vault_details(&self, vault_address: &str) -> Result<VaultDetails> {
            let request = VaultDetailsRequest::new(vault_address);
            self.post_info(&request).await
        }

        async fn user_vaults(&self, user: &str) -> Result<Vec<UserVaultDeposit>> {
            let request = UserVaultsRequest::new(user);
            self.post_info(&request).await
        }

        async fn builder_fee_approval(
            &self,
            user: &str,
            builder: &str,
        ) -> Result<BuilderFeeApprovalInfo> {
            let request = BuilderFeeApprovalRequest::new(user, builder);
            self.post_info(&request).await
        }

        async fn referral(&self, user: &str) -> Result<ReferralInfo> {
            let request = ReferralRequest::new(user);
            self.post_info(&request).await
        }

        async fn hip3_state(&self, user: &str) -> Result<Hip3State> {
            let request = Hip3StateRequest::new(user);
            self.post_info(&request).await
        }

        async fn active_asset_data(&self, user: &str, coin: &str) -> Result<ActiveAssetData> {
            let request = ActiveAssetDataRequest::new(user, coin);
            self.post_info(&request).await
        }

        async fn perps_at_open_interest_cap(&self, dex: Option<&str>) -> Result<Vec<String>> {
            let mut request = PerpsAtOpenInterestCapRequest::default();
            if let Some(d) = dex {
                request.dex = Some(d.to_string());
            }
            self.post_info(&request).await
        }

        async fn perp_dex_limits(&self, dex: &str) -> Result<PerpDexLimits> {
            let request = PerpDexLimitsRequest::new(dex);
            self.post_info(&request).await
        }

        async fn perp_dex_status(&self, dex: &str) -> Result<PerpDexStatus> {
            let request = PerpDexStatusRequest::new(dex);
            self.post_info(&request).await
        }

        async fn perp_deploy_auction_status(&self) -> Result<PerpDeployAuctionStatus> {
            let request = PerpDeployAuctionStatusRequest::default();
            self.post_info(&request).await
        }
    }

    // ========================================================================
    // Vault Details Tests
    // ========================================================================

    #[tokio::test]
    async fn test_vault_details() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "vaultDetails",
                "vaultAddress": "0xvault1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "name": "Alpha Vault",
                    "description": "High performance trading vault",
                    "portfolioVlm": "1000000.00",
                    "leader": "0xleader12345678901234567890123456789012345678",
                    "leaderCommission": "0.1",
                    "leaderFraction": "0.2",
                    "maxCapacity": "5000000.00",
                    "isClosed": false,
                    "created": 1700000000000,
                    "allTimePnl": "150000.00",
                    "followerCount": 50
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let details = client
            .vault_details("0xvault1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(details.name, "Alpha Vault");
        assert_eq!(
            details.description,
            Some("High performance trading vault".to_string())
        );
        assert_eq!(details.portfolio_vlm, "1000000.00");
        assert_eq!(
            details.leader,
            "0xleader12345678901234567890123456789012345678"
        );
        assert_eq!(details.leader_commission, "0.1");
        assert_eq!(details.leader_fraction, Some("0.2".to_string()));
        assert_eq!(details.max_capacity, Some("5000000.00".to_string()));
        assert_eq!(details.is_closed, Some(false));
        assert_eq!(details.created, Some(1700000000000));
        assert_eq!(details.all_time_pnl, Some("150000.00".to_string()));
        assert_eq!(details.follower_count, Some(50));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_vault_details_minimal() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "vaultDetails",
                "vaultAddress": "0xvault1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "name": "Simple Vault",
                    "portfolioVlm": "100.00",
                    "leader": "0xleader12345678901234567890123456789012345678",
                    "leaderCommission": "0.05"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let details = client
            .vault_details("0xvault1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(details.name, "Simple Vault");
        assert!(details.description.is_none());
        assert!(details.max_capacity.is_none());

        mock.assert_async().await;
    }

    // ========================================================================
    // User Vaults Tests
    // ========================================================================

    #[tokio::test]
    async fn test_user_vaults() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userVaults",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "vaultAddress": "0xvault1234567890123456789012345678901234567890",
                        "equityShare": "10000.00",
                        "equityFraction": "0.01",
                        "value": "10500.00",
                        "pendingWithdrawal": null
                    },
                    {
                        "vaultAddress": "0xvault2345678901234567890123456789012345678901",
                        "equityShare": "5000.00",
                        "equityFraction": "0.005",
                        "value": "5200.00",
                        "pendingWithdrawal": "1000.00"
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let vaults = client
            .user_vaults("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(vaults.len(), 2);
        assert_eq!(
            vaults[0].vault_address,
            "0xvault1234567890123456789012345678901234567890"
        );
        assert_eq!(vaults[0].equity_share, "10000.00");
        assert_eq!(vaults[0].equity_fraction, Some("0.01".to_string()));
        assert_eq!(vaults[0].value, Some("10500.00".to_string()));
        assert!(vaults[0].pending_withdrawal.is_none());

        assert_eq!(vaults[1].pending_withdrawal, Some("1000.00".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_vaults_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userVaults",
                "user": "0xdeadbeef12345678901234567890123456789012"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let vaults = client
            .user_vaults("0xdeadbeef12345678901234567890123456789012")
            .await
            .unwrap();

        assert_eq!(vaults.len(), 0);

        mock.assert_async().await;
    }

    // ========================================================================
    // Builder Fee Approval Tests
    // ========================================================================

    #[tokio::test]
    async fn test_builder_fee_approval_approved() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "builderFeeApproval",
                "user": "0x1234567890123456789012345678901234567890",
                "builder": "0xbuilder123456789012345678901234567890123"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "approved": true,
                    "maxFeeRate": "0.001",
                    "nonce": 12345
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let approval = client
            .builder_fee_approval(
                "0x1234567890123456789012345678901234567890",
                "0xbuilder123456789012345678901234567890123",
            )
            .await
            .unwrap();

        assert!(approval.approved);
        assert_eq!(approval.max_fee_rate, Some("0.001".to_string()));
        assert_eq!(approval.nonce, Some(12345));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_builder_fee_approval_not_approved() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "builderFeeApproval",
                "user": "0x1234567890123456789012345678901234567890",
                "builder": "0xbuilder123456789012345678901234567890123"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "approved": false
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let approval = client
            .builder_fee_approval(
                "0x1234567890123456789012345678901234567890",
                "0xbuilder123456789012345678901234567890123",
            )
            .await
            .unwrap();

        assert!(!approval.approved);
        assert!(approval.max_fee_rate.is_none());
        assert!(approval.nonce.is_none());

        mock.assert_async().await;
    }

    // ========================================================================
    // Referral Tests
    // ========================================================================

    #[tokio::test]
    async fn test_referral() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "referral",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "referralCode": "ALPHA123",
                    "referredBy": "0xreferrer1234567890123456789012345678901234",
                    "cumulativeReferralDiscount": "1000.00",
                    "cumulativeReferralVlm": "5000000.00",
                    "referralCount": 25
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let referral = client
            .referral("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(referral.referral_code, Some("ALPHA123".to_string()));
        assert_eq!(
            referral.referred_by,
            Some("0xreferrer1234567890123456789012345678901234".to_string())
        );
        assert_eq!(
            referral.cumulative_referral_discount,
            Some("1000.00".to_string())
        );
        assert_eq!(
            referral.cumulative_referral_vlm,
            Some("5000000.00".to_string())
        );
        assert_eq!(referral.referral_count, Some(25));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_referral_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "referral",
                "user": "0xnewuser12345678901234567890123456789012345"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let referral = client
            .referral("0xnewuser12345678901234567890123456789012345")
            .await
            .unwrap();

        assert!(referral.referral_code.is_none());
        assert!(referral.referred_by.is_none());

        mock.assert_async().await;
    }

    // ========================================================================
    // HIP-3 State Tests
    // ========================================================================

    #[tokio::test]
    async fn test_hip3_state_enabled() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "hip3State",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "enabled": true,
                    "balance": "1000.00"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let hip3 = client
            .hip3_state("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert!(hip3.enabled);
        assert_eq!(hip3.balance, Some("1000.00".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_hip3_state_disabled() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "hip3State",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "enabled": false
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let hip3 = client
            .hip3_state("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert!(!hip3.enabled);
        assert!(hip3.balance.is_none());

        mock.assert_async().await;
    }

    // ========================================================================
    // Active Asset Data Tests
    // ========================================================================

    #[tokio::test]
    async fn test_active_asset_data() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "activeAssetData",
                "user": "0x1234567890123456789012345678901234567890",
                "coin": "BTC"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "user": "0x1234567890123456789012345678901234567890",
                    "coin": "BTC",
                    "leverage": {
                        "type": "cross",
                        "value": 10
                    },
                    "maxTradeSzs": ["1.5", "1.2"],
                    "availableToTrade": ["50000.00", "40000.00"],
                    "markPx": "50000.00"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let data = client
            .active_asset_data("0x1234567890123456789012345678901234567890", "BTC")
            .await
            .unwrap();

        assert_eq!(data.user, "0x1234567890123456789012345678901234567890");
        assert_eq!(data.coin, "BTC");
        assert_eq!(data.leverage.leverage_type, "cross");
        assert_eq!(data.leverage.value, 10);
        assert_eq!(data.max_trade_szs, ("1.5".to_string(), "1.2".to_string()));
        assert_eq!(
            data.available_to_trade,
            ("50000.00".to_string(), "40000.00".to_string())
        );
        assert_eq!(data.mark_px, "50000.00");

        mock.assert_async().await;
    }

    // ========================================================================
    // Perps at Open Interest Cap Tests
    // ========================================================================

    #[tokio::test]
    async fn test_perps_at_open_interest_cap() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "perpsAtOpenInterestCap"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"["BTC", "ETH", "SOL"]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let capped = client.perps_at_open_interest_cap(None).await.unwrap();

        assert_eq!(capped.len(), 3);
        assert_eq!(capped[0], "BTC");
        assert_eq!(capped[1], "ETH");
        assert_eq!(capped[2], "SOL");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_perps_at_open_interest_cap_with_dex() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "perpsAtOpenInterestCap",
                "dex": "hyperliquid"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"["BTC"]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let capped = client
            .perps_at_open_interest_cap(Some("hyperliquid"))
            .await
            .unwrap();

        assert_eq!(capped.len(), 1);
        assert_eq!(capped[0], "BTC");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_perps_at_open_interest_cap_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "perpsAtOpenInterestCap"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let capped = client.perps_at_open_interest_cap(None).await.unwrap();

        assert_eq!(capped.len(), 0);

        mock.assert_async().await;
    }

    // ========================================================================
    // Perp DEX Limits Tests
    // ========================================================================

    #[tokio::test]
    async fn test_perp_dex_limits() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "perpDexLimits",
                "dex": "hyperliquid"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "totalOiCap": "1000000000.00",
                    "oiSzCapPerPerp": "50000000.00",
                    "maxTransferNtl": "10000000.00",
                    "coinToOiCap": [
                        ["BTC", "100000000.00"],
                        ["ETH", "50000000.00"]
                    ]
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let limits = client.perp_dex_limits("hyperliquid").await.unwrap();

        assert_eq!(limits.total_oi_cap, "1000000000.00");
        assert_eq!(limits.oi_sz_cap_per_perp, "50000000.00");
        assert_eq!(limits.max_transfer_ntl, "10000000.00");
        assert_eq!(limits.coin_to_oi_cap.len(), 2);
        assert_eq!(
            limits.coin_to_oi_cap[0],
            ("BTC".to_string(), "100000000.00".to_string())
        );
        assert_eq!(
            limits.coin_to_oi_cap[1],
            ("ETH".to_string(), "50000000.00".to_string())
        );

        mock.assert_async().await;
    }

    // ========================================================================
    // Perp DEX Status Tests
    // ========================================================================

    #[tokio::test]
    async fn test_perp_dex_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "perpDexStatus",
                "dex": "hyperliquid"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "name": "Hyperliquid",
                    "isActive": true,
                    "numAssets": 50,
                    "totalOpenInterest": "500000000.00",
                    "dayVolume": "1000000000.00"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let status = client.perp_dex_status("hyperliquid").await.unwrap();

        assert_eq!(status.name, "Hyperliquid");
        assert!(status.is_active);
        assert_eq!(status.num_assets, Some(50));
        assert_eq!(status.total_open_interest, Some("500000000.00".to_string()));
        assert_eq!(status.day_volume, Some("1000000000.00".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_perp_dex_status_minimal() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "perpDexStatus",
                "dex": "custom-dex"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "name": "Custom DEX",
                    "isActive": false
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let status = client.perp_dex_status("custom-dex").await.unwrap();

        assert_eq!(status.name, "Custom DEX");
        assert!(!status.is_active);
        assert!(status.num_assets.is_none());
        assert!(status.total_open_interest.is_none());

        mock.assert_async().await;
    }

    // ========================================================================
    // Perp Deploy Auction Status Tests
    // ========================================================================

    #[tokio::test]
    async fn test_perp_deploy_auction_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "perpDeployAuctionStatus"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "startTimeSeconds": 1700000000,
                    "durationSeconds": 86400,
                    "startGas": "1000000000000000000",
                    "currentGas": "500000000000000000",
                    "endGas": "100000000000000000"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let auction = client.perp_deploy_auction_status().await.unwrap();

        assert_eq!(auction.start_time_seconds, 1700000000);
        assert_eq!(auction.duration_seconds, 86400);
        assert_eq!(auction.start_gas, "1000000000000000000");
        assert_eq!(auction.current_gas, "500000000000000000");
        assert_eq!(auction.end_gas, Some("100000000000000000".to_string()));

        mock.assert_async().await;
    }

    // ========================================================================
    // Error Handling Tests
    // ========================================================================

    #[tokio::test]
    async fn test_vault_details_error_handling() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .vault_details("0xvault1234567890123456789012345678901234567890")
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("500"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_vaults_api_error_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "User not found"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result: Result<Vec<UserVaultDeposit>> = client.user_vaults("invalid-address").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("User not found"));

        mock.assert_async().await;
    }

    // ========================================================================
    // Request Serialization Tests
    // ========================================================================

    #[tokio::test]
    async fn test_vault_details_request_serialization() {
        let request = VaultDetailsRequest::new("0xvault1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"vaultDetails","vaultAddress":"0xvault1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_user_vaults_request_serialization() {
        let request = UserVaultsRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"userVaults","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_builder_fee_approval_request_serialization() {
        let request = BuilderFeeApprovalRequest::new(
            "0x1234567890123456789012345678901234567890",
            "0xbuilder123456789012345678901234567890123",
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"builderFeeApproval\""));
        assert!(json.contains("\"user\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"builder\":\"0xbuilder123456789012345678901234567890123\""));
    }

    #[tokio::test]
    async fn test_referral_request_serialization() {
        let request = ReferralRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"referral","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_hip3_state_request_serialization() {
        let request = Hip3StateRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"hip3State","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_active_asset_data_request_serialization() {
        let request =
            ActiveAssetDataRequest::new("0x1234567890123456789012345678901234567890", "BTC");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"activeAssetData\""));
        assert!(json.contains("\"user\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"coin\":\"BTC\""));
    }

    #[tokio::test]
    async fn test_perps_at_open_interest_cap_request_serialization() {
        let request = PerpsAtOpenInterestCapRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"perpsAtOpenInterestCap"}"#);
    }

    #[tokio::test]
    async fn test_perp_dex_limits_request_serialization() {
        let request = PerpDexLimitsRequest::new("hyperliquid");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"perpDexLimits","dex":"hyperliquid"}"#);
    }

    #[tokio::test]
    async fn test_perp_dex_status_request_serialization() {
        let request = PerpDexStatusRequest::new("hyperliquid");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"perpDexStatus","dex":"hyperliquid"}"#);
    }

    #[tokio::test]
    async fn test_perp_deploy_auction_status_request_serialization() {
        let request = PerpDeployAuctionStatusRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"type":"perpDeployAuctionStatus"}"#);
    }

    // ========================================================================
    // Response Type Deserialization Tests
    // ========================================================================

    #[tokio::test]
    async fn test_vault_details_deserialization() {
        let json = r#"{
            "name": "Test Vault",
            "portfolioVlm": "100000.00",
            "leader": "0xleader123",
            "leaderCommission": "0.1"
        }"#;
        let details: VaultDetails = serde_json::from_str(json).unwrap();
        assert_eq!(details.name, "Test Vault");
        assert_eq!(details.portfolio_vlm, "100000.00");
        assert_eq!(details.leader, "0xleader123");
        assert_eq!(details.leader_commission, "0.1");
        assert!(details.description.is_none());
    }

    #[tokio::test]
    async fn test_user_vault_deposit_deserialization() {
        let json = r#"{
            "vaultAddress": "0xvault123",
            "equityShare": "1000.00"
        }"#;
        let deposit: UserVaultDeposit = serde_json::from_str(json).unwrap();
        assert_eq!(deposit.vault_address, "0xvault123");
        assert_eq!(deposit.equity_share, "1000.00");
        assert!(deposit.equity_fraction.is_none());
        assert!(deposit.value.is_none());
    }

    #[tokio::test]
    async fn test_builder_fee_approval_info_deserialization() {
        let json = r#"{
            "approved": true,
            "maxFeeRate": "0.001"
        }"#;
        let approval: BuilderFeeApprovalInfo = serde_json::from_str(json).unwrap();
        assert!(approval.approved);
        assert_eq!(approval.max_fee_rate, Some("0.001".to_string()));
        assert!(approval.nonce.is_none());
    }

    #[tokio::test]
    async fn test_referral_info_deserialization() {
        let json = r#"{
            "referralCode": "CODE123"
        }"#;
        let referral: ReferralInfo = serde_json::from_str(json).unwrap();
        assert_eq!(referral.referral_code, Some("CODE123".to_string()));
        assert!(referral.referred_by.is_none());
    }

    #[tokio::test]
    async fn test_hip3_state_deserialization() {
        let json = r#"{
            "enabled": true,
            "balance": "500.00"
        }"#;
        let hip3: Hip3State = serde_json::from_str(json).unwrap();
        assert!(hip3.enabled);
        assert_eq!(hip3.balance, Some("500.00".to_string()));
    }

    #[tokio::test]
    async fn test_perp_dex_status_deserialization() {
        let json = r#"{
            "name": "Test DEX",
            "isActive": true
        }"#;
        let status: PerpDexStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.name, "Test DEX");
        assert!(status.is_active);
        assert!(status.num_assets.is_none());
    }
}
