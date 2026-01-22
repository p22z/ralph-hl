//! Info endpoint implementations for the Hyperliquid API
//!
//! This module provides high-level methods for querying market data and user state
//! from the Hyperliquid info endpoints.

mod account;
mod borrow_lend;
mod fills;
mod funding;
mod market;
mod orders;
mod perpetuals;
mod spot;
mod staking;
mod user;
mod vault;

// Re-export vault types
pub use vault::{
    BuilderFeeApprovalInfo, Hip3State, PerpDexStatus, ReferralInfo, UserVaultDeposit, VaultDetails,
};
