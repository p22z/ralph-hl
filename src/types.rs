//! Request and response types for the Hyperliquid API
//!
//! This module contains all the type definitions for interacting with the Hyperliquid API,
//! including info endpoints, exchange endpoints, and common types.

use serde::{Deserialize, Serialize};

// ============================================================================
// Common Enums
// ============================================================================

/// Time in force options for orders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good till cancelled - order remains active until filled or cancelled
    #[serde(rename = "Gtc")]
    Gtc,
    /// Immediate or cancel - fill what's possible immediately, cancel the rest
    #[serde(rename = "Ioc")]
    Ioc,
    /// Add liquidity only - order is rejected if it would take liquidity
    #[serde(rename = "Alo")]
    Alo,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    /// Limit order
    Limit,
    /// Market order (using trigger)
    Trigger,
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    /// Buy order
    #[serde(rename = "B")]
    Buy,
    /// Sell order
    #[serde(rename = "A")]
    Sell,
}

/// Trigger type for conditional orders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriggerType {
    /// Take profit - trigger when price rises above the trigger price
    Tp,
    /// Stop loss - trigger when price falls below the trigger price
    Sl,
}

/// Margin mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarginMode {
    /// Cross margin - share margin across positions
    Cross,
    /// Isolated margin - separate margin per position
    Isolated,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderStatus {
    /// Order is open and active
    Open,
    /// Order has been completely filled
    Filled,
    /// Order has been cancelled
    Canceled,
    /// Trigger order has been triggered
    Triggered,
    /// Order was rejected
    Rejected,
    /// Order was cancelled due to margin issues
    MarginCanceled,
}

/// Candle interval for historical data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandleInterval {
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "3m")]
    ThreeMinutes,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "30m")]
    ThirtyMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "2h")]
    TwoHours,
    #[serde(rename = "4h")]
    FourHours,
    #[serde(rename = "8h")]
    EightHours,
    #[serde(rename = "12h")]
    TwelveHours,
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "3d")]
    ThreeDays,
    #[serde(rename = "1w")]
    OneWeek,
    #[serde(rename = "1M")]
    OneMonth,
}

/// Order grouping type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderGrouping {
    /// No grouping
    Na,
    /// Normal take profit / stop loss
    NormalTpsl,
    /// Position-based take profit / stop loss
    PositionTpsl,
}

// ============================================================================
// Info Endpoint Request Types
// ============================================================================

/// Request for retrieving all perpetual DEXs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpDexsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
}

impl Default for PerpDexsRequest {
    fn default() -> Self {
        Self {
            request_type: "perpDexs".to_string(),
        }
    }
}

/// Request for perpetuals metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dex: Option<String>,
}

impl Default for MetaRequest {
    fn default() -> Self {
        Self {
            request_type: "meta".to_string(),
            dex: None,
        }
    }
}

/// Request for metadata and asset contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaAndAssetCtxsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dex: Option<String>,
}

impl Default for MetaAndAssetCtxsRequest {
    fn default() -> Self {
        Self {
            request_type: "metaAndAssetCtxs".to_string(),
            dex: None,
        }
    }
}

/// Request for all perpetual metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllPerpMetasRequest {
    #[serde(rename = "type")]
    pub request_type: String,
}

impl Default for AllPerpMetasRequest {
    fn default() -> Self {
        Self {
            request_type: "allPerpMetas".to_string(),
        }
    }
}

/// Request for spot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotMetaRequest {
    #[serde(rename = "type")]
    pub request_type: String,
}

impl Default for SpotMetaRequest {
    fn default() -> Self {
        Self {
            request_type: "spotMeta".to_string(),
        }
    }
}

/// Request for spot metadata and asset contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotMetaAndAssetCtxsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
}

impl Default for SpotMetaAndAssetCtxsRequest {
    fn default() -> Self {
        Self {
            request_type: "spotMetaAndAssetCtxs".to_string(),
        }
    }
}

/// Request for token details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDetailsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    #[serde(rename = "tokenId")]
    pub token_id: String,
}

impl TokenDetailsRequest {
    pub fn new(token_id: impl Into<String>) -> Self {
        Self {
            request_type: "tokenDetails".to_string(),
            token_id: token_id.into(),
        }
    }
}

/// Request for spot deploy state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotDeployStateRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl SpotDeployStateRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "spotDeployState".to_string(),
            user: user.into(),
        }
    }
}

/// Request for spot pair deploy auction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotPairDeployAuctionStatusRequest {
    #[serde(rename = "type")]
    pub request_type: String,
}

impl Default for SpotPairDeployAuctionStatusRequest {
    fn default() -> Self {
        Self {
            request_type: "spotPairDeployAuctionStatus".to_string(),
        }
    }
}

/// Request for all mid prices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllMidsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dex: Option<String>,
}

impl Default for AllMidsRequest {
    fn default() -> Self {
        Self {
            request_type: "allMids".to_string(),
            dex: None,
        }
    }
}

/// Request for L2 order book snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2BookRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub coin: String,
    #[serde(rename = "nSigFigs", skip_serializing_if = "Option::is_none")]
    pub n_sig_figs: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mantissa: Option<u8>,
}

