//! User account info endpoints
//!
//! This module provides methods for querying user account information from the Hyperliquid API.

use crate::client::Client;
use crate::error::Result;
use crate::types::{
    LedgerUpdate, Portfolio, PortfolioRequest, RateLimitInfo, RateLimitsRequest, Subaccount,
    SubaccountsRequest, UserFees, UserFeesRequest, UserNonFundingLedgerUpdatesRequest, UserRole,
    UserRoleRequest,
};

impl Client {
    /// Retrieve non-funding ledger updates for a user
    ///
    /// Returns ledger updates excluding funding payments within the specified time range.
    /// This includes deposits, withdrawals, transfers, and other account changes.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    /// * `start_time` - Start timestamp in milliseconds
    /// * `end_time` - Optional end timestamp in milliseconds
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let updates = client.user_non_funding_ledger_updates(
    ///     "0x1234...",
    ///     1700000000000,
    ///     Some(1700100000000)
    /// ).await?;
    /// for update in updates {
    ///     println!("{}: {} USDC", update.delta.delta_type, update.delta.usdc);
    /// }
    /// ```
    pub async fn user_non_funding_ledger_updates(
        &self,
        user: &str,
        start_time: u64,
        end_time: Option<u64>,
    ) -> Result<Vec<LedgerUpdate>> {
        let mut request = UserNonFundingLedgerUpdatesRequest::new(user, start_time);
        if let Some(et) = end_time {
            request = request.with_end_time(et);
        }
        self.post_info(&request).await
    }

    /// Retrieve rate limits for a user
    ///
    /// Returns the user's current rate limit status including cumulative volume
    /// and request counts.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let limits = client.rate_limits("0x1234...").await?;
    /// println!("Cumulative volume: {}", limits.cum_vlm);
    /// println!("Request IDs used: {}", limits.n_request_ids);
    /// ```
    pub async fn rate_limits(&self, user: &str) -> Result<RateLimitInfo> {
        let request = RateLimitsRequest::new(user);
        self.post_info(&request).await
    }

    /// Retrieve portfolio summary for a user
    ///
    /// Returns the user's portfolio including account value, margin usage,
    /// cumulative trading metrics, and PnL.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let portfolio = client.portfolio("0x1234...").await?;
    /// println!("Account value: {}", portfolio.account_value);
    /// println!("User PnL: {}", portfolio.user_pnl);
    /// ```
    pub async fn portfolio(&self, user: &str) -> Result<Portfolio> {
        let request = PortfolioRequest::new(user);
        self.post_info(&request).await
    }

    /// Retrieve fee structure for a user
    ///
    /// Returns the user's fee schedule including maker/taker rates,
    /// referral discounts, and volume-based fee tiers.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let fees = client.user_fees("0x1234...").await?;
    /// println!("Maker fee: {}", fees.fee_schedule.maker);
    /// println!("Taker fee: {}", fees.fee_schedule.taker);
    /// ```
    pub async fn user_fees(&self, user: &str) -> Result<UserFees> {
        let request = UserFeesRequest::new(user);
        self.post_info(&request).await
    }

    /// Retrieve account role for a user
    ///
    /// Returns the user's role information including VIP and market maker status.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let role = client.user_role("0x1234...").await?;
    /// println!("Is VIP: {}", role.is_vip);
    /// println!("Is MM: {}", role.is_mm);
    /// ```
    pub async fn user_role(&self, user: &str) -> Result<UserRole> {
        let request = UserRoleRequest::new(user);
        self.post_info(&request).await
    }

    /// Retrieve subaccounts for a user
    ///
    /// Returns a list of subaccounts associated with the master account.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address (master account)
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let subaccounts = client.subaccounts("0x1234...").await?;
    /// for sub in subaccounts {
    ///     println!("{}: {}", sub.name, sub.sub_account_user);
    /// }
    /// ```
    pub async fn subaccounts(&self, user: &str) -> Result<Vec<Subaccount>> {
        let request = SubaccountsRequest::new(user);
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

        async fn user_non_funding_ledger_updates(
            &self,
            user: &str,
            start_time: u64,
            end_time: Option<u64>,
        ) -> Result<Vec<LedgerUpdate>> {
            let mut request = UserNonFundingLedgerUpdatesRequest::new(user, start_time);
            if let Some(et) = end_time {
                request = request.with_end_time(et);
            }
            self.post_info(&request).await
        }

        async fn rate_limits(&self, user: &str) -> Result<RateLimitInfo> {
            let request = RateLimitsRequest::new(user);
            self.post_info(&request).await
        }

        async fn portfolio(&self, user: &str) -> Result<Portfolio> {
            let request = PortfolioRequest::new(user);
            self.post_info(&request).await
        }

        async fn user_fees(&self, user: &str) -> Result<UserFees> {
            let request = UserFeesRequest::new(user);
            self.post_info(&request).await
        }

        async fn user_role(&self, user: &str) -> Result<UserRole> {
            let request = UserRoleRequest::new(user);
            self.post_info(&request).await
        }

        async fn subaccounts(&self, user: &str) -> Result<Vec<Subaccount>> {
            let request = SubaccountsRequest::new(user);
            self.post_info(&request).await
        }
    }

    #[tokio::test]
    async fn test_user_non_funding_ledger_updates() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userNonFundingLedgerUpdates",
                "user": "0x1234567890123456789012345678901234567890",
                "startTime": 1700000000000_u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "hash": "0xabc123",
                        "time": 1700000001000,
                        "delta": {
                            "type": "deposit",
                            "usdc": "1000.0"
                        }
                    },
                    {
                        "hash": "0xdef456",
                        "time": 1700000002000,
                        "delta": {
                            "type": "withdraw",
                            "usdc": "-500.0",
                            "fee": "1.0",
                            "nonce": 12345,
                            "destination": "0xdest123"
                        }
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let updates = client
            .user_non_funding_ledger_updates(
                "0x1234567890123456789012345678901234567890",
                1700000000000,
                None,
            )
            .await
            .unwrap();

        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].hash, "0xabc123");
        assert_eq!(updates[0].delta.delta_type, "deposit");
        assert_eq!(updates[0].delta.usdc, "1000.0");
        assert_eq!(updates[1].hash, "0xdef456");
        assert_eq!(updates[1].delta.delta_type, "withdraw");
        assert_eq!(updates[1].delta.fee, Some("1.0".to_string()));
        assert_eq!(updates[1].delta.destination, Some("0xdest123".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_non_funding_ledger_updates_with_end_time() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userNonFundingLedgerUpdates",
                "user": "0x1234567890123456789012345678901234567890",
                "startTime": 1700000000000_u64,
                "endTime": 1700100000000_u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let updates = client
            .user_non_funding_ledger_updates(
                "0x1234567890123456789012345678901234567890",
                1700000000000,
                Some(1700100000000),
            )
            .await
            .unwrap();

        assert_eq!(updates.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_rate_limits() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "rateLimits",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "cumVlm": "1000000.0",
                    "nRequestIds": 500,
                    "nRequestWeights": 1000
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let limits = client
            .rate_limits("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(limits.cum_vlm, "1000000.0");
        assert_eq!(limits.n_request_ids, 500);
        assert_eq!(limits.n_request_weights, 1000);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_portfolio() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "portfolio",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "accountValue": "50000.0",
                    "totalMarginUsed": "10000.0",
                    "totalNtlPos": "40000.0",
                    "cumVlm": "1000000.0",
                    "cumTradingFee": "500.0",
                    "cumFunding": "100.0",
                    "cumReferralFee": "50.0",
                    "userPnl": "5000.0"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let portfolio = client
            .portfolio("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(portfolio.account_value, "50000.0");
        assert_eq!(portfolio.total_margin_used, "10000.0");
        assert_eq!(portfolio.total_ntl_pos, "40000.0");
        assert_eq!(portfolio.cum_vlm, "1000000.0");
        assert_eq!(portfolio.cum_trading_fee, "500.0");
        assert_eq!(portfolio.cum_funding, "100.0");
        assert_eq!(portfolio.user_pnl, "5000.0");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_fees() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFees",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "activeReferralDiscount": "0.1",
                    "feeSchedule": {
                        "taker": "0.00035",
                        "maker": "0.0001",
                        "addLiquidityRebate": "0.00002",
                        "referralDiscount": "0.1"
                    },
                    "userAddRate": "0.0001",
                    "userCrossRate": "0.00035"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fees = client
            .user_fees("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(fees.active_referral_discount, "0.1");
        assert_eq!(fees.fee_schedule.taker, "0.00035");
        assert_eq!(fees.fee_schedule.maker, "0.0001");
        assert_eq!(
            fees.fee_schedule.add_liquidity_rebate,
            Some("0.00002".to_string())
        );
        assert_eq!(fees.user_add_rate, "0.0001");
        assert_eq!(fees.user_cross_rate, "0.00035");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_fees_with_daily_vlm() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFees",
                "user": "0xabcdef1234567890123456789012345678901234"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "activeReferralDiscount": "0.0",
                    "dailyUserVlm": [
                        {
                            "date": "2024-01-15",
                            "exchangeVlm": "5000000.0",
                            "userVlm": "10000.0"
                        }
                    ],
                    "feeSchedule": {
                        "taker": "0.0005",
                        "maker": "0.0002"
                    },
                    "trialActive": true,
                    "userAddRate": "0.0002",
                    "userCrossRate": "0.0005"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fees = client
            .user_fees("0xabcdef1234567890123456789012345678901234")
            .await
            .unwrap();

        assert_eq!(fees.trial_active, Some(true));
        assert!(fees.daily_user_vlm.is_some());
        let daily_vlm = fees.daily_user_vlm.unwrap();
        assert_eq!(daily_vlm.len(), 1);
        assert_eq!(daily_vlm[0].date, "2024-01-15");
        assert_eq!(daily_vlm[0].exchange_vlm, "5000000.0");
        assert_eq!(daily_vlm[0].user_vlm, Some("10000.0".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_role() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userRole",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "role": "vip",
                    "isVip": true,
                    "isMm": false
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let role = client
            .user_role("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(role.role, Some("vip".to_string()));
        assert!(role.is_vip);
        assert!(!role.is_mm);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_role_market_maker() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userRole",
                "user": "0xabcdef1234567890123456789012345678901234"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "isVip": false,
                    "isMm": true
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let role = client
            .user_role("0xabcdef1234567890123456789012345678901234")
            .await
            .unwrap();

        assert_eq!(role.role, None);
        assert!(!role.is_vip);
        assert!(role.is_mm);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_subaccounts() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "subaccounts",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "subAccountUser": "0xsub1234567890123456789012345678901234",
                        "name": "Trading Sub 1",
                        "master": "0x1234567890123456789012345678901234567890",
                        "clearinghouseState": null
                    },
                    {
                        "subAccountUser": "0xsub2345678901234567890123456789012345",
                        "name": "Trading Sub 2",
                        "master": "0x1234567890123456789012345678901234567890",
                        "clearinghouseState": null
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let subaccounts = client
            .subaccounts("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(subaccounts.len(), 2);
        assert_eq!(
            subaccounts[0].sub_account_user,
            "0xsub1234567890123456789012345678901234"
        );
        assert_eq!(subaccounts[0].name, "Trading Sub 1");
        assert_eq!(
            subaccounts[0].master,
            "0x1234567890123456789012345678901234567890"
        );
        assert_eq!(subaccounts[1].name, "Trading Sub 2");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_subaccounts_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "subaccounts",
                "user": "0xdeadbeef12345678901234567890123456789012"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let subaccounts = client
            .subaccounts("0xdeadbeef12345678901234567890123456789012")
            .await
            .unwrap();

        assert_eq!(subaccounts.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_portfolio_error_handling() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .portfolio("0x1234567890123456789012345678901234567890")
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("500"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_rate_limits_api_error_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "User not found"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result: Result<RateLimitInfo> = client.rate_limits("invalid-address").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("User not found"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_non_funding_ledger_updates_request_serialization() {
        let request = UserNonFundingLedgerUpdatesRequest::new(
            "0x1234567890123456789012345678901234567890",
            1700000000000,
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"userNonFundingLedgerUpdates\""));
        assert!(json.contains("\"user\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"startTime\":1700000000000"));
    }

    #[tokio::test]
    async fn test_rate_limits_request_serialization() {
        let request = RateLimitsRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"rateLimits","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_portfolio_request_serialization() {
        let request = PortfolioRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"portfolio","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_user_fees_request_serialization() {
        let request = UserFeesRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"userFees","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_user_role_request_serialization() {
        let request = UserRoleRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"userRole","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_subaccounts_request_serialization() {
        let request = SubaccountsRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"subaccounts","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }
}
