//! Hyperliquid Rust SDK
//!
//! A Rust SDK for interacting with the Hyperliquid API, including:
//! - Info endpoints (read-only market data and user queries)
//! - Exchange endpoints (authenticated trading actions)
//! - WebSocket subscriptions (real-time data streams)

pub mod auth;
pub mod client;
pub mod error;
pub mod exchange;
pub mod info;
pub mod types;
pub mod websocket;

pub use auth::{Signature, Wallet};
pub use client::{Client, Network, MAINNET_URL, TESTNET_URL};
pub use error::{Error, Result};
pub use exchange::approval::{get_approval_hash, is_approval_successful};
pub use exchange::cancel::is_cancel_successful;
pub use exchange::leverage::{get_updated_leverage, is_leverage_update_successful};
pub use exchange::misc::{get_misc_hash, is_misc_successful};
pub use exchange::modify::{ModifyOrderBuilder, ModifyTriggerOrderBuilder, ModifyWire};
pub use exchange::orders::{
    get_order_id, is_order_successful, LimitOrderBuilder, TriggerOrderBuilder,
};
pub use exchange::schedule_cancel::{get_scheduled_time, is_schedule_cancel_successful};
pub use exchange::transfer::{get_transfer_hash, is_transfer_successful};
pub use exchange::twap::{get_twap_error, get_twap_id, is_twap_running};
pub use exchange::vault::{get_vault_hash, is_vault_action_successful};
pub use types::*;
pub use websocket::{
    ConnectionState, ReconnectConfig, WsClient, WsMessage, MAINNET_WS_URL, TESTNET_WS_URL,
};