impl L2BookRequest {
    pub fn new(coin: impl Into<String>) -> Self {
        Self {
            request_type: "l2Book".to_string(),
            coin: coin.into(),
            n_sig_figs: None,
            mantissa: None,
        }
    }

    pub fn with_sig_figs(mut self, n_sig_figs: u8) -> Self {
        self.n_sig_figs = Some(n_sig_figs);
        self
    }

    pub fn with_mantissa(mut self, mantissa: u8) -> Self {
        self.mantissa = Some(mantissa);
        self
    }
}

/// Nested request for candle snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleSnapshotInner {
    pub coin: String,
    pub interval: CandleInterval,
    #[serde(rename = "startTime")]
    pub start_time: u64,
    #[serde(rename = "endTime", skip_serializing_if = "Option::is_none")]
    pub end_time: Option<u64>,
}

/// Request for candle snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleSnapshotRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub req: CandleSnapshotInner,
}

impl CandleSnapshotRequest {
    pub fn new(coin: impl Into<String>, interval: CandleInterval, start_time: u64) -> Self {
        Self {
            request_type: "candleSnapshot".to_string(),
            req: CandleSnapshotInner {
                coin: coin.into(),
                interval,
                start_time,
                end_time: None,
            },
        }
    }

    pub fn with_end_time(mut self, end_time: u64) -> Self {
        self.req.end_time = Some(end_time);
        self
    }
}

/// Request for clearinghouse state (perpetuals account)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearinghouseStateRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dex: Option<String>,
}

impl ClearinghouseStateRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "clearinghouseState".to_string(),
            user: user.into(),
            dex: None,
        }
    }

    pub fn with_dex(mut self, dex: impl Into<String>) -> Self {
        self.dex = Some(dex.into());
        self
    }
}

/// Request for spot clearinghouse state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotClearinghouseStateRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl SpotClearinghouseStateRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "spotClearinghouseState".to_string(),
            user: user.into(),
        }
    }
}

/// Request for open orders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenOrdersRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dex: Option<String>,
}

impl OpenOrdersRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "openOrders".to_string(),
            user: user.into(),
            dex: None,
        }
    }

    pub fn with_dex(mut self, dex: impl Into<String>) -> Self {
        self.dex = Some(dex.into());
        self
    }
}

/// Request for frontend open orders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendOpenOrdersRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dex: Option<String>,
}

impl FrontendOpenOrdersRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "frontendOpenOrders".to_string(),
            user: user.into(),
            dex: None,
        }
    }

    pub fn with_dex(mut self, dex: impl Into<String>) -> Self {
        self.dex = Some(dex.into());
        self
    }
}

/// Request for historical orders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalOrdersRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl HistoricalOrdersRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "historicalOrders".to_string(),
            user: user.into(),
        }
    }
}

/// Order ID - can be numeric or client-provided string
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrderId {
    /// Numeric order ID
    Oid(u64),
    /// Client order ID (hex string)
    Cloid(String),
}

/// Request for order status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStatusRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
    pub oid: OrderId,
}

impl OrderStatusRequest {
    pub fn new(user: impl Into<String>, oid: OrderId) -> Self {
        Self {
            request_type: "orderStatus".to_string(),
            user: user.into(),
            oid,
        }
    }
}

/// Request for user fills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFillsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
    #[serde(rename = "aggregateByTime", skip_serializing_if = "Option::is_none")]
    pub aggregate_by_time: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dex: Option<String>,
}

impl UserFillsRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "userFills".to_string(),
            user: user.into(),
            aggregate_by_time: None,
            dex: None,
        }
    }

    pub fn with_aggregate_by_time(mut self, aggregate: bool) -> Self {
        self.aggregate_by_time = Some(aggregate);
        self
    }

    pub fn with_dex(mut self, dex: impl Into<String>) -> Self {
        self.dex = Some(dex.into());
        self
    }
}

/// Request for user fills by time range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFillsByTimeRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
    #[serde(rename = "startTime")]
    pub start_time: u64,
    #[serde(rename = "endTime", skip_serializing_if = "Option::is_none")]
    pub end_time: Option<u64>,
    #[serde(rename = "aggregateByTime", skip_serializing_if = "Option::is_none")]
    pub aggregate_by_time: Option<bool>,
}

impl UserFillsByTimeRequest {
    pub fn new(user: impl Into<String>, start_time: u64) -> Self {
        Self {
            request_type: "userFillsByTime".to_string(),
            user: user.into(),
            start_time,
            end_time: None,
            aggregate_by_time: None,
        }
    }

    pub fn with_end_time(mut self, end_time: u64) -> Self {
        self.end_time = Some(end_time);
        self
    }

    pub fn with_aggregate_by_time(mut self, aggregate: bool) -> Self {
        self.aggregate_by_time = Some(aggregate);
        self
    }
}

/// Request for user TWAP slice fills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTwapSliceFillsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl UserTwapSliceFillsRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "userTwapSliceFills".to_string(),
            user: user.into(),
        }
    }
}

/// Request for user funding history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFundingRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
    #[serde(rename = "startTime")]
    pub start_time: u64,
    #[serde(rename = "endTime", skip_serializing_if = "Option::is_none")]
    pub end_time: Option<u64>,
}

