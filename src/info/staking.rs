//! Staking info endpoints
//!
//! This module provides methods for querying staking information from the Hyperliquid API.

use crate::client::Client;
use crate::error::Result;
use crate::types::{
    StakingDelegation, StakingDelegationsRequest, StakingHistoryEntry, StakingHistoryRequest,
    StakingReward, StakingRewardsRequest, StakingSummary, StakingSummaryRequest,
};

impl Client {
    /// Retrieve staking delegations for a user
    ///
    /// Returns a list of all delegations the user has made to validators,
    /// including pending undelegations and lock times.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let delegations = client.staking_delegations("0x1234...").await?;
    /// for delegation in delegations {
    ///     println!("Validator: {}, Amount: {}", delegation.validator, delegation.amount);
    /// }
    /// ```
    pub async fn staking_delegations(&self, user: &str) -> Result<Vec<StakingDelegation>> {
        let request = StakingDelegationsRequest::new(user);
        self.post_info(&request).await
    }

    /// Retrieve staking summary for a user
    ///
    /// Returns an overview of the user's staking state including total staked amount,
    /// pending undelegations, and unclaimed rewards.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let summary = client.staking_summary("0x1234...").await?;
    /// println!("Total staked: {}", summary.staked);
    /// println!("Unclaimed rewards: {}", summary.unclaimed_rewards);
    /// ```
    pub async fn staking_summary(&self, user: &str) -> Result<StakingSummary> {
        let request = StakingSummaryRequest::new(user);
        self.post_info(&request).await
    }

    /// Retrieve staking history for a user
    ///
    /// Returns a list of staking-related actions the user has performed,
    /// including delegations, undelegations, and reward claims.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let history = client.staking_history("0x1234...").await?;
    /// for entry in history {
    ///     println!("{}: {} - {}", entry.time, entry.action_type, entry.amount);
    /// }
    /// ```
    pub async fn staking_history(&self, user: &str) -> Result<Vec<StakingHistoryEntry>> {
        let request = StakingHistoryRequest::new(user);
        self.post_info(&request).await
    }

    /// Retrieve staking rewards for a user
    ///
    /// Returns a list of rewards the user has earned from staking,
    /// organized by validator.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let rewards = client.staking_rewards("0x1234...").await?;
    /// for reward in rewards {
    ///     println!("Validator: {}, Reward: {}", reward.validator, reward.amount);
    /// }
    /// ```
    pub async fn staking_rewards(&self, user: &str) -> Result<Vec<StakingReward>> {
        let request = StakingRewardsRequest::new(user);
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

        async fn staking_delegations(&self, user: &str) -> Result<Vec<StakingDelegation>> {
            let request = StakingDelegationsRequest::new(user);
            self.post_info(&request).await
        }

        async fn staking_summary(&self, user: &str) -> Result<StakingSummary> {
            let request = StakingSummaryRequest::new(user);
            self.post_info(&request).await
        }

        async fn staking_history(&self, user: &str) -> Result<Vec<StakingHistoryEntry>> {
            let request = StakingHistoryRequest::new(user);
            self.post_info(&request).await
        }

        async fn staking_rewards(&self, user: &str) -> Result<Vec<StakingReward>> {
            let request = StakingRewardsRequest::new(user);
            self.post_info(&request).await
        }
    }

