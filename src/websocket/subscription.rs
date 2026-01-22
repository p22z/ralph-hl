//! Subscription management for Hyperliquid WebSocket
//!
//! This module provides subscription types and management for the Hyperliquid
//! WebSocket API. It supports all subscription types including market data,
//! user data, and user event subscriptions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::CandleInterval;

// ============================================================================
// Subscription Types
// ============================================================================

/// Subscription type for WebSocket channels
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Subscription {
    /// All mid prices subscription
    #[serde(rename = "allMids")]
    AllMids {
        #[serde(skip_serializing_if = "Option::is_none")]
        dex: Option<String>,
    },

    /// User notifications
    #[serde(rename = "notification")]
    Notification { user: String },

    /// Aggregate user data (webData3)
    #[serde(rename = "webData3")]
    WebData3 { user: String },

    /// TWAP order states
    #[serde(rename = "twapStates")]
    TwapStates {
        user: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        dex: Option<String>,
    },

    /// Clearinghouse state (account positions/margins)
    #[serde(rename = "clearinghouseState")]
    ClearinghouseState {
        user: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        dex: Option<String>,
    },

    /// Open orders
    #[serde(rename = "openOrders")]
    OpenOrders {
        user: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        dex: Option<String>,
    },

    /// Candlestick data
    #[serde(rename = "candle")]
    Candle {
        coin: String,
        interval: CandleInterval,
    },

    /// L2 order book
    #[serde(rename = "l2Book")]
    L2Book {
        coin: String,
        #[serde(rename = "nSigFigs", skip_serializing_if = "Option::is_none")]
        n_sig_figs: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mantissa: Option<u8>,
    },

    /// Trade stream
    #[serde(rename = "trades")]
    Trades { coin: String },

    /// Order status updates
    #[serde(rename = "orderUpdates")]
    OrderUpdates { user: String },

    /// User events (fills, funding, liquidation)
    #[serde(rename = "userEvents")]
    UserEvents { user: String },

    /// User fills
    #[serde(rename = "userFills")]
    UserFills {
        user: String,
        #[serde(rename = "aggregateByTime", skip_serializing_if = "Option::is_none")]
        aggregate_by_time: Option<bool>,
    },

    /// User funding payments
    #[serde(rename = "userFundings")]
    UserFundings { user: String },

    /// User non-funding ledger updates
    #[serde(rename = "userNonFundingLedgerUpdates")]
    UserNonFundingLedgerUpdates { user: String },

    /// Active asset context (market data)
    #[serde(rename = "activeAssetCtx")]
    ActiveAssetCtx { coin: String },

    /// Active asset data (user-specific, perps only)
    #[serde(rename = "activeAssetData")]
    ActiveAssetData { user: String, coin: String },

    /// User TWAP slice fills
    #[serde(rename = "userTwapSliceFills")]
    UserTwapSliceFills { user: String },

    /// User TWAP history
    #[serde(rename = "userTwapHistory")]
    UserTwapHistory { user: String },

    /// Best bid/offer
    #[serde(rename = "bbo")]
    Bbo { coin: String },
}

impl Subscription {
    // ========== Market Data Subscriptions ==========

    /// Create an AllMids subscription
    pub fn all_mids() -> Self {
        Subscription::AllMids { dex: None }
    }

    /// Create an AllMids subscription with a specific DEX
    pub fn all_mids_with_dex(dex: impl Into<String>) -> Self {
        Subscription::AllMids {
            dex: Some(dex.into()),
        }
    }

    /// Create an L2Book subscription
    pub fn l2_book(coin: impl Into<String>) -> Self {
        Subscription::L2Book {
            coin: coin.into(),
            n_sig_figs: None,
            mantissa: None,
        }
    }

    /// Create an L2Book subscription with aggregation parameters
    pub fn l2_book_with_params(
        coin: impl Into<String>,
        n_sig_figs: Option<u8>,
        mantissa: Option<u8>,
    ) -> Self {
        Subscription::L2Book {
            coin: coin.into(),
            n_sig_figs,
            mantissa,
        }
    }

    /// Create a Trades subscription
    pub fn trades(coin: impl Into<String>) -> Self {
        Subscription::Trades { coin: coin.into() }
    }

    /// Create a Candle subscription
    pub fn candle(coin: impl Into<String>, interval: CandleInterval) -> Self {
        Subscription::Candle {
            coin: coin.into(),
            interval,
        }
    }