impl UserFundingRequest {
    pub fn new(user: impl Into<String>, start_time: u64) -> Self {
        Self {
            request_type: "userFunding".to_string(),
            user: user.into(),
            start_time,
            end_time: None,
        }
    }

    pub fn with_end_time(mut self, end_time: u64) -> Self {
        self.end_time = Some(end_time);
        self
    }
}

/// Request for historical funding rates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingHistoryRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub coin: String,
    #[serde(rename = "startTime")]
    pub start_time: u64,
    #[serde(rename = "endTime", skip_serializing_if = "Option::is_none")]
    pub end_time: Option<u64>,
}

impl FundingHistoryRequest {
    pub fn new(coin: impl Into<String>, start_time: u64) -> Self {
        Self {
            request_type: "fundingHistory".to_string(),
            coin: coin.into(),
            start_time,
            end_time: None,
        }
    }

    pub fn with_end_time(mut self, end_time: u64) -> Self {
        self.end_time = Some(end_time);
        self
    }
}

/// Request for predicted funding rates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedFundingsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
}

impl Default for PredictedFundingsRequest {
    fn default() -> Self {
        Self {
            request_type: "predictedFundings".to_string(),
        }
    }
}

/// Request for user non-funding ledger updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNonFundingLedgerUpdatesRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
    #[serde(rename = "startTime")]
    pub start_time: u64,
    #[serde(rename = "endTime", skip_serializing_if = "Option::is_none")]
    pub end_time: Option<u64>,
}

impl UserNonFundingLedgerUpdatesRequest {
    pub fn new(user: impl Into<String>, start_time: u64) -> Self {
        Self {
            request_type: "userNonFundingLedgerUpdates".to_string(),
            user: user.into(),
            start_time,
            end_time: None,
        }
    }

    pub fn with_end_time(mut self, end_time: u64) -> Self {
        self.end_time = Some(end_time);
        self
    }
}

/// Request for rate limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl RateLimitsRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "rateLimits".to_string(),
            user: user.into(),
        }
    }
}

/// Request for portfolio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl PortfolioRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "portfolio".to_string(),
            user: user.into(),
        }
    }
}

/// Request for user fees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeesRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl UserFeesRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "userFees".to_string(),
            user: user.into(),
        }
    }
}

/// Request for user role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRoleRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl UserRoleRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "userRole".to_string(),
            user: user.into(),
        }
    }
}

/// Request for subaccounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubaccountsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl SubaccountsRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "subaccounts".to_string(),
            user: user.into(),
        }
    }
}

/// Request for vault details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultDetailsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    #[serde(rename = "vaultAddress")]
    pub vault_address: String,
}

impl VaultDetailsRequest {
    pub fn new(vault_address: impl Into<String>) -> Self {
        Self {
            request_type: "vaultDetails".to_string(),
            vault_address: vault_address.into(),
        }
    }
}

/// Request for user vaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserVaultsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl UserVaultsRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "userVaults".to_string(),
            user: user.into(),
        }
    }
}

/// Request for builder fee approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderFeeApprovalRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
    pub builder: String,
}

impl BuilderFeeApprovalRequest {
    pub fn new(user: impl Into<String>, builder: impl Into<String>) -> Self {
        Self {
            request_type: "builderFeeApproval".to_string(),
            user: user.into(),
            builder: builder.into(),
        }
    }
}

/// Request for referral info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl ReferralRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "referral".to_string(),
            user: user.into(),
        }
    }
}

/// Request for active asset data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveAssetDataRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
    pub coin: String,
}

impl ActiveAssetDataRequest {
    pub fn new(user: impl Into<String>, coin: impl Into<String>) -> Self {
        Self {
            request_type: "activeAssetData".to_string(),
            user: user.into(),
            coin: coin.into(),
        }
    }
}

/// Request for perps at open interest cap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpsAtOpenInterestCapRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dex: Option<String>,
}

impl Default for PerpsAtOpenInterestCapRequest {
    fn default() -> Self {
        Self {
            request_type: "perpsAtOpenInterestCap".to_string(),
            dex: None,
        }
    }
}

/// Request for perp DEX limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpDexLimitsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub dex: String,
}

impl PerpDexLimitsRequest {
    pub fn new(dex: impl Into<String>) -> Self {
        Self {
            request_type: "perpDexLimits".to_string(),
            dex: dex.into(),
        }
    }
}

/// Request for perp DEX status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpDexStatusRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub dex: String,
}

impl PerpDexStatusRequest {
    pub fn new(dex: impl Into<String>) -> Self {
        Self {
            request_type: "perpDexStatus".to_string(),
            dex: dex.into(),
        }
    }
}

/// Request for perp deploy auction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpDeployAuctionStatusRequest {
    #[serde(rename = "type")]
    pub request_type: String,
}

impl Default for PerpDeployAuctionStatusRequest {
    fn default() -> Self {
        Self {
            request_type: "perpDeployAuctionStatus".to_string(),
        }
    }
}

/// Request for staking delegations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingDelegationsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl StakingDelegationsRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "stakingDelegations".to_string(),
            user: user.into(),
        }
    }
}

/// Request for staking summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingSummaryRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl StakingSummaryRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "stakingSummary".to_string(),
            user: user.into(),
        }
    }
}

