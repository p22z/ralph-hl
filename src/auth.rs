//! Authentication module for Hyperliquid API
//!
//! This module provides EIP-712 signing functionality for authenticated API requests.
//! Hyperliquid uses a phantom agent construction for L1 actions where the action is
//! hashed and used as a connection ID in the EIP-712 typed data.

use ethers::core::k256::ecdsa::SigningKey;
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{
    transaction::eip712::{EIP712Domain, Eip712DomainType, TypedData, Types},
    Address, H256,
};
use ethers::utils::keccak256;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};

/// EIP-712 domain separator for Hyperliquid L1 actions
const DOMAIN_NAME: &str = "Exchange";
const DOMAIN_VERSION: &str = "1";
const DOMAIN_CHAIN_ID: u64 = 1337;

/// Source identifier for phantom agent
const MAINNET_SOURCE: &str = "a";
const TESTNET_SOURCE: &str = "b";

/// Signature components for API submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub r: String,
    pub s: String,
    pub v: u8,
}

impl Signature {
    /// Create a new signature from components
    pub fn new(r: H256, s: H256, v: u8) -> Self {
        Self {
            r: format!("{:#x}", r),
            s: format!("{:#x}", s),
            v,
        }
    }
}

/// Wallet wrapper for signing Hyperliquid actions
#[derive(Clone)]
pub struct Wallet {
    inner: LocalWallet,
    is_mainnet: bool,
}

impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wallet")
            .field("address", &self.address())
            .field("is_mainnet", &self.is_mainnet)
            .finish()
    }
}

impl Wallet {
    /// Create a wallet from a private key hex string
    ///
    /// The private key should be a 32-byte hex string (with or without 0x prefix).
    pub fn from_private_key(private_key: &str, is_mainnet: bool) -> Result<Self> {
        let key = private_key.strip_prefix("0x").unwrap_or(private_key);

        let key_bytes =
            hex::decode(key).map_err(|e| Error::Auth(format!("Invalid private key hex: {}", e)))?;

        if key_bytes.len() != 32 {
            return Err(Error::Auth(format!(
                "Private key must be 32 bytes, got {}",
                key_bytes.len()
            )));
        }

        let signing_key = SigningKey::from_bytes(key_bytes.as_slice().into())
            .map_err(|e| Error::Auth(format!("Invalid private key: {}", e)))?;

        let wallet = LocalWallet::from(signing_key);

        Ok(Self {
            inner: wallet,
            is_mainnet,
        })
    }

    /// Create a wallet from raw private key bytes
    pub fn from_bytes(private_key: &[u8; 32], is_mainnet: bool) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(private_key.into())
            .map_err(|e| Error::Auth(format!("Invalid private key: {}", e)))?;

        let wallet = LocalWallet::from(signing_key);

        Ok(Self {
            inner: wallet,
            is_mainnet,
        })
    }

    /// Get the wallet's Ethereum address
    pub fn address(&self) -> Address {
        self.inner.address()
    }

    /// Get whether this wallet is configured for mainnet
    pub fn is_mainnet(&self) -> bool {
        self.is_mainnet
    }

    /// Generate a nonce (current timestamp in milliseconds)
    pub fn generate_nonce() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64
    }

    /// Sign an L1 action (trading operations like orders, cancels, etc.)
    ///
    /// This uses the phantom agent construction where:
    /// 1. The action is serialized with msgpack
    /// 2. Nonce, vault address (if any), and expiration (if any) are appended
    /// 3. The result is hashed with keccak256 to create the connection_id
    /// 4. A phantom agent is created with source and connection_id
    /// 5. The agent is signed using EIP-712
    ///
    /// # Arguments
    /// * `action` - The action to sign (must be serializable)
    /// * `nonce` - Timestamp in milliseconds
    /// * `vault_address` - Optional vault address
    ///
    /// # Returns
    /// The signature components (r, s, v)
    pub async fn sign_l1_action<T: Serialize>(
        &self,
        action: &T,
        nonce: u64,
        vault_address: Option<Address>,
    ) -> Result<Signature> {
        // Compute connection ID
        let connection_id = self.compute_connection_id(action, nonce, vault_address)?;

        // Create phantom agent and sign it
        let source = if self.is_mainnet {
            MAINNET_SOURCE
        } else {
            TESTNET_SOURCE
        };

        let typed_data = create_agent_typed_data(source, connection_id);

        // Sign the typed data
        let signature = self
            .inner
            .sign_typed_data(&typed_data)
            .await
            .map_err(|e| Error::Auth(format!("Failed to sign: {}", e)))?;

        // Extract r, s, v components
        let sig_bytes = signature.to_vec();
        let r = H256::from_slice(&sig_bytes[0..32]);
        let s = H256::from_slice(&sig_bytes[32..64]);
        let v = sig_bytes[64] as u8;

        Ok(Signature::new(r, s, v))
    }

    /// Compute the connection ID (keccak256 hash) for an action
    ///
    /// Format:
    /// - msgpack(action)
    /// - nonce as 8 bytes big-endian
    /// - vault flag (1 byte): 1 if vault present, 0 otherwise
    /// - vault address (20 bytes) if present
    fn compute_connection_id<T: Serialize>(
        &self,
        action: &T,
        nonce: u64,
        vault_address: Option<Address>,
    ) -> Result<[u8; 32]> {
        let mut data = Vec::new();

        // Serialize action with msgpack
        let action_bytes = rmp_serde::to_vec_named(action)
            .map_err(|e| Error::Auth(format!("Failed to serialize action: {}", e)))?;
        data.extend_from_slice(&action_bytes);

        // Append nonce as 8 bytes big-endian
        data.extend_from_slice(&nonce.to_be_bytes());

        // Append vault address info
        match vault_address {
            Some(addr) => {
                data.push(1u8);
                data.extend_from_slice(addr.as_bytes());
            }
            None => {
                data.push(0u8);
            }
        }

        // Compute keccak256 hash
        let hash = keccak256(&data);
        Ok(hash)
    }
}