    /// Create a BBO subscription
    pub fn bbo(coin: impl Into<String>) -> Self {
        Subscription::Bbo { coin: coin.into() }
    }

    /// Create an ActiveAssetCtx subscription
    pub fn active_asset_ctx(coin: impl Into<String>) -> Self {
        Subscription::ActiveAssetCtx { coin: coin.into() }
    }

    // ========== User Data Subscriptions ==========

    /// Create a Notification subscription
    pub fn notification(user: impl Into<String>) -> Self {
        Subscription::Notification { user: user.into() }
    }

    /// Create a WebData3 subscription
    pub fn web_data3(user: impl Into<String>) -> Self {
        Subscription::WebData3 { user: user.into() }
    }

    /// Create a TwapStates subscription
    pub fn twap_states(user: impl Into<String>) -> Self {
        Subscription::TwapStates {
            user: user.into(),
            dex: None,
        }
    }

    /// Create a TwapStates subscription with a specific DEX
    pub fn twap_states_with_dex(user: impl Into<String>, dex: impl Into<String>) -> Self {
        Subscription::TwapStates {
            user: user.into(),
            dex: Some(dex.into()),
        }
    }

    /// Create a ClearinghouseState subscription
    pub fn clearinghouse_state(user: impl Into<String>) -> Self {
        Subscription::ClearinghouseState {
            user: user.into(),
            dex: None,
        }
    }

    /// Create a ClearinghouseState subscription with a specific DEX
    pub fn clearinghouse_state_with_dex(user: impl Into<String>, dex: impl Into<String>) -> Self {
        Subscription::ClearinghouseState {
            user: user.into(),
            dex: Some(dex.into()),
        }
    }

    /// Create an OpenOrders subscription
    pub fn open_orders(user: impl Into<String>) -> Self {
        Subscription::OpenOrders {
            user: user.into(),
            dex: None,
        }
    }

    /// Create an OpenOrders subscription with a specific DEX
    pub fn open_orders_with_dex(user: impl Into<String>, dex: impl Into<String>) -> Self {
        Subscription::OpenOrders {
            user: user.into(),
            dex: Some(dex.into()),
        }
    }

    // ========== User Event Subscriptions ==========

    /// Create an OrderUpdates subscription
    pub fn order_updates(user: impl Into<String>) -> Self {
        Subscription::OrderUpdates { user: user.into() }
    }

    /// Create a UserEvents subscription
    pub fn user_events(user: impl Into<String>) -> Self {
        Subscription::UserEvents { user: user.into() }
    }

    /// Create a UserFills subscription
    pub fn user_fills(user: impl Into<String>) -> Self {
        Subscription::UserFills {
            user: user.into(),
            aggregate_by_time: None,
        }
    }

    /// Create a UserFills subscription with aggregation option
    pub fn user_fills_with_aggregation(user: impl Into<String>, aggregate_by_time: bool) -> Self {
        Subscription::UserFills {
            user: user.into(),
            aggregate_by_time: Some(aggregate_by_time),
        }
    }

    /// Create a UserFundings subscription
    pub fn user_fundings(user: impl Into<String>) -> Self {
        Subscription::UserFundings { user: user.into() }
    }

    /// Create a UserNonFundingLedgerUpdates subscription
    pub fn user_non_funding_ledger_updates(user: impl Into<String>) -> Self {
        Subscription::UserNonFundingLedgerUpdates { user: user.into() }
    }

    /// Create an ActiveAssetData subscription
    pub fn active_asset_data(user: impl Into<String>, coin: impl Into<String>) -> Self {
        Subscription::ActiveAssetData {
            user: user.into(),
            coin: coin.into(),
        }
    }

    /// Create a UserTwapSliceFills subscription
    pub fn user_twap_slice_fills(user: impl Into<String>) -> Self {
        Subscription::UserTwapSliceFills { user: user.into() }
    }

    /// Create a UserTwapHistory subscription
    pub fn user_twap_history(user: impl Into<String>) -> Self {
        Subscription::UserTwapHistory { user: user.into() }
    }