/// Request for staking history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingHistoryRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl StakingHistoryRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "stakingHistory".to_string(),
            user: user.into(),
        }
    }
}

/// Request for staking rewards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingRewardsRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl StakingRewardsRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "stakingRewards".to_string(),
            user: user.into(),
        }
    }
}

/// Request for borrow/lend user state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowLendUserStateRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl BorrowLendUserStateRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "borrowLendUserState".to_string(),
            user: user.into(),
        }
    }
}

/// Request for borrow/lend reserve state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowLendReserveStateRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub coin: String,
}

impl BorrowLendReserveStateRequest {
    pub fn new(coin: impl Into<String>) -> Self {
        Self {
            request_type: "borrowLendReserveState".to_string(),
            coin: coin.into(),
        }
    }
}

/// Request for all borrow/lend reserve states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllBorrowLendReserveStatesRequest {
    #[serde(rename = "type")]
    pub request_type: String,
}

impl Default for AllBorrowLendReserveStatesRequest {
    fn default() -> Self {
        Self {
            request_type: "allBorrowLendReserveStates".to_string(),
        }
    }
}

/// Request for HIP-3 state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hip3StateRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

impl Hip3StateRequest {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            request_type: "hip3State".to_string(),
            user: user.into(),
        }
    }
}

// ============================================================================
// Info Endpoint Response Types
// ============================================================================

/// Perpetual asset metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerpAssetMeta {
    pub name: String,
    pub sz_decimals: u8,
    pub max_leverage: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_isolated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_delisted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_mode: Option<String>,
}

/// Margin tier information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginTier {
    pub lower_bound: String,
    pub max_leverage: u32,
}

/// Margin table entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginTableEntry {
    pub description: String,
    #[serde(rename = "marginTiers")]
    pub margin_tiers: Vec<MarginTier>,
}

/// Perpetuals metadata response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerpMetaResponse {
    pub universe: Vec<PerpAssetMeta>,
    pub margin_tables: Vec<(u32, MarginTableEntry)>,
}

/// Asset context for perpetuals
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetCtx {
    pub day_ntl_vlm: String,
    pub funding: String,
    pub impact_pxs: (String, String),
    pub mark_px: String,
    pub mid_px: Option<String>,
    pub open_interest: String,
    pub oracle_px: String,
    pub premium: String,
    pub prev_day_px: String,
}

/// Spot token metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotToken {
    pub name: String,
    pub sz_decimals: u8,
    pub wei_decimals: u8,
    pub index: u32,
    pub token_id: String,
    pub is_canonical: bool,
    pub evm_contract: Option<String>,
    pub full_name: Option<String>,
}

/// Spot pair metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotPair {
    pub name: String,
    pub tokens: Vec<u32>,
    pub index: u32,
    pub is_canonical: bool,
}

/// Spot metadata response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotMetaResponse {
    pub tokens: Vec<SpotToken>,
    pub universe: Vec<SpotPair>,
}

/// L2 book level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2BookLevel {
    pub px: String,
    pub sz: String,
    pub n: u32,
}

/// L2 book response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2BookResponse {
    pub coin: String,
    pub time: u64,
    pub levels: (Vec<L2BookLevel>, Vec<L2BookLevel>),
}

/// Candle data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    /// Close timestamp
    #[serde(rename = "T")]
    pub close_time: u64,
    /// Close price
    pub c: String,
    /// High price
    pub h: String,
    /// Interval
    pub i: String,
    /// Low price
    pub l: String,
    /// Number of trades
    pub n: u64,
    /// Open price
    pub o: String,
    /// Symbol
    pub s: String,
    /// Open timestamp
    pub t: u64,
    /// Volume
    pub v: String,
}

/// Open order
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOrder {
    pub coin: String,
    pub limit_px: String,
    pub oid: u64,
    pub side: Side,
    pub sz: String,
    pub timestamp: u64,
}

/// Frontend open order with additional info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendOpenOrder {
    pub coin: String,
    pub limit_px: String,
    pub oid: u64,
    pub side: Side,
    pub sz: String,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orig_sz: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_px: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloid: Option<String>,
}

/// User fill (trade)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFill {
    pub closed_pnl: String,
    pub coin: String,
    pub crossed: bool,
    pub dir: String,
    pub hash: String,
    pub oid: u64,
    pub px: String,
    pub side: Side,
    pub start_position: String,
    pub sz: String,
    pub time: u64,
    pub fee: String,
    pub fee_token: String,
    pub tid: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builder_fee: Option<String>,
}

/// Order status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStatusResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<OrderStatusInfo>,
}

/// Order status info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderStatusInfo {
    pub order: OrderDetails,
    pub status: OrderStatus,
    pub status_timestamp: u64,
}

/// Order details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetails {
    pub coin: String,
    pub limit_px: String,
    pub oid: u64,
    pub side: Side,
    pub sz: String,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orig_sz: Option<String>,
}

/// Funding delta
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingDelta {
    pub coin: String,
    pub funding_rate: String,
    pub szi: String,
    #[serde(rename = "type")]
    pub delta_type: String,
    pub usdc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_samples: Option<u32>,
}