/// Create the EIP-712 typed data for the phantom agent
fn create_agent_typed_data(source: &str, connection_id: [u8; 32]) -> TypedData {
    // Build the types
    let mut types = Types::new();

    // EIP712Domain type
    types.insert(
        "EIP712Domain".to_string(),
        vec![
            Eip712DomainType {
                name: "name".to_string(),
                r#type: "string".to_string(),
            },
            Eip712DomainType {
                name: "version".to_string(),
                r#type: "string".to_string(),
            },
            Eip712DomainType {
                name: "chainId".to_string(),
                r#type: "uint256".to_string(),
            },
            Eip712DomainType {
                name: "verifyingContract".to_string(),
                r#type: "address".to_string(),
            },
        ],
    );

    // Agent type
    types.insert(
        "Agent".to_string(),
        vec![
            Eip712DomainType {
                name: "source".to_string(),
                r#type: "string".to_string(),
            },
            Eip712DomainType {
                name: "connectionId".to_string(),
                r#type: "bytes32".to_string(),
            },
        ],
    );

    // Build the domain
    let domain = EIP712Domain {
        name: Some(DOMAIN_NAME.to_string()),
        version: Some(DOMAIN_VERSION.to_string()),
        chain_id: Some(DOMAIN_CHAIN_ID.into()),
        verifying_contract: Some(Address::zero()),
        salt: None,
    };

    // Build the message
    let mut message = BTreeMap::new();
    message.insert(
        "source".to_string(),
        serde_json::Value::String(source.to_string()),
    );
    message.insert(
        "connectionId".to_string(),
        serde_json::Value::String(format!("0x{}", hex::encode(connection_id))),
    );

    TypedData {
        types,
        domain,
        primary_type: "Agent".to_string(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TEST_PRIVATE_KEY: &str =
        "0x0123456789012345678901234567890123456789012345678901234567890123";

    #[test]
    fn test_wallet_creation_from_hex() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        assert!(wallet.is_mainnet());

        // Also test without 0x prefix
        let wallet2 = Wallet::from_private_key(
            "0123456789012345678901234567890123456789012345678901234567890123",
            false,
        )
        .unwrap();
        assert!(!wallet2.is_mainnet());

        // Both should have the same address
        assert_eq!(wallet.address(), wallet2.address());
    }

    #[test]
    fn test_wallet_creation_from_bytes() {
        let bytes: [u8; 32] = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0x01, 0x23, 0x45, 0x67, 0x89, 0x01, 0x23, 0x45, 0x67,
            0x89, 0x01, 0x23, 0x45, 0x67, 0x89, 0x01, 0x23, 0x45, 0x67, 0x89, 0x01, 0x23, 0x45,
            0x67, 0x89, 0x01, 0x23,
        ];
        let wallet = Wallet::from_bytes(&bytes, true).unwrap();
        assert!(wallet.is_mainnet());
    }

    #[test]
    fn test_invalid_private_key() {
        // Too short
        let result = Wallet::from_private_key("0x1234", true);
        assert!(result.is_err());

        // Invalid hex
        let result = Wallet::from_private_key("0xGGGG", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_nonce_generation() {
        let nonce1 = Wallet::generate_nonce();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let nonce2 = Wallet::generate_nonce();

        // Nonce should be increasing
        assert!(nonce2 > nonce1);

        // Nonce should be in reasonable range (after year 2020)
        assert!(nonce1 > 1577836800000);
    }

    #[test]
    fn test_connection_id_computation() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        // Create a simple action
        let action = json!({"type": "dummy", "num": 1000});

        // Compute connection ID with nonce 0
        let conn_id = wallet.compute_connection_id(&action, 0, None).unwrap();

        // Connection ID should be 32 bytes
        assert_eq!(conn_id.len(), 32);

        // Same inputs should produce same output
        let conn_id2 = wallet.compute_connection_id(&action, 0, None).unwrap();
        assert_eq!(conn_id, conn_id2);

        // Different nonce should produce different output
        let conn_id3 = wallet.compute_connection_id(&action, 1, None).unwrap();
        assert_ne!(conn_id, conn_id3);
    }

    #[test]
    fn test_connection_id_with_vault() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let action = json!({"type": "dummy", "num": 1000});

        let vault_address: Address = "0x1719884eb866cb12b2287399b15f7db5e7d775ea"
            .parse()
            .unwrap();

        // Without vault
        let conn_id_no_vault = wallet.compute_connection_id(&action, 0, None).unwrap();

        // With vault
        let conn_id_with_vault = wallet
            .compute_connection_id(&action, 0, Some(vault_address))
            .unwrap();

        // Should be different
        assert_ne!(conn_id_no_vault, conn_id_with_vault);
    }

    #[tokio::test]
    async fn test_sign_l1_action_mainnet() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let action = json!({"type": "dummy", "num": 1000});

        let signature = wallet.sign_l1_action(&action, 0, None).await.unwrap();

        // Verify signature format
        assert!(signature.r.starts_with("0x"));
        assert!(signature.s.starts_with("0x"));
        assert!(signature.v == 27 || signature.v == 28);

        // Check signature is deterministic
        let signature2 = wallet.sign_l1_action(&action, 0, None).await.unwrap();
        assert_eq!(signature.r, signature2.r);
        assert_eq!(signature.s, signature2.s);
        assert_eq!(signature.v, signature2.v);
    }

    #[tokio::test]
    async fn test_sign_l1_action_testnet() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, false).unwrap();

        let action = json!({"type": "dummy", "num": 1000});

        let signature = wallet.sign_l1_action(&action, 0, None).await.unwrap();

        // Verify signature format
        assert!(signature.r.starts_with("0x"));
        assert!(signature.s.starts_with("0x"));
        assert!(signature.v == 27 || signature.v == 28);
    }

    #[tokio::test]
    async fn test_mainnet_vs_testnet_different_signatures() {
        let wallet_mainnet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let wallet_testnet = Wallet::from_private_key(TEST_PRIVATE_KEY, false).unwrap();

        let action = json!({"type": "dummy", "num": 1000});

        let sig_mainnet = wallet_mainnet
            .sign_l1_action(&action, 0, None)
            .await
            .unwrap();
        let sig_testnet = wallet_testnet
            .sign_l1_action(&action, 0, None)
            .await
            .unwrap();

        // Signatures should be different due to different source ("a" vs "b")
        assert_ne!(sig_mainnet.r, sig_testnet.r);
    }

    #[tokio::test]
    async fn test_sign_with_vault_address() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let action = json!({"type": "dummy", "num": 1000});
        let vault: Address = "0x1719884eb866cb12b2287399b15f7db5e7d775ea"
            .parse()
            .unwrap();

        let sig_no_vault = wallet.sign_l1_action(&action, 0, None).await.unwrap();
        let sig_with_vault = wallet
            .sign_l1_action(&action, 0, Some(vault))
            .await
            .unwrap();

        // Signatures should be different due to vault address
        assert_ne!(sig_no_vault.r, sig_with_vault.r);
    }

    #[test]
    fn test_signature_serialization() {
        let sig = Signature::new(
            H256::from_low_u64_be(0x1234),
            H256::from_low_u64_be(0x5678),
            27,
        );

        let json = serde_json::to_string(&sig).unwrap();
        assert!(json.contains("\"r\""));
        assert!(json.contains("\"s\""));
        assert!(json.contains("\"v\":27"));

        // Can deserialize back
        let sig2: Signature = serde_json::from_str(&json).unwrap();
        assert_eq!(sig.v, sig2.v);
    }

    #[test]
    fn test_wallet_debug() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();
        let debug_str = format!("{:?}", wallet);

        // Debug should not expose private key
        assert!(!debug_str.contains("0123456789"));
        assert!(debug_str.contains("Wallet"));
        assert!(debug_str.contains("address"));
    }

    #[test]
    fn test_wallet_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<Wallet>();
        assert_sync::<Wallet>();
    }

    #[test]
    fn test_create_agent_typed_data() {
        let connection_id = [0u8; 32];
        let typed_data = create_agent_typed_data("a", connection_id);

        // Verify the structure
        assert_eq!(typed_data.primary_type, "Agent");
        assert!(typed_data.types.contains_key("Agent"));
        assert!(typed_data.types.contains_key("EIP712Domain"));
        assert_eq!(typed_data.domain.name, Some("Exchange".to_string()));
        assert_eq!(typed_data.domain.version, Some("1".to_string()));
        assert_eq!(typed_data.domain.chain_id, Some(1337.into()));
        assert_eq!(typed_data.domain.verifying_contract, Some(Address::zero()));
    }
}
