//! Hyperliquid Rust SDK
//!
//! A Rust SDK for interacting with the Hyperliquid API, including:
//! - Info endpoints (read-only market data and user queries)
//! - Exchange endpoints (authenticated trading actions)
//! - WebSocket subscriptions (real-time data streams)

pub mod client;
pub mod error;
pub mod types;

pub use client::Client;
pub use error::{Error, Result};