/// Funding history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingHistoryEntry {
    pub delta: FundingDelta,
    pub hash: String,
    pub time: u64,
}

/// Historical funding rate
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalFundingRate {
    pub coin: String,
    pub funding_rate: String,
    pub premium: String,
    pub time: u64,
}

/// Predicted funding
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictedFunding {
    pub funding_rate: String,
    pub next_funding_time: u64,
}

/// Leverage info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeverageInfo {
    #[serde(rename = "type")]
    pub leverage_type: String,
    pub value: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_usd: Option<String>,
}

/// Cumulative funding
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CumulativeFunding {
    pub all_time: String,
    pub since_change: String,
    pub since_open: String,
}

/// Position info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub coin: String,
    pub cum_funding: CumulativeFunding,
    pub entry_px: String,
    pub leverage: LeverageInfo,
    pub liquidation_px: Option<String>,
    pub margin_used: String,
    pub max_leverage: u32,
    pub position_value: String,
    pub return_on_equity: String,
    pub szi: String,
    pub unrealized_pnl: String,
}

/// Asset position
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetPosition {
    pub position: Position,
    #[serde(rename = "type")]
    pub position_type: String,
}

/// Margin summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginSummary {
    pub account_value: String,
    pub total_margin_used: String,
    pub total_ntl_pos: String,
    pub total_raw_usd: String,
}

/// Clearinghouse state response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearinghouseState {
    pub asset_positions: Vec<AssetPosition>,
    pub cross_maintenance_margin_used: String,
    pub cross_margin_summary: MarginSummary,
    pub margin_summary: MarginSummary,
    pub time: u64,
    pub withdrawable: String,
}

/// Spot balance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotBalance {
    pub coin: String,
    pub token: u32,
    pub hold: String,
    pub total: String,
    pub entry_ntl: String,
}

/// Spot clearinghouse state response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotClearinghouseState {
    pub balances: Vec<SpotBalance>,
}

/// Active asset data response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveAssetData {
    pub user: String,
    pub coin: String,
    pub leverage: LeverageInfo,
    pub max_trade_szs: (String, String),
    pub available_to_trade: (String, String),
    pub mark_px: String,
}

/// Perp DEX info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerpDexInfo {
    pub name: String,
    pub full_name: String,
    pub deployer: String,
    pub oracle_updater: Option<String>,
    pub fee_recipient: Option<String>,
    pub asset_to_streaming_oi_cap: Vec<(String, String)>,
    pub asset_to_funding_multiplier: Vec<(String, String)>,
}

/// Perp deploy auction status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerpDeployAuctionStatus {
    pub start_time_seconds: u64,
    pub duration_seconds: u64,
    pub start_gas: String,
    pub current_gas: String,
    pub end_gas: Option<String>,
}

/// Perp DEX limits
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerpDexLimits {
    pub total_oi_cap: String,
    pub oi_sz_cap_per_perp: String,
    pub max_transfer_ntl: String,
    pub coin_to_oi_cap: Vec<(String, String)>,
}

// ============================================================================
// Exchange Endpoint Types
// ============================================================================

/// Limit order specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitOrderType {
    pub tif: TimeInForce,
}

/// Trigger order specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerOrderType {
    pub is_market: bool,
    pub trigger_px: String,
    pub tpsl: TriggerType,
}

/// Order type specification (limit or trigger)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrderTypeSpec {
    Limit { limit: LimitOrderType },
    Trigger { trigger: TriggerOrderType },
}

/// Individual order for placement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderWire {
    /// Asset index
    pub a: u32,
    /// Is buy
    pub b: bool,
    /// Price
    pub p: String,
    /// Size
    pub s: String,
    /// Reduce only
    pub r: bool,
    /// Order type (limit or trigger)
    pub t: OrderTypeSpec,
    /// Client order ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c: Option<String>,
}

/// Builder fee specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderFee {
    /// Builder address
    pub b: String,
    /// Fee rate
    pub f: u64,
}

/// Order action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub orders: Vec<OrderWire>,
    pub grouping: OrderGrouping,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builder: Option<BuilderFee>,
}

/// Cancel specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelWire {
    /// Asset index
    pub a: u32,
    /// Order ID
    pub o: u64,
}

/// Cancel action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub cancels: Vec<CancelWire>,
}

/// Cancel by CLOID specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelByCloidWire {
    pub asset: u32,
    pub cloid: String,
}

/// Cancel by CLOID action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelByCloidAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub cancels: Vec<CancelByCloidWire>,
}

/// Modify order action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub oid: OrderId,
    pub order: OrderWire,
}

/// Batch modify action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchModifyAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub modifies: Vec<ModifyAction>,
}

/// TWAP order specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwapOrderWire {
    /// Asset index
    pub a: u32,
    /// Is buy
    pub b: bool,
    /// Size
    pub s: String,
    /// Reduce only
    pub r: bool,
    /// Duration in minutes
    pub m: u32,
    /// Randomize
    pub t: bool,
}

/// TWAP order action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwapOrderAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub twap: TwapOrderWire,
}

/// Cancel TWAP action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelTwapAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub a: u32,
    pub t: u64,
}

/// Schedule cancel action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleCancelAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub time: u64,
}

