//! HTTP client wrapper for Hyperliquid API

use reqwest::Client as ReqwestClient;

use crate::error::Result;

/// Base URLs for Hyperliquid API
pub const MAINNET_URL: &str = "https://api.hyperliquid.xyz";
pub const TESTNET_URL: &str = "https://api.hyperliquid-testnet.xyz";

/// Network configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Network {
    /// Mainnet environment
    #[default]
    Mainnet,
    /// Testnet environment
    Testnet,
}

impl Network {
    /// Get the base URL for this network
    pub fn base_url(&self) -> &'static str {
        match self {
            Network::Mainnet => MAINNET_URL,
            Network::Testnet => TESTNET_URL,
        }
    }
}


/// Hyperliquid API client
#[derive(Debug, Clone)]
pub struct Client {
    http: ReqwestClient,
    network: Network,
}

impl Client {
    /// Create a new client for the specified network
    pub fn new(network: Network) -> Result<Self> {
        let http = ReqwestClient::builder()
            .build()
            .map_err(crate::error::Error::Http)?;

        Ok(Self { http, network })
    }

    /// Create a new client for mainnet
    pub fn mainnet() -> Result<Self> {
        Self::new(Network::Mainnet)
    }

    /// Create a new client for testnet
    pub fn testnet() -> Result<Self> {
        Self::new(Network::Testnet)
    }

    /// Get the base URL for the current network
    pub fn base_url(&self) -> &str {
        self.network.base_url()
    }

    /// Get the info endpoint URL
    pub fn info_url(&self) -> String {
        format!("{}/info", self.base_url())
    }

    /// Get the exchange endpoint URL
    pub fn exchange_url(&self) -> String {
        format!("{}/exchange", self.base_url())
    }

    /// Get the underlying HTTP client
    pub fn http(&self) -> &ReqwestClient {
        &self.http
    }

    /// Get the current network
    pub fn network(&self) -> Network {
        self.network
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::mainnet().expect("Failed to create default client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_urls() {
        assert_eq!(Network::Mainnet.base_url(), MAINNET_URL);
        assert_eq!(Network::Testnet.base_url(), TESTNET_URL);
    }

    #[test]
    fn test_client_creation() {
        let client = Client::mainnet().unwrap();
        assert_eq!(client.network(), Network::Mainnet);
        assert_eq!(client.base_url(), MAINNET_URL);

        let client = Client::testnet().unwrap();
        assert_eq!(client.network(), Network::Testnet);
        assert_eq!(client.base_url(), TESTNET_URL);
    }

    #[test]
    fn test_endpoint_urls() {
        let client = Client::mainnet().unwrap();
        assert_eq!(client.info_url(), "https://api.hyperliquid.xyz/info");
        assert_eq!(client.exchange_url(), "https://api.hyperliquid.xyz/exchange");
    }
}
