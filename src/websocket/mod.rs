//! WebSocket client for Hyperliquid real-time data streams
//!
//! This module provides a WebSocket client for connecting to Hyperliquid's
//! real-time data feed. It handles connection management, automatic reconnection,
//! and heartbeat/ping handling.

mod client;

pub use client::{
    ConnectionState, ReconnectConfig, WsClient, WsMessage, MAINNET_WS_URL, TESTNET_WS_URL,
};