/// Update leverage action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLeverageAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub asset: u32,
    pub is_cross: bool,
    pub leverage: u32,
}

/// Update isolated margin action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateIsolatedMarginAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub asset: u32,
    pub is_buy: bool,
    pub ntli: i64,
}

/// USD transfer action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsdTransferAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub destination: String,
    pub amount: String,
}

/// Spot transfer action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotTransferAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub destination: String,
    pub token: String,
    pub amount: String,
}

/// Withdraw action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub destination: String,
    pub amount: String,
}

/// Spot-perp transfer action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotPerpTransferAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub amount: String,
    pub to_perp: bool,
}

/// Vault deposit action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultDepositAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub vault_address: String,
    pub amount: String,
}

/// Vault withdraw action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultWithdrawAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub vault_address: String,
    pub amount: String,
}

/// Approve agent action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveAgentAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub agent_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
}

/// Approve builder fee action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveBuilderFeeAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub builder: String,
    pub max_fee_rate: String,
}

/// Set HIP-3 enabled action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetHip3EnabledAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub enabled: bool,
}

// ============================================================================
// Exchange Response Types
// ============================================================================

/// Resting order status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestingOrderStatus {
    pub oid: u64,
}

/// Filled order status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilledOrderStatus {
    pub total_sz: String,
    pub avg_px: String,
    pub oid: u64,
}

/// Order placement status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrderPlacementStatus {
    Resting { resting: RestingOrderStatus },
    Filled { filled: FilledOrderStatus },
    Error { error: String },
}

/// Order response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponseData {
    pub statuses: Vec<OrderPlacementStatus>,
}

/// Order response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: OrderResponseData,
}

/// Cancel response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelResponseData {
    pub statuses: Vec<String>,
}

/// Cancel response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: CancelResponseData,
}

/// TWAP running status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwapRunningStatus {
    pub twap_id: u64,
}

/// TWAP order status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TwapOrderStatus {
    Running { running: TwapRunningStatus },
    Error { error: String },
}

/// TWAP response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwapResponseData {
    pub status: TwapOrderStatus,
}

/// TWAP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwapResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: TwapResponseData,
}

/// Generic exchange response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExchangeResponseData {
    Order(OrderResponse),
    Cancel(CancelResponse),
    Twap(TwapResponse),
}