    /// Get the subscription channel name (type)
    pub fn channel_name(&self) -> &'static str {
        match self {
            Subscription::AllMids { .. } => "allMids",
            Subscription::Notification { .. } => "notification",
            Subscription::WebData3 { .. } => "webData3",
            Subscription::TwapStates { .. } => "twapStates",
            Subscription::ClearinghouseState { .. } => "clearinghouseState",
            Subscription::OpenOrders { .. } => "openOrders",
            Subscription::Candle { .. } => "candle",
            Subscription::L2Book { .. } => "l2Book",
            Subscription::Trades { .. } => "trades",
            Subscription::OrderUpdates { .. } => "orderUpdates",
            Subscription::UserEvents { .. } => "userEvents",
            Subscription::UserFills { .. } => "userFills",
            Subscription::UserFundings { .. } => "userFundings",
            Subscription::UserNonFundingLedgerUpdates { .. } => "userNonFundingLedgerUpdates",
            Subscription::ActiveAssetCtx { .. } => "activeAssetCtx",
            Subscription::ActiveAssetData { .. } => "activeAssetData",
            Subscription::UserTwapSliceFills { .. } => "userTwapSliceFills",
            Subscription::UserTwapHistory { .. } => "userTwapHistory",
            Subscription::Bbo { .. } => "bbo",
        }
    }

    /// Check if this is a user-specific subscription
    pub fn is_user_subscription(&self) -> bool {
        matches!(
            self,
            Subscription::Notification { .. }
                | Subscription::WebData3 { .. }
                | Subscription::TwapStates { .. }
                | Subscription::ClearinghouseState { .. }
                | Subscription::OpenOrders { .. }
                | Subscription::OrderUpdates { .. }
                | Subscription::UserEvents { .. }
                | Subscription::UserFills { .. }
                | Subscription::UserFundings { .. }
                | Subscription::UserNonFundingLedgerUpdates { .. }
                | Subscription::ActiveAssetData { .. }
                | Subscription::UserTwapSliceFills { .. }
                | Subscription::UserTwapHistory { .. }
        )
    }

    /// Check if this is a market data subscription
    pub fn is_market_subscription(&self) -> bool {
        matches!(
            self,
            Subscription::AllMids { .. }
                | Subscription::L2Book { .. }
                | Subscription::Trades { .. }
                | Subscription::Candle { .. }
                | Subscription::Bbo { .. }
                | Subscription::ActiveAssetCtx { .. }
        )
    }

    /// Get the user address if this is a user subscription
    pub fn user(&self) -> Option<&str> {
        match self {
            Subscription::Notification { user }
            | Subscription::WebData3 { user }
            | Subscription::TwapStates { user, .. }
            | Subscription::ClearinghouseState { user, .. }
            | Subscription::OpenOrders { user, .. }
            | Subscription::OrderUpdates { user }
            | Subscription::UserEvents { user }
            | Subscription::UserFills { user, .. }
            | Subscription::UserFundings { user }
            | Subscription::UserNonFundingLedgerUpdates { user }
            | Subscription::ActiveAssetData { user, .. }
            | Subscription::UserTwapSliceFills { user }
            | Subscription::UserTwapHistory { user } => Some(user),
            _ => None,
        }
    }

    /// Get the coin/asset if this subscription has one
    pub fn coin(&self) -> Option<&str> {
        match self {
            Subscription::L2Book { coin, .. }
            | Subscription::Trades { coin }
            | Subscription::Candle { coin, .. }
            | Subscription::Bbo { coin }
            | Subscription::ActiveAssetCtx { coin }
            | Subscription::ActiveAssetData { coin, .. } => Some(coin),
            _ => None,
        }
    }
}

// ============================================================================
// WebSocket Message Types
// ============================================================================

/// Method for WebSocket subscription messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionMethod {
    /// Subscribe to a channel
    Subscribe,
    /// Unsubscribe from a channel
    Unsubscribe,
}

/// WebSocket subscription request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    /// Method (subscribe/unsubscribe)
    pub method: SubscriptionMethod,
    /// Subscription details
    pub subscription: Subscription,
}

impl SubscriptionRequest {
    /// Create a subscribe request
    pub fn subscribe(subscription: Subscription) -> Self {
        Self {
            method: SubscriptionMethod::Subscribe,
            subscription,
        }
    }

    /// Create an unsubscribe request
    pub fn unsubscribe(subscription: Subscription) -> Self {
        Self {
            method: SubscriptionMethod::Unsubscribe,
            subscription,
        }
    }
}

/// Response to a subscription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionResponse {
    /// Channel type
    pub channel: String,
    /// Response data
    pub data: SubscriptionResponseData,
}

