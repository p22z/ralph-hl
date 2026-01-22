//! WebSocket client for Hyperliquid real-time data streams
//!
//! This module provides a WebSocket client for connecting to Hyperliquid's
//! real-time data feed. It handles connection management, automatic reconnection,
//! heartbeat/ping handling, and subscription management.
//!
//! # Example
//!
//! ```ignore
//! use hyperliquid::websocket::{WsClient, Subscription};
//!
//! let client = WsClient::mainnet();
//! client.connect().await?;
//!
//! // Subscribe to trades
//! client.subscribe(Subscription::trades("BTC")).await?;
//!
//! // Receive messages
//! while let Some(msg) = client.recv().await {
//!     // Handle message
//! }
//! ```

mod client;
mod subscription;

pub use client::{
    ConnectionState, ReconnectConfig, WsClient, WsMessage, MAINNET_WS_URL, TESTNET_WS_URL,
};
pub use subscription::{
    ChannelMessage, Subscription, SubscriptionManager, SubscriptionMethod, SubscriptionRequest,
    SubscriptionResponse, SubscriptionResponseData, SubscriptionStatus,
};
