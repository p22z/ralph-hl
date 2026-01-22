//! Application state for the demo server

use hyperliquid_sdk::{Client, Network};
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// SDK client for mainnet
    pub client: Arc<Client>,
}

impl AppState {
    /// Create a new application state with mainnet client
    pub fn new() -> Self {
        Self {
            client: Arc::new(Client::new(Network::Mainnet).expect("Failed to create SDK client")),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