    #[tokio::test]
    async fn test_staking_delegations() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "stakingDelegations",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "validator": "0xvalidator1234567890123456789012345678901234",
                        "amount": "1000000000000000000",
                        "pendingUndelegation": "0",
                        "lockedUntil": null
                    },
                    {
                        "validator": "0xvalidator2345678901234567890123456789012345",
                        "amount": "500000000000000000",
                        "pendingUndelegation": "100000000000000000",
                        "lockedUntil": 1700100000000
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let delegations = client
            .staking_delegations("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(delegations.len(), 2);
        assert_eq!(
            delegations[0].validator,
            "0xvalidator1234567890123456789012345678901234"
        );
        assert_eq!(delegations[0].amount, "1000000000000000000");
        assert_eq!(delegations[0].pending_undelegation, Some("0".to_string()));
        assert!(delegations[0].locked_until.is_none());
        assert_eq!(
            delegations[1].validator,
            "0xvalidator2345678901234567890123456789012345"
        );
        assert_eq!(
            delegations[1].pending_undelegation,
            Some("100000000000000000".to_string())
        );
        assert_eq!(delegations[1].locked_until, Some(1700100000000));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_staking_delegations_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "stakingDelegations",
                "user": "0xdeadbeef12345678901234567890123456789012"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let delegations = client
            .staking_delegations("0xdeadbeef12345678901234567890123456789012")
            .await
            .unwrap();

        assert_eq!(delegations.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_staking_summary() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "stakingSummary",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "staked": "5000000000000000000",
                    "pendingUndelegation": "100000000000000000",
                    "unclaimedRewards": "50000000000000000",
                    "totalClaimedRewards": "200000000000000000"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let summary = client
            .staking_summary("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(summary.staked, "5000000000000000000");
        assert_eq!(summary.pending_undelegation, "100000000000000000");
        assert_eq!(summary.unclaimed_rewards, "50000000000000000");
        assert_eq!(
            summary.total_claimed_rewards,
            Some("200000000000000000".to_string())
        );

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_staking_summary_minimal() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "stakingSummary",
                "user": "0xabcdef1234567890123456789012345678901234"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "staked": "0",
                    "pendingUndelegation": "0",
                    "unclaimedRewards": "0"
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let summary = client
            .staking_summary("0xabcdef1234567890123456789012345678901234")
            .await
            .unwrap();

        assert_eq!(summary.staked, "0");
        assert_eq!(summary.pending_undelegation, "0");
        assert_eq!(summary.unclaimed_rewards, "0");
        assert!(summary.total_claimed_rewards.is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_staking_history() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "stakingHistory",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "hash": "0xhash1234567890abcdef",
                        "time": 1700000000000,
                        "type": "delegate",
                        "amount": "1000000000000000000",
                        "validator": "0xvalidator1234567890123456789012345678901234"
                    },
                    {
                        "hash": "0xhash2345678901abcdef",
                        "time": 1700050000000,
                        "type": "claim",
                        "amount": "50000000000000000"
                    },
                    {
                        "hash": "0xhash3456789012abcdef",
                        "time": 1700100000000,
                        "type": "undelegate",
                        "amount": "500000000000000000",
                        "validator": "0xvalidator1234567890123456789012345678901234"
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let history = client
            .staking_history("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(history.len(), 3);

        assert_eq!(history[0].hash, "0xhash1234567890abcdef");
        assert_eq!(history[0].time, 1700000000000);
        assert_eq!(history[0].action_type, "delegate");
        assert_eq!(history[0].amount, "1000000000000000000");
        assert_eq!(
            history[0].validator,
            Some("0xvalidator1234567890123456789012345678901234".to_string())
        );

        assert_eq!(history[1].action_type, "claim");
        assert!(history[1].validator.is_none());

        assert_eq!(history[2].action_type, "undelegate");
        assert!(history[2].validator.is_some());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_staking_history_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "stakingHistory",
                "user": "0xdeadbeef12345678901234567890123456789012"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let history = client
            .staking_history("0xdeadbeef12345678901234567890123456789012")
            .await
            .unwrap();

        assert_eq!(history.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_staking_rewards() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "stakingRewards",
                "user": "0x1234567890123456789012345678901234567890"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "validator": "0xvalidator1234567890123456789012345678901234",
                        "amount": "25000000000000000",
                        "time": 1700000000000
                    },
                    {
                        "validator": "0xvalidator1234567890123456789012345678901234",
                        "amount": "25000000000000000",
                        "time": 1700100000000
                    },
                    {
                        "validator": "0xvalidator2345678901234567890123456789012345",
                        "amount": "10000000000000000",
                        "time": 1700050000000
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let rewards = client
            .staking_rewards("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(rewards.len(), 3);

        assert_eq!(
            rewards[0].validator,
            "0xvalidator1234567890123456789012345678901234"
        );
        assert_eq!(rewards[0].amount, "25000000000000000");
        assert_eq!(rewards[0].time, 1700000000000);

        assert_eq!(
            rewards[1].validator,
            "0xvalidator1234567890123456789012345678901234"
        );
        assert_eq!(rewards[1].time, 1700100000000);

        assert_eq!(
            rewards[2].validator,
            "0xvalidator2345678901234567890123456789012345"
        );

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_staking_rewards_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "stakingRewards",
                "user": "0xdeadbeef12345678901234567890123456789012"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let rewards = client
            .staking_rewards("0xdeadbeef12345678901234567890123456789012")
            .await
            .unwrap();

        assert_eq!(rewards.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_staking_delegations_error_handling() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .staking_delegations("0x1234567890123456789012345678901234567890")
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Api(_)));
        assert!(err.to_string().contains("500"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_staking_summary_api_error_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "User not found"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result: Result<StakingSummary> = client.staking_summary("invalid-address").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("User not found"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_staking_delegations_request_serialization() {
        let request = StakingDelegationsRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"stakingDelegations","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_staking_summary_request_serialization() {
        let request = StakingSummaryRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"stakingSummary","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_staking_history_request_serialization() {
        let request = StakingHistoryRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"stakingHistory","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_staking_rewards_request_serialization() {
        let request = StakingRewardsRequest::new("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"stakingRewards","user":"0x1234567890123456789012345678901234567890"}"#
        );
    }

    #[tokio::test]
    async fn test_staking_delegation_deserialization() {
        let json = r#"{
            "validator": "0xvalidator1234567890123456789012345678901234",
            "amount": "1000000000000000000"
        }"#;
        let delegation: StakingDelegation = serde_json::from_str(json).unwrap();
        assert_eq!(
            delegation.validator,
            "0xvalidator1234567890123456789012345678901234"
        );
        assert_eq!(delegation.amount, "1000000000000000000");
        assert!(delegation.pending_undelegation.is_none());
        assert!(delegation.locked_until.is_none());
    }

    #[tokio::test]
    async fn test_staking_summary_deserialization() {
        let json = r#"{
            "staked": "5000000000000000000",
            "pendingUndelegation": "0",
            "unclaimedRewards": "100000000000000000"
        }"#;
        let summary: StakingSummary = serde_json::from_str(json).unwrap();
        assert_eq!(summary.staked, "5000000000000000000");
        assert_eq!(summary.pending_undelegation, "0");
        assert_eq!(summary.unclaimed_rewards, "100000000000000000");
        assert!(summary.total_claimed_rewards.is_none());
    }

    #[tokio::test]
    async fn test_staking_history_entry_deserialization() {
        let json = r#"{
            "hash": "0xhash1234567890",
            "time": 1700000000000,
            "type": "delegate",
            "amount": "1000000000000000000",
            "validator": "0xvalidator123"
        }"#;
        let entry: StakingHistoryEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.hash, "0xhash1234567890");
        assert_eq!(entry.time, 1700000000000);
        assert_eq!(entry.action_type, "delegate");
        assert_eq!(entry.amount, "1000000000000000000");
        assert_eq!(entry.validator, Some("0xvalidator123".to_string()));
    }

    #[tokio::test]
    async fn test_staking_reward_deserialization() {
        let json = r#"{
            "validator": "0xvalidator123",
            "amount": "50000000000000000",
            "time": 1700000000000
        }"#;
        let reward: StakingReward = serde_json::from_str(json).unwrap();
        assert_eq!(reward.validator, "0xvalidator123");
        assert_eq!(reward.amount, "50000000000000000");
        assert_eq!(reward.time, 1700000000000);
    }
}