/// Data in a subscription response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionResponseData {
    /// Method that was requested
    pub method: SubscriptionMethod,
    /// Subscription that was affected
    pub subscription: serde_json::Value,
}

// ============================================================================
// Channel Data Types
// ============================================================================

/// Wrapper for incoming WebSocket channel data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// Channel type
    pub channel: String,
    /// Data payload
    pub data: serde_json::Value,
}

impl ChannelMessage {
    /// Check if this is a subscription response
    pub fn is_subscription_response(&self) -> bool {
        self.channel == "subscriptionResponse"
    }

    /// Check if this is an error message
    pub fn is_error(&self) -> bool {
        self.channel == "error"
    }

    /// Try to parse the channel as a specific type
    pub fn parse_channel(&self) -> Option<&str> {
        if self.is_subscription_response() || self.is_error() {
            None
        } else {
            Some(&self.channel)
        }
    }

    /// Get the data as a specific type
    pub fn parse_data<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.data.clone())
    }
}

// ============================================================================
// Subscription Manager
// ============================================================================

/// Status of a subscription
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionStatus {
    /// Subscription request sent, waiting for confirmation
    Pending,
    /// Subscription is active
    Active,
    /// Unsubscription request sent, waiting for confirmation
    Unsubscribing,
}

/// Manages active subscriptions for a WebSocket connection
#[derive(Debug)]
pub struct SubscriptionManager {
    /// Map of subscriptions to their status
    subscriptions: Arc<RwLock<HashMap<Subscription, SubscriptionStatus>>>,
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a subscription as pending
    pub async fn add_pending(&self, subscription: Subscription) {
        let mut subs = self.subscriptions.write().await;
        subs.insert(subscription, SubscriptionStatus::Pending);
    }

    /// Mark a subscription as active
    pub async fn mark_active(&self, subscription: &Subscription) -> bool {
        let mut subs = self.subscriptions.write().await;
        if let Some(status) = subs.get_mut(subscription) {
            if *status == SubscriptionStatus::Pending {
                *status = SubscriptionStatus::Active;
                return true;
            }
        }
        false
    }

    /// Mark a subscription as unsubscribing
    pub async fn mark_unsubscribing(&self, subscription: &Subscription) -> bool {
        let mut subs = self.subscriptions.write().await;
        if let Some(status) = subs.get_mut(subscription) {
            if *status == SubscriptionStatus::Active {
                *status = SubscriptionStatus::Unsubscribing;
                return true;
            }
        }
        false
    }

    /// Remove a subscription (after unsubscribe confirmation)
    pub async fn remove(&self, subscription: &Subscription) -> bool {
        let mut subs = self.subscriptions.write().await;
        subs.remove(subscription).is_some()
    }

    /// Check if a subscription exists
    pub async fn contains(&self, subscription: &Subscription) -> bool {
        let subs = self.subscriptions.read().await;
        subs.contains_key(subscription)
    }

    /// Get the status of a subscription
    pub async fn status(&self, subscription: &Subscription) -> Option<SubscriptionStatus> {
        let subs = self.subscriptions.read().await;
        subs.get(subscription).copied()
    }

    /// Check if a subscription is active
    pub async fn is_active(&self, subscription: &Subscription) -> bool {
        self.status(subscription).await == Some(SubscriptionStatus::Active)
    }

    /// Get all active subscriptions
    pub async fn active_subscriptions(&self) -> Vec<Subscription> {
        let subs = self.subscriptions.read().await;
        subs.iter()
            .filter(|(_, status)| **status == SubscriptionStatus::Active)
            .map(|(sub, _)| sub.clone())
            .collect()
    }

    /// Get all subscriptions with their status
    pub async fn all_subscriptions(&self) -> Vec<(Subscription, SubscriptionStatus)> {
        let subs = self.subscriptions.read().await;
        subs.iter()
            .map(|(sub, status)| (sub.clone(), *status))
            .collect()
    }

    /// Get the count of active subscriptions
    pub async fn active_count(&self) -> usize {
        let subs = self.subscriptions.read().await;
        subs.values()
            .filter(|status| **status == SubscriptionStatus::Active)
            .count()
    }

    /// Get the total count of all subscriptions (including pending)
    pub async fn total_count(&self) -> usize {
        let subs = self.subscriptions.read().await;
        subs.len()
    }

    /// Clear all subscriptions
    pub async fn clear(&self) {
        let mut subs = self.subscriptions.write().await;
        subs.clear();
    }
}

