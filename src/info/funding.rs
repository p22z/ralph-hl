//! Funding info endpoints
//!
//! This module provides methods for querying funding rate information from the Hyperliquid API,
//! including user funding history, historical funding rates, and predicted funding rates.

use std::collections::HashMap;

use crate::client::Client;
use crate::error::Result;
use crate::types::{
    FundingHistoryEntry, FundingHistoryRequest, HistoricalFundingRate, PredictedFunding,
    PredictedFundingsRequest, UserFundingRequest,
};

impl Client {
    /// Retrieve user funding history
    ///
    /// Returns the funding payments received or paid by the user within the specified time range.
    /// Funding payments are periodic adjustments based on the difference between the perpetual
    /// mark price and the underlying spot price.
    ///
    /// # Arguments
    /// * `user` - The user's Ethereum address
    /// * `start_time` - Start time in milliseconds since Unix epoch
    /// * `end_time` - Optional end time in milliseconds since Unix epoch. If None, returns records up to now.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let now = std::time::SystemTime::now()
    ///     .duration_since(std::time::UNIX_EPOCH)
    ///     .unwrap()
    ///     .as_millis() as u64;
    /// let week_ago = now - 7 * 24 * 60 * 60 * 1000;
    /// let fundings = client.user_funding("0x1234...", week_ago, Some(now)).await?;
    /// for entry in fundings {
    ///     println!("{}: {} @ {}", entry.delta.coin, entry.delta.funding_rate, entry.time);
    /// }
    /// ```
    pub async fn user_funding(
        &self,
        user: &str,
        start_time: u64,
        end_time: Option<u64>,
    ) -> Result<Vec<FundingHistoryEntry>> {
        let mut request = UserFundingRequest::new(user, start_time);
        if let Some(t) = end_time {
            request = request.with_end_time(t);
        }
        self.post_info(&request).await
    }

    /// Retrieve historical funding rates for a coin
    ///
    /// Returns the historical funding rates for a specific perpetual contract
    /// within the specified time range.
    ///
    /// # Arguments
    /// * `coin` - The asset symbol (e.g., "BTC", "ETH")
    /// * `start_time` - Start time in milliseconds since Unix epoch
    /// * `end_time` - Optional end time in milliseconds since Unix epoch. If None, returns records up to now.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let now = std::time::SystemTime::now()
    ///     .duration_since(std::time::UNIX_EPOCH)
    ///     .unwrap()
    ///     .as_millis() as u64;
    /// let day_ago = now - 24 * 60 * 60 * 1000;
    /// let rates = client.funding_history("BTC", day_ago, Some(now)).await?;
    /// for rate in rates {
    ///     println!("{}: funding_rate={} premium={} @ {}", rate.coin, rate.funding_rate, rate.premium, rate.time);
    /// }
    /// ```
    pub async fn funding_history(
        &self,
        coin: &str,
        start_time: u64,
        end_time: Option<u64>,
    ) -> Result<Vec<HistoricalFundingRate>> {
        let mut request = FundingHistoryRequest::new(coin, start_time);
        if let Some(t) = end_time {
            request = request.with_end_time(t);
        }
        self.post_info(&request).await
    }

    /// Retrieve predicted funding rates for all perpetuals
    ///
    /// Returns the predicted next funding rates and their scheduled times for all
    /// perpetual contracts. This helps traders anticipate upcoming funding payments.
    ///
    /// # Example
    /// ```ignore
    /// let client = Client::mainnet()?;
    /// let predictions = client.predicted_fundings().await?;
    /// for (coin, funding) in predictions {
    ///     println!("{}: rate={} next_time={}", coin, funding.funding_rate, funding.next_funding_time);
    /// }
    /// ```
    pub async fn predicted_fundings(&self) -> Result<HashMap<String, PredictedFunding>> {
        let request = PredictedFundingsRequest::default();
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

        async fn user_funding(
            &self,
            user: &str,
            start_time: u64,
            end_time: Option<u64>,
        ) -> Result<Vec<FundingHistoryEntry>> {
            let mut request = UserFundingRequest::new(user, start_time);
            if let Some(t) = end_time {
                request = request.with_end_time(t);
            }
            self.post_info(&request).await
        }

        async fn funding_history(
            &self,
            coin: &str,
            start_time: u64,
            end_time: Option<u64>,
        ) -> Result<Vec<HistoricalFundingRate>> {
            let mut request = FundingHistoryRequest::new(coin, start_time);
            if let Some(t) = end_time {
                request = request.with_end_time(t);
            }
            self.post_info(&request).await
        }

        async fn predicted_fundings(&self) -> Result<HashMap<String, PredictedFunding>> {
            let request = PredictedFundingsRequest::default();
            self.post_info(&request).await
        }
    }

