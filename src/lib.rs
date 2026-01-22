//! Hyperliquid Rust SDK
//!
//! A Rust SDK for interacting with the Hyperliquid API, including:
//! - Info endpoints (read-only market data and user queries)
//! - Exchange endpoints (authenticated trading actions)
//! - WebSocket subscriptions (real-time data streams)

pub mod auth;
pub mod client;
pub mod error;
pub mod info;
pub mod types;

pub use auth::{Signature, Wallet};
pub use client::{Client, Network, MAINNET_URL, TESTNET_URL};
pub use error::{Error, Result};
pub use types::*;