impl Clone for SubscriptionManager {
    fn clone(&self) -> Self {
        Self {
            subscriptions: Arc::clone(&self.subscriptions),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ Subscription Construction Tests ============

    #[test]
    fn test_all_mids_subscription() {
        let sub = Subscription::all_mids();
        assert_eq!(sub.channel_name(), "allMids");
        assert!(sub.is_market_subscription());
        assert!(!sub.is_user_subscription());
        assert!(sub.user().is_none());
    }

    #[test]
    fn test_all_mids_with_dex() {
        let sub = Subscription::all_mids_with_dex("perp");
        if let Subscription::AllMids { dex } = sub {
            assert_eq!(dex, Some("perp".to_string()));
        } else {
            panic!("Expected AllMids subscription");
        }
    }

    #[test]
    fn test_l2_book_subscription() {
        let sub = Subscription::l2_book("BTC");
        assert_eq!(sub.channel_name(), "l2Book");
        assert_eq!(sub.coin(), Some("BTC"));
        assert!(sub.is_market_subscription());
    }

    #[test]
    fn test_l2_book_with_params() {
        let sub = Subscription::l2_book_with_params("ETH", Some(5), Some(2));
        if let Subscription::L2Book {
            coin,
            n_sig_figs,
            mantissa,
        } = sub
        {
            assert_eq!(coin, "ETH");
            assert_eq!(n_sig_figs, Some(5));
            assert_eq!(mantissa, Some(2));
        } else {
            panic!("Expected L2Book subscription");
        }
    }

    #[test]
    fn test_trades_subscription() {
        let sub = Subscription::trades("SOL");
        assert_eq!(sub.channel_name(), "trades");
        assert_eq!(sub.coin(), Some("SOL"));
    }

    #[test]
    fn test_candle_subscription() {
        let sub = Subscription::candle("BTC", CandleInterval::OneHour);
        assert_eq!(sub.channel_name(), "candle");
        assert_eq!(sub.coin(), Some("BTC"));
        if let Subscription::Candle { interval, .. } = sub {
            assert_eq!(interval, CandleInterval::OneHour);
        }
    }

    #[test]
    fn test_bbo_subscription() {
        let sub = Subscription::bbo("ETH");
        assert_eq!(sub.channel_name(), "bbo");
        assert_eq!(sub.coin(), Some("ETH"));
    }

    #[test]
    fn test_notification_subscription() {
        let sub = Subscription::notification("0x1234");
        assert_eq!(sub.channel_name(), "notification");
        assert_eq!(sub.user(), Some("0x1234"));
        assert!(sub.is_user_subscription());
        assert!(!sub.is_market_subscription());
    }

    #[test]
    fn test_order_updates_subscription() {
        let sub = Subscription::order_updates("0xabcd");
        assert_eq!(sub.channel_name(), "orderUpdates");
        assert_eq!(sub.user(), Some("0xabcd"));
    }

    #[test]
    fn test_user_fills_subscription() {
        let sub = Subscription::user_fills("0x5678");
        assert_eq!(sub.channel_name(), "userFills");
        assert_eq!(sub.user(), Some("0x5678"));
    }

    #[test]
    fn test_user_fills_with_aggregation() {
        let sub = Subscription::user_fills_with_aggregation("0x5678", true);
        if let Subscription::UserFills {
            user,
            aggregate_by_time,
        } = sub
        {
            assert_eq!(user, "0x5678");
            assert_eq!(aggregate_by_time, Some(true));
        } else {
            panic!("Expected UserFills subscription");
        }
    }

    #[test]
    fn test_clearinghouse_state_subscription() {
        let sub = Subscription::clearinghouse_state("0x1111");
        assert_eq!(sub.channel_name(), "clearinghouseState");
        assert_eq!(sub.user(), Some("0x1111"));
    }

    #[test]
    fn test_clearinghouse_state_with_dex() {
        let sub = Subscription::clearinghouse_state_with_dex("0x1111", "spot");
        if let Subscription::ClearinghouseState { user, dex } = sub {
            assert_eq!(user, "0x1111");
            assert_eq!(dex, Some("spot".to_string()));
        } else {
            panic!("Expected ClearinghouseState subscription");
        }
    }

    #[test]
    fn test_active_asset_data_subscription() {
        let sub = Subscription::active_asset_data("0x2222", "BTC");
        assert_eq!(sub.channel_name(), "activeAssetData");
        assert_eq!(sub.user(), Some("0x2222"));
        assert_eq!(sub.coin(), Some("BTC"));
        assert!(sub.is_user_subscription());
    }

    // ============ Subscription Serialization Tests ============

    #[test]
    fn test_subscription_serialize_all_mids() {
        let sub = Subscription::all_mids();
        let json = serde_json::to_string(&sub).unwrap();
        assert!(json.contains("\"type\":\"allMids\""));
    }

    #[test]
    fn test_subscription_serialize_l2_book() {
        let sub = Subscription::l2_book("BTC");
        let json = serde_json::to_string(&sub).unwrap();
        assert!(json.contains("\"type\":\"l2Book\""));
        assert!(json.contains("\"coin\":\"BTC\""));
    }

    #[test]
    fn test_subscription_serialize_candle() {
        let sub = Subscription::candle("ETH", CandleInterval::FifteenMinutes);
        let json = serde_json::to_string(&sub).unwrap();
        assert!(json.contains("\"type\":\"candle\""));
        assert!(json.contains("\"coin\":\"ETH\""));
        assert!(json.contains("\"interval\":\"15m\""));
    }

    #[test]
    fn test_subscription_serialize_user_fills() {
        let sub = Subscription::user_fills("0x1234567890123456789012345678901234567890");
        let json = serde_json::to_string(&sub).unwrap();
        assert!(json.contains("\"type\":\"userFills\""));
        assert!(json.contains("\"user\":\"0x1234567890123456789012345678901234567890\""));
    }

    // ============ SubscriptionRequest Tests ============

    #[test]
    fn test_subscription_request_subscribe() {
        let req = SubscriptionRequest::subscribe(Subscription::trades("BTC"));
        assert_eq!(req.method, SubscriptionMethod::Subscribe);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"trades\""));
    }