    // ==================== User Funding Tests ====================

    #[tokio::test]
    async fn test_user_funding_without_end_time() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFunding",
                "user": "0x1234567890123456789012345678901234567890",
                "startTime": 1700000000000u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "delta": {
                            "coin": "BTC",
                            "fundingRate": "0.0001",
                            "szi": "1.5",
                            "type": "funding",
                            "usdc": "5.25"
                        },
                        "hash": "0xabc123def456",
                        "time": 1700000100000
                    },
                    {
                        "delta": {
                            "coin": "ETH",
                            "fundingRate": "-0.00005",
                            "szi": "-2.0",
                            "type": "funding",
                            "usdc": "-1.50",
                            "nSamples": 100
                        },
                        "hash": "0x789abc012345",
                        "time": 1700000200000
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fundings = client
            .user_funding(
                "0x1234567890123456789012345678901234567890",
                1700000000000,
                None,
            )
            .await
            .unwrap();

        assert_eq!(fundings.len(), 2);
        assert_eq!(fundings[0].delta.coin, "BTC");
        assert_eq!(fundings[0].delta.funding_rate, "0.0001");
        assert_eq!(fundings[0].delta.szi, "1.5");
        assert_eq!(fundings[0].delta.delta_type, "funding");
        assert_eq!(fundings[0].delta.usdc, "5.25");
        assert_eq!(fundings[0].delta.n_samples, None);
        assert_eq!(fundings[0].hash, "0xabc123def456");
        assert_eq!(fundings[0].time, 1700000100000);

        assert_eq!(fundings[1].delta.coin, "ETH");
        assert_eq!(fundings[1].delta.funding_rate, "-0.00005");
        assert_eq!(fundings[1].delta.szi, "-2.0");
        assert_eq!(fundings[1].delta.usdc, "-1.50");
        assert_eq!(fundings[1].delta.n_samples, Some(100));
        assert_eq!(fundings[1].time, 1700000200000);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_funding_with_end_time() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFunding",
                "user": "0x1234567890123456789012345678901234567890",
                "startTime": 1700000000000u64,
                "endTime": 1700100000000u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "delta": {
                            "coin": "BTC",
                            "fundingRate": "0.0002",
                            "szi": "0.5",
                            "type": "funding",
                            "usdc": "2.00"
                        },
                        "hash": "0xdef789",
                        "time": 1700050000000
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fundings = client
            .user_funding(
                "0x1234567890123456789012345678901234567890",
                1700000000000,
                Some(1700100000000),
            )
            .await
            .unwrap();

        assert_eq!(fundings.len(), 1);
        assert_eq!(fundings[0].delta.coin, "BTC");
        assert_eq!(fundings[0].delta.funding_rate, "0.0002");
        assert_eq!(fundings[0].time, 1700050000000);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_funding_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFunding",
                "user": "0xdeadbeef12345678901234567890123456789012",
                "startTime": 1700000000000u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let fundings = client
            .user_funding(
                "0xdeadbeef12345678901234567890123456789012",
                1700000000000,
                None,
            )
            .await
            .unwrap();

        assert_eq!(fundings.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_funding_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFunding",
                "user": "invalid_address",
                "startTime": 1700000000000u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid user address format"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .user_funding("invalid_address", 1700000000000, None)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert_eq!(msg, "Invalid user address format"),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_user_funding_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "userFunding",
                "user": "0x1234567890123456789012345678901234567890",
                "startTime": 1700000000000u64
            })))
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client
            .user_funding(
                "0x1234567890123456789012345678901234567890",
                1700000000000,
                None,
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert!(msg.contains("500")),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    // ==================== Funding History Tests ====================

    #[tokio::test]
    async fn test_funding_history_without_end_time() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "fundingHistory",
                "coin": "BTC",
                "startTime": 1700000000000u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "coin": "BTC",
                        "fundingRate": "0.0001",
                        "premium": "0.00005",
                        "time": 1700003600000
                    },
                    {
                        "coin": "BTC",
                        "fundingRate": "0.00012",
                        "premium": "0.00006",
                        "time": 1700007200000
                    },
                    {
                        "coin": "BTC",
                        "fundingRate": "-0.00008",
                        "premium": "-0.00004",
                        "time": 1700010800000
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let rates = client.funding_history("BTC", 1700000000000, None).await.unwrap();

        assert_eq!(rates.len(), 3);
        assert_eq!(rates[0].coin, "BTC");
        assert_eq!(rates[0].funding_rate, "0.0001");
        assert_eq!(rates[0].premium, "0.00005");
        assert_eq!(rates[0].time, 1700003600000);

        assert_eq!(rates[1].funding_rate, "0.00012");
        assert_eq!(rates[1].premium, "0.00006");
        assert_eq!(rates[1].time, 1700007200000);

        assert_eq!(rates[2].funding_rate, "-0.00008");
        assert_eq!(rates[2].premium, "-0.00004");
        assert_eq!(rates[2].time, 1700010800000);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_funding_history_with_end_time() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "fundingHistory",
                "coin": "ETH",
                "startTime": 1700000000000u64,
                "endTime": 1700100000000u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                    {
                        "coin": "ETH",
                        "fundingRate": "0.00015",
                        "premium": "0.00008",
                        "time": 1700050000000
                    }
                ]"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let rates = client
            .funding_history("ETH", 1700000000000, Some(1700100000000))
            .await
            .unwrap();

        assert_eq!(rates.len(), 1);
        assert_eq!(rates[0].coin, "ETH");
        assert_eq!(rates[0].funding_rate, "0.00015");
        assert_eq!(rates[0].premium, "0.00008");
        assert_eq!(rates[0].time, 1700050000000);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_funding_history_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "fundingHistory",
                "coin": "UNKNOWN",
                "startTime": 1700000000000u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let rates = client
            .funding_history("UNKNOWN", 1700000000000, None)
            .await
            .unwrap();

        assert_eq!(rates.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_funding_history_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "fundingHistory",
                "coin": "",
                "startTime": 1700000000000u64
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid coin parameter"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.funding_history("", 1700000000000, None).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert_eq!(msg, "Invalid coin parameter"),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_funding_history_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "fundingHistory",
                "coin": "BTC",
                "startTime": 1700000000000u64
            })))
            .with_status(503)
            .with_body("Service Unavailable")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.funding_history("BTC", 1700000000000, None).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert!(msg.contains("503")),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    // ==================== Predicted Fundings Tests ====================

    #[tokio::test]
    async fn test_predicted_fundings() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "predictedFundings"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "BTC": {
                        "fundingRate": "0.0001",
                        "nextFundingTime": 1700003600000
                    },
                    "ETH": {
                        "fundingRate": "-0.00005",
                        "nextFundingTime": 1700003600000
                    },
                    "SOL": {
                        "fundingRate": "0.00025",
                        "nextFundingTime": 1700003600000
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let predictions = client.predicted_fundings().await.unwrap();

        assert_eq!(predictions.len(), 3);

        let btc = predictions.get("BTC").unwrap();
        assert_eq!(btc.funding_rate, "0.0001");
        assert_eq!(btc.next_funding_time, 1700003600000);

        let eth = predictions.get("ETH").unwrap();
        assert_eq!(eth.funding_rate, "-0.00005");
        assert_eq!(eth.next_funding_time, 1700003600000);

        let sol = predictions.get("SOL").unwrap();
        assert_eq!(sol.funding_rate, "0.00025");
        assert_eq!(sol.next_funding_time, 1700003600000);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_predicted_fundings_single_coin() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "predictedFundings"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "BTC": {
                        "fundingRate": "0.00015",
                        "nextFundingTime": 1700007200000
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let predictions = client.predicted_fundings().await.unwrap();

        assert_eq!(predictions.len(), 1);
        let btc = predictions.get("BTC").unwrap();
        assert_eq!(btc.funding_rate, "0.00015");
        assert_eq!(btc.next_funding_time, 1700007200000);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_predicted_fundings_empty() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "predictedFundings"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let predictions = client.predicted_fundings().await.unwrap();

        assert_eq!(predictions.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_predicted_fundings_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "predictedFundings"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Service temporarily unavailable"}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.predicted_fundings().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert_eq!(msg, "Service temporarily unavailable"),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_predicted_fundings_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "predictedFundings"
            })))
            .with_status(429)
            .with_body("Rate Limited")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let result = client.predicted_fundings().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => assert!(msg.contains("429")),
            _ => panic!("Expected Api error"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_predicted_fundings_with_negative_rates() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/info")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({
                "type": "predictedFundings"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "BTC": {
                        "fundingRate": "-0.0003",
                        "nextFundingTime": 1700003600000
                    },
                    "ETH": {
                        "fundingRate": "-0.0001",
                        "nextFundingTime": 1700003600000
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let predictions = client.predicted_fundings().await.unwrap();

        assert_eq!(predictions.len(), 2);

        let btc = predictions.get("BTC").unwrap();
        assert_eq!(btc.funding_rate, "-0.0003");

        let eth = predictions.get("ETH").unwrap();
        assert_eq!(eth.funding_rate, "-0.0001");

        mock.assert_async().await;
    }
}