/// Exchange API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ExchangeResponseData>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_in_force_serialization() {
        assert_eq!(serde_json::to_string(&TimeInForce::Gtc).unwrap(), "\"Gtc\"");
        assert_eq!(serde_json::to_string(&TimeInForce::Ioc).unwrap(), "\"Ioc\"");
        assert_eq!(serde_json::to_string(&TimeInForce::Alo).unwrap(), "\"Alo\"");
    }

    #[test]
    fn test_time_in_force_deserialization() {
        assert_eq!(
            serde_json::from_str::<TimeInForce>("\"Gtc\"").unwrap(),
            TimeInForce::Gtc
        );
        assert_eq!(
            serde_json::from_str::<TimeInForce>("\"Ioc\"").unwrap(),
            TimeInForce::Ioc
        );
        assert_eq!(
            serde_json::from_str::<TimeInForce>("\"Alo\"").unwrap(),
            TimeInForce::Alo
        );
    }

    #[test]
    fn test_side_serialization() {
        assert_eq!(serde_json::to_string(&Side::Buy).unwrap(), "\"B\"");
        assert_eq!(serde_json::to_string(&Side::Sell).unwrap(), "\"A\"");
    }

    #[test]
    fn test_side_deserialization() {
        assert_eq!(serde_json::from_str::<Side>("\"B\"").unwrap(), Side::Buy);
        assert_eq!(serde_json::from_str::<Side>("\"A\"").unwrap(), Side::Sell);
    }

    #[test]
    fn test_margin_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&MarginMode::Cross).unwrap(),
            "\"cross\""
        );
        assert_eq!(
            serde_json::to_string(&MarginMode::Isolated).unwrap(),
            "\"isolated\""
        );
    }

    #[test]
    fn test_trigger_type_serialization() {
        assert_eq!(serde_json::to_string(&TriggerType::Tp).unwrap(), "\"tp\"");
        assert_eq!(serde_json::to_string(&TriggerType::Sl).unwrap(), "\"sl\"");
    }

    #[test]
    fn test_order_status_serialization() {
        assert_eq!(
            serde_json::to_string(&OrderStatus::Open).unwrap(),
            "\"open\""
        );
        assert_eq!(
            serde_json::to_string(&OrderStatus::Filled).unwrap(),
            "\"filled\""
        );
        assert_eq!(
            serde_json::to_string(&OrderStatus::Canceled).unwrap(),
            "\"canceled\""
        );
        assert_eq!(
            serde_json::to_string(&OrderStatus::MarginCanceled).unwrap(),
            "\"marginCanceled\""
        );
    }

    #[test]
    fn test_candle_interval_serialization() {
        assert_eq!(
            serde_json::to_string(&CandleInterval::OneMinute).unwrap(),
            "\"1m\""
        );
        assert_eq!(
            serde_json::to_string(&CandleInterval::FifteenMinutes).unwrap(),
            "\"15m\""
        );
        assert_eq!(
            serde_json::to_string(&CandleInterval::OneHour).unwrap(),
            "\"1h\""
        );
        assert_eq!(
            serde_json::to_string(&CandleInterval::OneDay).unwrap(),
            "\"1d\""
        );
        assert_eq!(
            serde_json::to_string(&CandleInterval::OneMonth).unwrap(),
            "\"1M\""
        );
    }

    #[test]
    fn test_order_grouping_serialization() {
        assert_eq!(
            serde_json::to_string(&OrderGrouping::Na).unwrap(),
            "\"na\""
        );
        assert_eq!(
            serde_json::to_string(&OrderGrouping::NormalTpsl).unwrap(),
            "\"normalTpsl\""
        );
        assert_eq!(
            serde_json::to_string(&OrderGrouping::PositionTpsl).unwrap(),
            "\"positionTpsl\""
        );
    }

    #[test]
    fn test_all_mids_request_serialization() {
        let request = AllMidsRequest::default();
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"allMids\""));
    }

    #[test]
    fn test_l2_book_request_serialization() {
        let request = L2BookRequest::new("BTC").with_sig_figs(5);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"l2Book\""));
        assert!(json.contains("\"coin\":\"BTC\""));
        assert!(json.contains("\"nSigFigs\":5"));
    }

    #[test]
    fn test_candle_request_serialization() {
        let request = CandleSnapshotRequest::new("ETH", CandleInterval::OneHour, 1700000000000);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"candleSnapshot\""));
        assert!(json.contains("\"coin\":\"ETH\""));
        assert!(json.contains("\"interval\":\"1h\""));
        assert!(json.contains("\"startTime\":1700000000000"));
    }

    #[test]
    fn test_open_orders_request_serialization() {
        let request =
            OpenOrdersRequest::new("0x1234567890123456789012345678901234567890").with_dex("perp");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"openOrders\""));
        assert!(json.contains("\"user\":\"0x1234567890123456789012345678901234567890\""));
        assert!(json.contains("\"dex\":\"perp\""));
    }

    #[test]
    fn test_order_id_serialization() {
        let oid = OrderId::Oid(12345);
        assert_eq!(serde_json::to_string(&oid).unwrap(), "12345");

        let cloid = OrderId::Cloid("0xabc123".to_string());
        assert_eq!(serde_json::to_string(&cloid).unwrap(), "\"0xabc123\"");
    }

    #[test]
    fn test_order_wire_serialization() {
        let order = OrderWire {
            a: 0,
            b: true,
            p: "50000.0".to_string(),
            s: "0.1".to_string(),
            r: false,
            t: OrderTypeSpec::Limit {
                limit: LimitOrderType {
                    tif: TimeInForce::Gtc,
                },
            },
            c: Some("my-order-1".to_string()),
        };
        let json = serde_json::to_string(&order).unwrap();
        assert!(json.contains("\"a\":0"));
        assert!(json.contains("\"b\":true"));
        assert!(json.contains("\"p\":\"50000.0\""));
        assert!(json.contains("\"tif\":\"Gtc\""));
    }

    #[test]
    fn test_trigger_order_serialization() {
        let order = OrderWire {
            a: 0,
            b: false,
            p: "48000.0".to_string(),
            s: "0.05".to_string(),
            r: true,
            t: OrderTypeSpec::Trigger {
                trigger: TriggerOrderType {
                    is_market: true,
                    trigger_px: "49000.0".to_string(),
                    tpsl: TriggerType::Sl,
                },
            },
            c: None,
        };
        let json = serde_json::to_string(&order).unwrap();
        assert!(json.contains("\"isMarket\":true"));
        assert!(json.contains("\"triggerPx\":\"49000.0\""));
        assert!(json.contains("\"tpsl\":\"sl\""));
    }

    #[test]
    fn test_cancel_wire_serialization() {
        let cancel = CancelWire { a: 0, o: 12345 };
        let json = serde_json::to_string(&cancel).unwrap();
        assert!(json.contains("\"a\":0"));
        assert!(json.contains("\"o\":12345"));
    }

    #[test]
    fn test_twap_order_wire_serialization() {
        let twap = TwapOrderWire {
            a: 0,
            b: true,
            s: "1.0".to_string(),
            r: false,
            m: 60,
            t: true,
        };
        let json = serde_json::to_string(&twap).unwrap();
        assert!(json.contains("\"a\":0"));
        assert!(json.contains("\"m\":60"));
        assert!(json.contains("\"t\":true"));
    }

    #[test]
    fn test_l2_book_level_deserialization() {
        let json = r#"{"px": "50000.0", "sz": "1.5", "n": 10}"#;
        let level: L2BookLevel = serde_json::from_str(json).unwrap();
        assert_eq!(level.px, "50000.0");
        assert_eq!(level.sz, "1.5");
        assert_eq!(level.n, 10);
    }

    #[test]
    fn test_candle_deserialization() {
        let json = r#"{
            "T": 1700000060000,
            "c": "50100.0",
            "h": "50200.0",
            "i": "1m",
            "l": "49900.0",
            "n": 150,
            "o": "50000.0",
            "s": "BTC",
            "t": 1700000000000,
            "v": "10.5"
        }"#;
        let candle: Candle = serde_json::from_str(json).unwrap();
        assert_eq!(candle.close_time, 1700000060000);
        assert_eq!(candle.c, "50100.0");
        assert_eq!(candle.h, "50200.0");
        assert_eq!(candle.l, "49900.0");
        assert_eq!(candle.o, "50000.0");
        assert_eq!(candle.v, "10.5");
        assert_eq!(candle.n, 150);
    }

    #[test]
    fn test_open_order_deserialization() {
        let json = r#"{
            "coin": "BTC",
            "limitPx": "50000.0",
            "oid": 12345,
            "side": "B",
            "sz": "0.1",
            "timestamp": 1700000000000
        }"#;
        let order: OpenOrder = serde_json::from_str(json).unwrap();
        assert_eq!(order.coin, "BTC");
        assert_eq!(order.limit_px, "50000.0");
        assert_eq!(order.oid, 12345);
        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.sz, "0.1");
    }

    #[test]
    fn test_user_fill_deserialization() {
        let json = r#"{
            "closedPnl": "100.0",
            "coin": "BTC",
            "crossed": true,
            "dir": "Open Long",
            "hash": "0xabc123",
            "oid": 12345,
            "px": "50000.0",
            "side": "B",
            "startPosition": "0.0",
            "sz": "0.1",
            "time": 1700000000000,
            "fee": "5.0",
            "feeToken": "USDC",
            "tid": 67890
        }"#;
        let fill: UserFill = serde_json::from_str(json).unwrap();
        assert_eq!(fill.coin, "BTC");
        assert_eq!(fill.px, "50000.0");
        assert_eq!(fill.side, Side::Buy);
        assert_eq!(fill.fee, "5.0");
    }

    #[test]
    fn test_order_placement_status_resting() {
        let json = r#"{"resting": {"oid": 12345}}"#;
        let status: OrderPlacementStatus = serde_json::from_str(json).unwrap();
        match status {
            OrderPlacementStatus::Resting { resting } => {
                assert_eq!(resting.oid, 12345);
            }
            _ => panic!("Expected Resting status"),
        }
    }

    #[test]
    fn test_order_placement_status_filled() {
        let json = r#"{"filled": {"totalSz": "0.1", "avgPx": "50000.0", "oid": 12345}}"#;
        let status: OrderPlacementStatus = serde_json::from_str(json).unwrap();
        match status {
            OrderPlacementStatus::Filled { filled } => {
                assert_eq!(filled.total_sz, "0.1");
                assert_eq!(filled.avg_px, "50000.0");
                assert_eq!(filled.oid, 12345);
            }
            _ => panic!("Expected Filled status"),
        }
    }

    #[test]
    fn test_perp_asset_meta_deserialization() {
        let json = r#"{
            "name": "BTC",
            "szDecimals": 4,
            "maxLeverage": 50,
            "onlyIsolated": false
        }"#;
        let meta: PerpAssetMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.name, "BTC");
        assert_eq!(meta.sz_decimals, 4);
        assert_eq!(meta.max_leverage, 50);
        assert_eq!(meta.only_isolated, Some(false));
    }

    #[test]
    fn test_spot_token_deserialization() {
        let json = r#"{
            "name": "USDC",
            "szDecimals": 2,
            "weiDecimals": 6,
            "index": 0,
            "tokenId": "0x1",
            "isCanonical": true,
            "evmContract": null,
            "fullName": "USD Coin"
        }"#;
        let token: SpotToken = serde_json::from_str(json).unwrap();
        assert_eq!(token.name, "USDC");
        assert_eq!(token.sz_decimals, 2);
        assert_eq!(token.wei_decimals, 6);
        assert!(token.is_canonical);
        assert_eq!(token.full_name, Some("USD Coin".to_string()));
    }

    #[test]
    fn test_position_deserialization() {
        let json = r#"{
            "coin": "BTC",
            "cumFunding": {
                "allTime": "100.0",
                "sinceChange": "10.0",
                "sinceOpen": "5.0"
            },
            "entryPx": "50000.0",
            "leverage": {
                "type": "cross",
                "value": 10,
                "rawUsd": "5000.0"
            },
            "liquidationPx": "45000.0",
            "marginUsed": "500.0",
            "maxLeverage": 50,
            "positionValue": "5000.0",
            "returnOnEquity": "0.1",
            "szi": "0.1",
            "unrealizedPnl": "100.0"
        }"#;
        let position: Position = serde_json::from_str(json).unwrap();
        assert_eq!(position.coin, "BTC");
        assert_eq!(position.entry_px, "50000.0");
        assert_eq!(position.leverage.leverage_type, "cross");
        assert_eq!(position.leverage.value, 10);
    }

    #[test]
    fn test_spot_balance_deserialization() {
        let json = r#"{
            "coin": "USDC",
            "token": 0,
            "hold": "100.0",
            "total": "1000.0",
            "entryNtl": "1000.0"
        }"#;
        let balance: SpotBalance = serde_json::from_str(json).unwrap();
        assert_eq!(balance.coin, "USDC");
        assert_eq!(balance.total, "1000.0");
        assert_eq!(balance.hold, "100.0");
    }
}