    #[test]
    fn test_subscription_request_unsubscribe() {
        let req = SubscriptionRequest::unsubscribe(Subscription::trades("BTC"));
        assert_eq!(req.method, SubscriptionMethod::Unsubscribe);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"method\":\"unsubscribe\""));
    }

    // ============ ChannelMessage Tests ============

    #[test]
    fn test_channel_message_parse() {
        let json = r#"{"channel": "trades", "data": {"coin": "BTC"}}"#;
        let msg: ChannelMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.channel, "trades");
        assert!(!msg.is_subscription_response());
        assert!(!msg.is_error());
        assert_eq!(msg.parse_channel(), Some("trades"));
    }

    #[test]
    fn test_channel_message_subscription_response() {
        let json = r#"{"channel": "subscriptionResponse", "data": {"method": "subscribe", "subscription": {}}}"#;
        let msg: ChannelMessage = serde_json::from_str(json).unwrap();
        assert!(msg.is_subscription_response());
        assert!(msg.parse_channel().is_none());
    }

    #[test]
    fn test_channel_message_error() {
        let json = r#"{"channel": "error", "data": "Invalid subscription"}"#;
        let msg: ChannelMessage = serde_json::from_str(json).unwrap();
        assert!(msg.is_error());
        assert!(msg.parse_channel().is_none());
    }

    // ============ SubscriptionManager Tests ============

    #[tokio::test]
    async fn test_subscription_manager_new() {
        let manager = SubscriptionManager::new();
        assert_eq!(manager.total_count().await, 0);
        assert_eq!(manager.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_subscription_manager_add_pending() {
        let manager = SubscriptionManager::new();
        let sub = Subscription::trades("BTC");

        manager.add_pending(sub.clone()).await;

        assert!(manager.contains(&sub).await);
        assert_eq!(
            manager.status(&sub).await,
            Some(SubscriptionStatus::Pending)
        );
        assert!(!manager.is_active(&sub).await);
        assert_eq!(manager.total_count().await, 1);
        assert_eq!(manager.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_subscription_manager_mark_active() {
        let manager = SubscriptionManager::new();
        let sub = Subscription::trades("BTC");

        manager.add_pending(sub.clone()).await;
        let result = manager.mark_active(&sub).await;

        assert!(result);
        assert_eq!(manager.status(&sub).await, Some(SubscriptionStatus::Active));
        assert!(manager.is_active(&sub).await);
        assert_eq!(manager.active_count().await, 1);
    }

    #[tokio::test]
    async fn test_subscription_manager_mark_active_not_pending() {
        let manager = SubscriptionManager::new();
        let sub = Subscription::trades("BTC");

        // Try to mark active without adding first
        let result = manager.mark_active(&sub).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_subscription_manager_mark_unsubscribing() {
        let manager = SubscriptionManager::new();
        let sub = Subscription::trades("BTC");

        manager.add_pending(sub.clone()).await;
        manager.mark_active(&sub).await;
        let result = manager.mark_unsubscribing(&sub).await;

        assert!(result);
        assert_eq!(
            manager.status(&sub).await,
            Some(SubscriptionStatus::Unsubscribing)
        );
    }

    #[tokio::test]
    async fn test_subscription_manager_mark_unsubscribing_not_active() {
        let manager = SubscriptionManager::new();
        let sub = Subscription::trades("BTC");

        manager.add_pending(sub.clone()).await;
        // Don't mark as active

        let result = manager.mark_unsubscribing(&sub).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_subscription_manager_remove() {
        let manager = SubscriptionManager::new();
        let sub = Subscription::trades("BTC");

        manager.add_pending(sub.clone()).await;
        manager.mark_active(&sub).await;

        let result = manager.remove(&sub).await;
        assert!(result);
        assert!(!manager.contains(&sub).await);
        assert_eq!(manager.total_count().await, 0);
    }

    #[tokio::test]
    async fn test_subscription_manager_remove_nonexistent() {
        let manager = SubscriptionManager::new();
        let sub = Subscription::trades("BTC");

        let result = manager.remove(&sub).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_subscription_manager_active_subscriptions() {
        let manager = SubscriptionManager::new();
        let sub1 = Subscription::trades("BTC");
        let sub2 = Subscription::trades("ETH");
        let sub3 = Subscription::l2_book("SOL");

        manager.add_pending(sub1.clone()).await;
        manager.add_pending(sub2.clone()).await;
        manager.add_pending(sub3.clone()).await;

        manager.mark_active(&sub1).await;
        manager.mark_active(&sub2).await;
        // sub3 stays pending

        let active = manager.active_subscriptions().await;
        assert_eq!(active.len(), 2);
        assert!(active.contains(&sub1));
        assert!(active.contains(&sub2));
        assert!(!active.contains(&sub3));
    }

    #[tokio::test]
    async fn test_subscription_manager_all_subscriptions() {
        let manager = SubscriptionManager::new();
        let sub1 = Subscription::trades("BTC");
        let sub2 = Subscription::trades("ETH");

        manager.add_pending(sub1.clone()).await;
        manager.add_pending(sub2.clone()).await;
        manager.mark_active(&sub1).await;

        let all = manager.all_subscriptions().await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_subscription_manager_clear() {
        let manager = SubscriptionManager::new();
        let sub1 = Subscription::trades("BTC");
        let sub2 = Subscription::trades("ETH");

        manager.add_pending(sub1).await;
        manager.add_pending(sub2).await;

        manager.clear().await;
        assert_eq!(manager.total_count().await, 0);
    }

    #[tokio::test]
    async fn test_subscription_manager_clone_shares_state() {
        let manager = SubscriptionManager::new();
        let sub = Subscription::trades("BTC");

        manager.add_pending(sub.clone()).await;

        let cloned = manager.clone();
        cloned.mark_active(&sub).await;

        // Original should see the change
        assert!(manager.is_active(&sub).await);
    }

    // ============ Subscription Equality Tests ============

    #[test]
    fn test_subscription_equality() {
        let sub1 = Subscription::trades("BTC");
        let sub2 = Subscription::trades("BTC");
        let sub3 = Subscription::trades("ETH");

        assert_eq!(sub1, sub2);
        assert_ne!(sub1, sub3);
    }

    #[test]
    fn test_subscription_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(Subscription::trades("BTC"));
        set.insert(Subscription::trades("BTC")); // Duplicate
        set.insert(Subscription::trades("ETH"));

        assert_eq!(set.len(), 2);
    }

    // ============ Send/Sync Tests ============

    #[test]
    fn test_subscription_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Subscription>();
        assert_sync::<Subscription>();
    }

    #[test]
    fn test_subscription_request_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<SubscriptionRequest>();
        assert_sync::<SubscriptionRequest>();
    }

    #[test]
    fn test_subscription_manager_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<SubscriptionManager>();
        assert_sync::<SubscriptionManager>();
    }
}
