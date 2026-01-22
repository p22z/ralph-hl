//! API routes for the demo server

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{any, get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message as TungsteniteMessage};

use crate::state::AppState;

/// Standard API response wrapper
#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Build API routes
pub fn api_routes() -> Router<AppState> {
    Router::new()
        // Info endpoints
        .nest("/info", info_routes())
        // Exchange endpoints
        .nest("/exchange", exchange_routes())
}

/// WebSocket proxy route
pub fn ws_routes() -> Router<AppState> {
    Router::new().route("/ws", any(ws_handler))
}

// =============================================================================
// Info Routes
// =============================================================================

/// Info endpoint routes
fn info_routes() -> Router<AppState> {
    Router::new()
        // Perpetuals metadata
        .route("/perp-dexs", get(get_perp_dexs))
        .route("/meta", get(get_meta))
        .route("/meta-and-asset-ctxs", get(get_meta_and_asset_ctxs))
        .route("/all-perp-metas", get(get_all_perp_metas))
        // Spot metadata
        .route("/spot-meta", get(get_spot_meta))
        .route(
            "/spot-meta-and-asset-ctxs",
            get(get_spot_meta_and_asset_ctxs),
        )
        .route("/token-details", get(get_token_details))
        .route("/spot-deploy-state", get(get_spot_deploy_state))
        .route(
            "/spot-pair-deploy-auction-status",
            get(get_spot_pair_deploy_auction_status),
        )
        // Market data
        .route("/all-mids", get(get_all_mids))
        .route("/l2-book", get(get_l2_book))
        .route("/candle", get(get_candle))
        // User clearinghouse state
        .route("/clearinghouse-state", get(get_clearinghouse_state))
        .route(
            "/spot-clearinghouse-state",
            get(get_spot_clearinghouse_state),
        )
        // User orders
        .route("/open-orders", get(get_open_orders))
        .route("/frontend-open-orders", get(get_frontend_open_orders))
        .route("/historical-orders", get(get_historical_orders))
        .route("/order-status", get(get_order_status))
        // User fills
        .route("/user-fills", get(get_user_fills))
        .route("/user-fills-by-time", get(get_user_fills_by_time))
        .route("/user-twap-slice-fills", get(get_user_twap_slice_fills))
        // Funding
        .route("/user-funding", get(get_user_funding))
        .route("/funding-history", get(get_funding_history))
        .route("/predicted-fundings", get(get_predicted_fundings))
        // Account info
        .route(
            "/user-non-funding-ledger-updates",
            get(get_user_non_funding_ledger_updates),
        )
        .route("/rate-limits", get(get_rate_limits))
        .route("/portfolio", get(get_portfolio))
        .route("/user-fees", get(get_user_fees))
        .route("/user-role", get(get_user_role))
        .route("/subaccounts", get(get_subaccounts))
        // Staking info
        .route("/staking-delegations", get(get_staking_delegations))
        .route("/staking-summary", get(get_staking_summary))
        .route("/staking-history", get(get_staking_history))
        .route("/staking-rewards", get(get_staking_rewards))
        // Vault and misc info
        .route("/vault-details", get(get_vault_details))
        .route("/user-vaults", get(get_user_vaults))
        .route("/builder-fee-approval", get(get_builder_fee_approval))
        .route("/referral", get(get_referral))
        .route("/hip3-state", get(get_hip3_state))
        .route("/active-asset-data", get(get_active_asset_data))
        .route(
            "/perps-at-open-interest-cap",
            get(get_perps_at_open_interest_cap),
        )
        .route("/perp-dex-limits", get(get_perp_dex_limits))
        .route("/perp-dex-status", get(get_perp_dex_status))
        .route(
            "/perp-deploy-auction-status",
            get(get_perp_deploy_auction_status),
        )
        // Borrow/lend info
        .route("/borrow-lend-user-state", get(get_borrow_lend_user_state))
        .route(
            "/borrow-lend-reserve-state",
            get(get_borrow_lend_reserve_state),
        )
        .route(
            "/all-borrow-lend-reserve-states",
            get(get_all_borrow_lend_reserve_states),
        )
        .route("/aligned-quote-token", get(get_aligned_quote_token))
}

// =============================================================================
// Query Parameter Structs
// =============================================================================

#[derive(Deserialize)]
pub struct OptionalDexQuery {
    pub dex: Option<String>,
}

#[derive(Deserialize)]
pub struct UserQuery {
    pub user: String,
}

#[derive(Deserialize)]
pub struct UserWithOptionalDexQuery {
    pub user: String,
    pub dex: Option<String>,
}

#[derive(Deserialize)]
pub struct TokenIdQuery {
    pub token_id: String,
}

#[derive(Deserialize)]
pub struct CoinQuery {
    pub coin: String,
}

#[derive(Deserialize)]
pub struct L2BookQuery {
    pub coin: String,
    pub n_sig_figs: Option<u8>,
    pub mantissa: Option<u8>,
}

#[derive(Deserialize)]
pub struct CandleQuery {
    pub coin: String,
    pub interval: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
}

#[derive(Deserialize)]
pub struct OrderStatusQuery {
    pub user: String,
    pub oid: Option<u64>,
    pub cloid: Option<String>,
}

#[derive(Deserialize)]
pub struct TimeRangeQuery {
    pub user: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
}

#[derive(Deserialize)]
pub struct CoinTimeRangeQuery {
    pub coin: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
}

#[derive(Deserialize)]
pub struct VaultQuery {
    pub vault_address: String,
}

#[derive(Deserialize)]
pub struct BuilderFeeApprovalQuery {
    pub user: String,
    pub builder: String,
}

#[derive(Deserialize)]
pub struct ActiveAssetDataQuery {
    pub user: String,
    pub coin: String,
}

#[derive(Deserialize)]
pub struct DexQuery {
    pub dex: String,
}

// =============================================================================
// Perpetuals Metadata Endpoints
// =============================================================================

/// GET /api/info/perp-dexs - Get all perpetual DEXs
async fn get_perp_dexs(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.perp_dexs().await {
        Ok(dexs) => Json(ApiResponse::success(dexs)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/meta - Get perpetual metadata
async fn get_meta(
    State(state): State<AppState>,
    Query(params): Query<OptionalDexQuery>,
) -> impl IntoResponse {
    match state.client.meta(params.dex.as_deref()).await {
        Ok(meta) => Json(ApiResponse::success(meta)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/meta-and-asset-ctxs - Get perpetual metadata with asset contexts
async fn get_meta_and_asset_ctxs(
    State(state): State<AppState>,
    Query(params): Query<OptionalDexQuery>,
) -> impl IntoResponse {
    match state
        .client
        .meta_and_asset_ctxs(params.dex.as_deref())
        .await
    {
        Ok((meta, ctxs)) => Json(ApiResponse::success(serde_json::json!({
            "meta": meta,
            "assetCtxs": ctxs
        })))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/all-perp-metas - Get all perpetual metadata
async fn get_all_perp_metas(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.all_perp_metas().await {
        Ok(metas) => Json(ApiResponse::success(metas)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

// =============================================================================
// Spot Metadata Endpoints
// =============================================================================

/// GET /api/info/spot-meta - Get spot metadata
async fn get_spot_meta(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.spot_meta().await {
        Ok(meta) => Json(ApiResponse::success(meta)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/spot-meta-and-asset-ctxs - Get spot metadata with asset contexts
async fn get_spot_meta_and_asset_ctxs(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.spot_meta_and_asset_ctxs().await {
        Ok((meta, ctxs)) => Json(ApiResponse::success(serde_json::json!({
            "meta": meta,
            "assetCtxs": ctxs
        })))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/token-details - Get token details
async fn get_token_details(
    State(state): State<AppState>,
    Query(params): Query<TokenIdQuery>,
) -> impl IntoResponse {
    match state.client.token_details(&params.token_id).await {
        Ok(details) => Json(ApiResponse::success(details)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/spot-deploy-state - Get spot deploy state
async fn get_spot_deploy_state(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.spot_deploy_state(&params.user).await {
        Ok(state_data) => Json(ApiResponse::success(state_data)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/spot-pair-deploy-auction-status - Get spot pair deploy auction status
async fn get_spot_pair_deploy_auction_status(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.spot_pair_deploy_auction_status().await {
        Ok(status) => Json(ApiResponse::success(status)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

// =============================================================================
// Market Data Endpoints
// =============================================================================

/// GET /api/info/all-mids - Get all mid prices
async fn get_all_mids(
    State(state): State<AppState>,
    Query(params): Query<OptionalDexQuery>,
) -> impl IntoResponse {
    match state.client.all_mids(params.dex.as_deref()).await {
        Ok(mids) => Json(ApiResponse::success(mids)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/l2-book - Get L2 order book
async fn get_l2_book(
    State(state): State<AppState>,
    Query(params): Query<L2BookQuery>,
) -> impl IntoResponse {
    match state
        .client
        .l2_book(&params.coin, params.n_sig_figs, params.mantissa)
        .await
    {
        Ok(book) => Json(ApiResponse::success(book)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/candle - Get candlestick data
async fn get_candle(
    State(state): State<AppState>,
    Query(params): Query<CandleQuery>,
) -> impl IntoResponse {
    use hyperliquid_sdk::CandleInterval;

    let interval = match params.interval.as_str() {
        "1m" => CandleInterval::OneMinute,
        "3m" => CandleInterval::ThreeMinutes,
        "5m" => CandleInterval::FiveMinutes,
        "15m" => CandleInterval::FifteenMinutes,
        "30m" => CandleInterval::ThirtyMinutes,
        "1h" => CandleInterval::OneHour,
        "2h" => CandleInterval::TwoHours,
        "4h" => CandleInterval::FourHours,
        "8h" => CandleInterval::EightHours,
        "12h" => CandleInterval::TwelveHours,
        "1d" => CandleInterval::OneDay,
        "3d" => CandleInterval::ThreeDays,
        "1w" => CandleInterval::OneWeek,
        "1M" => CandleInterval::OneMonth,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid interval: {}. Valid values: 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 8h, 12h, 1d, 3d, 1w, 1M",
                    params.interval
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .candle(&params.coin, interval, params.start_time, params.end_time)
        .await
    {
        Ok(candles) => Json(ApiResponse::success(candles)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

// =============================================================================
// User Clearinghouse State Endpoints
// =============================================================================

/// GET /api/info/clearinghouse-state - Get user clearinghouse state
async fn get_clearinghouse_state(
    State(state): State<AppState>,
    Query(params): Query<UserWithOptionalDexQuery>,
) -> impl IntoResponse {
    match state
        .client
        .clearinghouse_state(&params.user, params.dex.as_deref())
        .await
    {
        Ok(ch_state) => Json(ApiResponse::success(ch_state)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/spot-clearinghouse-state - Get user spot clearinghouse state
async fn get_spot_clearinghouse_state(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.spot_clearinghouse_state(&params.user).await {
        Ok(ch_state) => Json(ApiResponse::success(ch_state)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

// =============================================================================
// User Orders Endpoints
// =============================================================================

/// GET /api/info/open-orders - Get user open orders
async fn get_open_orders(
    State(state): State<AppState>,
    Query(params): Query<UserWithOptionalDexQuery>,
) -> impl IntoResponse {
    match state
        .client
        .open_orders(&params.user, params.dex.as_deref())
        .await
    {
        Ok(orders) => Json(ApiResponse::success(orders)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/frontend-open-orders - Get user frontend open orders
async fn get_frontend_open_orders(
    State(state): State<AppState>,
    Query(params): Query<UserWithOptionalDexQuery>,
) -> impl IntoResponse {
    match state
        .client
        .frontend_open_orders(&params.user, params.dex.as_deref())
        .await
    {
        Ok(orders) => Json(ApiResponse::success(orders)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/historical-orders - Get user historical orders
async fn get_historical_orders(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.historical_orders(&params.user).await {
        Ok(orders) => Json(ApiResponse::success(orders)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/order-status - Get order status by ID
async fn get_order_status(
    State(state): State<AppState>,
    Query(params): Query<OrderStatusQuery>,
) -> impl IntoResponse {
    use hyperliquid_sdk::OrderId;

    let oid = if let Some(oid) = params.oid {
        OrderId::Oid(oid)
    } else if let Some(cloid) = params.cloid {
        OrderId::Cloid(cloid)
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(
                "Either 'oid' or 'cloid' parameter is required".to_string(),
            )),
        )
            .into_response();
    };

    match state.client.order_status(&params.user, oid).await {
        Ok(status) => Json(ApiResponse::success(status)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

// =============================================================================
// User Fills Endpoints
// =============================================================================

/// GET /api/info/user-fills - Get user fills
async fn get_user_fills(
    State(state): State<AppState>,
    Query(params): Query<UserWithOptionalDexQuery>,
) -> impl IntoResponse {
    match state
        .client
        .user_fills(&params.user, params.dex.as_deref())
        .await
    {
        Ok(fills) => Json(ApiResponse::success(fills)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/user-fills-by-time - Get user fills by time range
async fn get_user_fills_by_time(
    State(state): State<AppState>,
    Query(params): Query<TimeRangeQuery>,
) -> impl IntoResponse {
    match state
        .client
        .user_fills_by_time(&params.user, params.start_time, params.end_time)
        .await
    {
        Ok(fills) => Json(ApiResponse::success(fills)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/user-twap-slice-fills - Get user TWAP slice fills
async fn get_user_twap_slice_fills(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.user_twap_slice_fills(&params.user).await {
        Ok(fills) => Json(ApiResponse::success(fills)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

// =============================================================================
// Funding Endpoints
// =============================================================================

/// GET /api/info/user-funding - Get user funding history
async fn get_user_funding(
    State(state): State<AppState>,
    Query(params): Query<TimeRangeQuery>,
) -> impl IntoResponse {
    match state
        .client
        .user_funding(&params.user, params.start_time, params.end_time)
        .await
    {
        Ok(funding) => Json(ApiResponse::success(funding)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/funding-history - Get funding rate history
async fn get_funding_history(
    State(state): State<AppState>,
    Query(params): Query<CoinTimeRangeQuery>,
) -> impl IntoResponse {
    match state
        .client
        .funding_history(&params.coin, params.start_time, params.end_time)
        .await
    {
        Ok(history) => Json(ApiResponse::success(history)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/predicted-fundings - Get predicted funding rates
async fn get_predicted_fundings(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.predicted_fundings().await {
        Ok(fundings) => Json(ApiResponse::success(fundings)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

// =============================================================================
// Account Info Endpoints
// =============================================================================

/// GET /api/info/user-non-funding-ledger-updates - Get user non-funding ledger updates
async fn get_user_non_funding_ledger_updates(
    State(state): State<AppState>,
    Query(params): Query<TimeRangeQuery>,
) -> impl IntoResponse {
    match state
        .client
        .user_non_funding_ledger_updates(&params.user, params.start_time, params.end_time)
        .await
    {
        Ok(updates) => Json(ApiResponse::success(updates)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/rate-limits - Get user rate limits
async fn get_rate_limits(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.rate_limits(&params.user).await {
        Ok(limits) => Json(ApiResponse::success(limits)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/portfolio - Get user portfolio
async fn get_portfolio(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.portfolio(&params.user).await {
        Ok(portfolio) => Json(ApiResponse::success(portfolio)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/user-fees - Get user fees
async fn get_user_fees(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.user_fees(&params.user).await {
        Ok(fees) => Json(ApiResponse::success(fees)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/user-role - Get user role
async fn get_user_role(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.user_role(&params.user).await {
        Ok(role) => Json(ApiResponse::success(role)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/subaccounts - Get user subaccounts
async fn get_subaccounts(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.subaccounts(&params.user).await {
        Ok(subaccounts) => Json(ApiResponse::success(subaccounts)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

// =============================================================================
// Staking Info Endpoints
// =============================================================================

/// GET /api/info/staking-delegations - Get user staking delegations
async fn get_staking_delegations(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.staking_delegations(&params.user).await {
        Ok(delegations) => Json(ApiResponse::success(delegations)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/staking-summary - Get user staking summary
async fn get_staking_summary(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.staking_summary(&params.user).await {
        Ok(summary) => Json(ApiResponse::success(summary)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/staking-history - Get user staking history
async fn get_staking_history(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.staking_history(&params.user).await {
        Ok(history) => Json(ApiResponse::success(history)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/staking-rewards - Get user staking rewards
async fn get_staking_rewards(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.staking_rewards(&params.user).await {
        Ok(rewards) => Json(ApiResponse::success(rewards)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

// =============================================================================
// Vault and Misc Info Endpoints
// =============================================================================

/// GET /api/info/vault-details - Get vault details
async fn get_vault_details(
    State(state): State<AppState>,
    Query(params): Query<VaultQuery>,
) -> impl IntoResponse {
    match state.client.vault_details(&params.vault_address).await {
        Ok(details) => Json(ApiResponse::success(details)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/user-vaults - Get user vaults
async fn get_user_vaults(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.user_vaults(&params.user).await {
        Ok(vaults) => Json(ApiResponse::success(vaults)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/builder-fee-approval - Get builder fee approval
async fn get_builder_fee_approval(
    State(state): State<AppState>,
    Query(params): Query<BuilderFeeApprovalQuery>,
) -> impl IntoResponse {
    match state
        .client
        .builder_fee_approval(&params.user, &params.builder)
        .await
    {
        Ok(approval) => Json(ApiResponse::success(approval)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/referral - Get user referral info
async fn get_referral(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.referral(&params.user).await {
        Ok(referral) => Json(ApiResponse::success(referral)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/hip3-state - Get HIP-3 state
async fn get_hip3_state(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.hip3_state(&params.user).await {
        Ok(hip3_state) => Json(ApiResponse::success(hip3_state)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/active-asset-data - Get active asset data for user
async fn get_active_asset_data(
    State(state): State<AppState>,
    Query(params): Query<ActiveAssetDataQuery>,
) -> impl IntoResponse {
    match state
        .client
        .active_asset_data(&params.user, &params.coin)
        .await
    {
        Ok(data) => Json(ApiResponse::success(data)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/perps-at-open-interest-cap - Get perps at open interest cap
async fn get_perps_at_open_interest_cap(
    State(state): State<AppState>,
    Query(params): Query<OptionalDexQuery>,
) -> impl IntoResponse {
    match state
        .client
        .perps_at_open_interest_cap(params.dex.as_deref())
        .await
    {
        Ok(perps) => Json(ApiResponse::success(perps)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/perp-dex-limits - Get perp DEX limits
async fn get_perp_dex_limits(
    State(state): State<AppState>,
    Query(params): Query<DexQuery>,
) -> impl IntoResponse {
    match state.client.perp_dex_limits(&params.dex).await {
        Ok(limits) => Json(ApiResponse::success(limits)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/perp-dex-status - Get perp DEX status
async fn get_perp_dex_status(
    State(state): State<AppState>,
    Query(params): Query<DexQuery>,
) -> impl IntoResponse {
    match state.client.perp_dex_status(&params.dex).await {
        Ok(status) => Json(ApiResponse::success(status)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/perp-deploy-auction-status - Get perp deploy auction status
async fn get_perp_deploy_auction_status(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.perp_deploy_auction_status().await {
        Ok(status) => Json(ApiResponse::success(status)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

// =============================================================================
// Borrow/Lend Info Endpoints
// =============================================================================

/// GET /api/info/borrow-lend-user-state - Get user borrow/lend state
async fn get_borrow_lend_user_state(
    State(state): State<AppState>,
    Query(params): Query<UserQuery>,
) -> impl IntoResponse {
    match state.client.borrow_lend_user_state(&params.user).await {
        Ok(state_data) => Json(ApiResponse::success(state_data)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/borrow-lend-reserve-state - Get borrow/lend reserve state
async fn get_borrow_lend_reserve_state(
    State(state): State<AppState>,
    Query(params): Query<CoinQuery>,
) -> impl IntoResponse {
    match state.client.borrow_lend_reserve_state(&params.coin).await {
        Ok(state_data) => Json(ApiResponse::success(state_data)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/all-borrow-lend-reserve-states - Get all borrow/lend reserve states
async fn get_all_borrow_lend_reserve_states(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.all_borrow_lend_reserve_states().await {
        Ok(states) => Json(ApiResponse::success(states)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/info/aligned-quote-token - Get aligned quote token status
async fn get_aligned_quote_token(
    State(state): State<AppState>,
    Query(params): Query<CoinQuery>,
) -> impl IntoResponse {
    match state.client.aligned_quote_token(&params.coin).await {
        Ok(token) => Json(ApiResponse::success(token)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

// =============================================================================
// Exchange Routes
// =============================================================================

/// Exchange endpoint routes (require wallet/private key for authentication)
fn exchange_routes() -> Router<AppState> {
    Router::new()
        // Order management
        .route("/place-order", post(post_place_order))
        .route("/place-batch-orders", post(post_place_batch_orders))
        .route("/cancel-order", post(post_cancel_order))
        .route("/cancel-batch-orders", post(post_cancel_batch_orders))
        .route("/cancel-order-by-cloid", post(post_cancel_order_by_cloid))
        .route("/modify-order", post(post_modify_order))
        .route("/modify-batch-orders", post(post_modify_batch_orders))
        // TWAP orders
        .route("/place-twap-order", post(post_place_twap_order))
        .route("/cancel-twap-order", post(post_cancel_twap_order))
        // Schedule cancel (dead man's switch)
        .route("/schedule-cancel", post(post_schedule_cancel))
        // Leverage and margin
        .route("/update-leverage", post(post_update_leverage))
        .route("/update-isolated-margin", post(post_update_isolated_margin))
        // Transfers
        .route("/usd-transfer", post(post_usd_transfer))
        .route("/spot-transfer", post(post_spot_transfer))
        .route("/withdraw", post(post_withdraw))
        .route("/spot-perp-transfer", post(post_spot_perp_transfer))
        // Staking
        .route("/stake-deposit", post(post_stake_deposit))
        .route("/stake-withdraw", post(post_stake_withdraw))
        .route("/delegate", post(post_delegate))
        .route("/undelegate", post(post_undelegate))
        // Vault
        .route("/vault-deposit", post(post_vault_deposit))
        .route("/vault-withdraw", post(post_vault_withdraw))
        // Approvals
        .route("/approve-agent", post(post_approve_agent))
        .route("/approve-builder-fee", post(post_approve_builder_fee))
        // Misc
        .route("/reserve-request-weight", post(post_reserve_request_weight))
        .route("/noop", post(post_noop))
        .route("/set-hip3-enabled", post(post_set_hip3_enabled))
}

// =============================================================================
// Exchange Request Structs
// =============================================================================

/// Common base for authenticated requests
#[derive(Deserialize)]
pub struct AuthenticatedRequest<T> {
    /// Private key (hex string with or without 0x prefix)
    /// WARNING: Never use a mainnet private key with real funds in a demo environment!
    pub private_key: String,
    /// Whether to use mainnet (true) or testnet (false)
    #[serde(default = "default_mainnet")]
    pub mainnet: bool,
    /// The request payload
    #[serde(flatten)]
    pub payload: T,
}

fn default_mainnet() -> bool {
    true
}

#[derive(Deserialize)]
pub struct PlaceOrderRequest {
    pub asset: u32,
    pub is_buy: bool,
    pub price: String,
    pub size: String,
    #[serde(default)]
    pub reduce_only: bool,
    pub time_in_force: Option<String>,
    pub client_order_id: Option<String>,
    pub trigger_price: Option<String>,
    pub trigger_type: Option<String>,
    pub vault_address: Option<String>,
}

#[derive(Deserialize)]
pub struct PlaceBatchOrdersRequest {
    pub orders: Vec<PlaceOrderRequest>,
    pub vault_address: Option<String>,
}

#[derive(Deserialize)]
pub struct CancelOrderRequest {
    pub asset: u32,
    pub oid: u64,
    pub vault_address: Option<String>,
}

#[derive(Deserialize)]
pub struct CancelBatchOrdersRequest {
    pub cancels: Vec<CancelOrderRequest>,
    pub vault_address: Option<String>,
}

#[derive(Deserialize)]
pub struct CancelOrderByCloidRequest {
    pub asset: u32,
    pub cloid: String,
    pub vault_address: Option<String>,
}

#[derive(Deserialize)]
pub struct ModifyOrderRequest {
    pub oid: u64,
    pub asset: u32,
    pub is_buy: bool,
    pub price: String,
    pub size: String,
    #[serde(default)]
    pub reduce_only: bool,
    pub time_in_force: Option<String>,
    pub client_order_id: Option<String>,
    pub trigger_price: Option<String>,
    pub trigger_type: Option<String>,
    pub vault_address: Option<String>,
}

#[derive(Deserialize)]
pub struct ModifyBatchOrdersRequest {
    pub modifications: Vec<ModifyOrderRequest>,
    pub vault_address: Option<String>,
}

#[derive(Deserialize)]
pub struct PlaceTwapOrderRequest {
    pub asset: u32,
    pub is_buy: bool,
    pub size: String,
    pub duration_minutes: u32,
    #[serde(default)]
    pub randomize: bool,
    #[serde(default)]
    pub reduce_only: bool,
    pub vault_address: Option<String>,
}

#[derive(Deserialize)]
pub struct CancelTwapOrderRequest {
    pub asset: u32,
    pub twap_id: u64,
    pub vault_address: Option<String>,
}

#[derive(Deserialize)]
pub struct ScheduleCancelRequest {
    pub timestamp_ms: u64,
    pub vault_address: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateLeverageRequest {
    pub asset: u32,
    pub is_cross: bool,
    pub leverage: u32,
    pub vault_address: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateIsolatedMarginRequest {
    pub asset: u32,
    pub is_buy: bool,
    pub ntli: i64,
    pub vault_address: Option<String>,
}

#[derive(Deserialize)]
pub struct UsdTransferRequest {
    pub destination: String,
    pub amount: String,
}

#[derive(Deserialize)]
pub struct SpotTransferRequest {
    pub destination: String,
    pub token: String,
    pub amount: String,
}

#[derive(Deserialize)]
pub struct WithdrawRequest {
    pub destination: String,
    pub amount: String,
}

#[derive(Deserialize)]
pub struct SpotPerpTransferRequest {
    pub amount: String,
    pub to_perp: bool,
}

#[derive(Deserialize)]
pub struct StakeRequest {
    pub wei_amount: String,
}

#[derive(Deserialize)]
pub struct DelegateRequest {
    pub validator: String,
    pub amount: String,
}

#[derive(Deserialize)]
pub struct VaultDepositWithdrawRequest {
    pub vault_address: String,
    pub amount: String,
}

#[derive(Deserialize)]
pub struct ApproveAgentRequest {
    pub agent_address: String,
    pub agent_name: Option<String>,
}

#[derive(Deserialize)]
pub struct ApproveBuilderFeeRequest {
    pub builder: String,
    pub max_fee_rate: String,
}

#[derive(Deserialize)]
pub struct ReserveRequestWeightRequest {
    pub weight: u64,
}

#[derive(Deserialize)]
pub struct SetHip3EnabledRequest {
    pub enabled: bool,
}

// =============================================================================
// Exchange Endpoint Handlers
// =============================================================================

/// POST /api/exchange/place-order - Place a single order
async fn post_place_order(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<PlaceOrderRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::{
        LimitOrderBuilder, OrderGrouping, TimeInForce, TriggerOrderBuilder, TriggerType, Wallet,
    };

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    let payload = &req.payload;

    // Build order based on whether it's a trigger order or limit order
    let order = if let Some(trigger_px) = &payload.trigger_price {
        let trigger_type = match payload.trigger_type.as_deref() {
            Some("tp") => TriggerType::Tp,
            Some("sl") => TriggerType::Sl,
            _ => TriggerType::Sl,
        };
        let mut builder = TriggerOrderBuilder::new(
            payload.asset,
            payload.is_buy,
            &payload.price,
            &payload.size,
            trigger_px,
            trigger_type,
        );
        builder = builder.reduce_only(payload.reduce_only);
        if let Some(cloid) = &payload.client_order_id {
            builder = builder.client_order_id(cloid);
        }
        builder.build()
    } else {
        let tif = match payload.time_in_force.as_deref() {
            Some("Ioc") | Some("ioc") | Some("IOC") => TimeInForce::Ioc,
            Some("Alo") | Some("alo") | Some("ALO") => TimeInForce::Alo,
            _ => TimeInForce::Gtc,
        };
        let mut builder =
            LimitOrderBuilder::new(payload.asset, payload.is_buy, &payload.price, &payload.size);
        builder = builder.time_in_force(tif).reduce_only(payload.reduce_only);
        if let Some(cloid) = &payload.client_order_id {
            builder = builder.client_order_id(cloid);
        }
        builder.build()
    };

    match state
        .client
        .place_order(&wallet, order, OrderGrouping::Na)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/place-batch-orders - Place multiple orders
async fn post_place_batch_orders(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<PlaceBatchOrdersRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::{
        LimitOrderBuilder, OrderGrouping, TimeInForce, TriggerOrderBuilder, TriggerType, Wallet,
    };

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    let orders: Vec<_> = req
        .payload
        .orders
        .iter()
        .map(|o| {
            if let Some(trigger_px) = &o.trigger_price {
                let trigger_type = match o.trigger_type.as_deref() {
                    Some("tp") => TriggerType::Tp,
                    Some("sl") => TriggerType::Sl,
                    _ => TriggerType::Sl,
                };
                let mut builder = TriggerOrderBuilder::new(
                    o.asset,
                    o.is_buy,
                    &o.price,
                    &o.size,
                    trigger_px,
                    trigger_type,
                );
                builder = builder.reduce_only(o.reduce_only);
                if let Some(cloid) = &o.client_order_id {
                    builder = builder.client_order_id(cloid);
                }
                builder.build()
            } else {
                let tif = match o.time_in_force.as_deref() {
                    Some("Ioc") | Some("ioc") | Some("IOC") => TimeInForce::Ioc,
                    Some("Alo") | Some("alo") | Some("ALO") => TimeInForce::Alo,
                    _ => TimeInForce::Gtc,
                };
                let mut builder = LimitOrderBuilder::new(o.asset, o.is_buy, &o.price, &o.size);
                builder = builder.time_in_force(tif).reduce_only(o.reduce_only);
                if let Some(cloid) = &o.client_order_id {
                    builder = builder.client_order_id(cloid);
                }
                builder.build()
            }
        })
        .collect();

    match state
        .client
        .place_batch_orders(&wallet, orders, OrderGrouping::Na)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/cancel-order - Cancel a single order
async fn post_cancel_order(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<CancelOrderRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .cancel_order(&wallet, req.payload.asset, req.payload.oid)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/cancel-batch-orders - Cancel multiple orders
async fn post_cancel_batch_orders(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<CancelBatchOrdersRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    let cancels: Vec<_> = req
        .payload
        .cancels
        .iter()
        .map(|c| (c.asset, c.oid))
        .collect();

    match state.client.cancel_batch_orders(&wallet, cancels).await {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/cancel-order-by-cloid - Cancel an order by client order ID
async fn post_cancel_order_by_cloid(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<CancelOrderByCloidRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .cancel_order_by_cloid(&wallet, req.payload.asset, &req.payload.cloid)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/modify-order - Modify a single order
async fn post_modify_order(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<ModifyOrderRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::{ModifyOrderBuilder, TimeInForce, Wallet};

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    let payload = &req.payload;
    let tif = match payload.time_in_force.as_deref() {
        Some("Ioc") | Some("ioc") | Some("IOC") => TimeInForce::Ioc,
        Some("Alo") | Some("alo") | Some("ALO") => TimeInForce::Alo,
        _ => TimeInForce::Gtc,
    };

    let mut builder = ModifyOrderBuilder::new(
        payload.oid,
        payload.asset,
        payload.is_buy,
        &payload.price,
        &payload.size,
    );
    builder = builder.time_in_force(tif).reduce_only(payload.reduce_only);
    if let Some(cloid) = &payload.client_order_id {
        builder = builder.client_order_id(cloid);
    }

    match state.client.modify_order(&wallet, builder.build()).await {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/modify-batch-orders - Modify multiple orders
async fn post_modify_batch_orders(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<ModifyBatchOrdersRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::{ModifyOrderBuilder, TimeInForce, Wallet};

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    let modifications: Vec<_> = req
        .payload
        .modifications
        .iter()
        .map(|m| {
            let tif = match m.time_in_force.as_deref() {
                Some("Ioc") | Some("ioc") | Some("IOC") => TimeInForce::Ioc,
                Some("Alo") | Some("alo") | Some("ALO") => TimeInForce::Alo,
                _ => TimeInForce::Gtc,
            };
            let mut builder = ModifyOrderBuilder::new(m.oid, m.asset, m.is_buy, &m.price, &m.size);
            builder = builder.time_in_force(tif).reduce_only(m.reduce_only);
            if let Some(cloid) = &m.client_order_id {
                builder = builder.client_order_id(cloid);
            }
            builder.build()
        })
        .collect();

    match state
        .client
        .modify_batch_orders(&wallet, modifications)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/place-twap-order - Place a TWAP order
async fn post_place_twap_order(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<PlaceTwapOrderRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    let payload = &req.payload;
    match state
        .client
        .place_twap_order(
            &wallet,
            payload.asset,
            payload.is_buy,
            &payload.size,
            payload.duration_minutes,
            payload.randomize,
        )
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/cancel-twap-order - Cancel a TWAP order
async fn post_cancel_twap_order(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<CancelTwapOrderRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .cancel_twap_order(&wallet, req.payload.asset, req.payload.twap_id)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/schedule-cancel - Schedule order cancellation (dead man's switch)
async fn post_schedule_cancel(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<ScheduleCancelRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .schedule_cancel(&wallet, req.payload.timestamp_ms)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/update-leverage - Update leverage settings
async fn post_update_leverage(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<UpdateLeverageRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    let payload = &req.payload;
    match state
        .client
        .update_leverage(&wallet, payload.asset, payload.is_cross, payload.leverage)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/update-isolated-margin - Update isolated margin
async fn post_update_isolated_margin(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<UpdateIsolatedMarginRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    let payload = &req.payload;
    match state
        .client
        .update_isolated_margin(&wallet, payload.asset, payload.is_buy, payload.ntli)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/usd-transfer - Transfer USDC
async fn post_usd_transfer(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<UsdTransferRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .usd_transfer(&wallet, &req.payload.destination, &req.payload.amount)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/spot-transfer - Transfer spot tokens
async fn post_spot_transfer(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<SpotTransferRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    let payload = &req.payload;
    match state
        .client
        .spot_transfer(
            &wallet,
            &payload.destination,
            &payload.token,
            &payload.amount,
        )
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/withdraw - Withdraw to L1
async fn post_withdraw(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<WithdrawRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .withdraw(&wallet, &req.payload.destination, &req.payload.amount)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/spot-perp-transfer - Transfer between spot and perp
async fn post_spot_perp_transfer(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<SpotPerpTransferRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .spot_perp_transfer(&wallet, &req.payload.amount, req.payload.to_perp)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/stake-deposit - Deposit to staking
async fn post_stake_deposit(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<StakeRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .stake_deposit(&wallet, &req.payload.wei_amount)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/stake-withdraw - Withdraw from staking
async fn post_stake_withdraw(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<StakeRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .stake_withdraw(&wallet, &req.payload.wei_amount)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/delegate - Delegate to validator
async fn post_delegate(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<DelegateRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .delegate(&wallet, &req.payload.validator, &req.payload.amount)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/undelegate - Undelegate from validator
async fn post_undelegate(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<DelegateRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .undelegate(&wallet, &req.payload.validator, &req.payload.amount)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/vault-deposit - Deposit to vault
async fn post_vault_deposit(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<VaultDepositWithdrawRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .vault_deposit(&wallet, &req.payload.vault_address, &req.payload.amount)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/vault-withdraw - Withdraw from vault
async fn post_vault_withdraw(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<VaultDepositWithdrawRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .vault_withdraw(&wallet, &req.payload.vault_address, &req.payload.amount)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/approve-agent - Approve an API wallet agent
async fn post_approve_agent(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<ApproveAgentRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .approve_agent(
            &wallet,
            &req.payload.agent_address,
            req.payload.agent_name.clone(),
        )
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/approve-builder-fee - Approve builder fee
async fn post_approve_builder_fee(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<ApproveBuilderFeeRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .approve_builder_fee(&wallet, &req.payload.builder, &req.payload.max_fee_rate)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/reserve-request-weight - Reserve rate limit weight
async fn post_reserve_request_weight(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<ReserveRequestWeightRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .reserve_request_weight(&wallet, req.payload.weight)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/noop - No-op action (heartbeat)
async fn post_noop(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<()>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state.client.noop(&wallet).await {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /api/exchange/set-hip3-enabled - Enable/disable HIP-3 abstraction
async fn post_set_hip3_enabled(
    State(state): State<AppState>,
    Json(req): Json<AuthenticatedRequest<SetHip3EnabledRequest>>,
) -> impl IntoResponse {
    use hyperliquid_sdk::Wallet;

    let wallet = match Wallet::from_private_key(&req.private_key, req.mainnet) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!(
                    "Invalid private key: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    match state
        .client
        .set_hip3_enabled(&wallet, req.payload.enabled)
        .await
    {
        Ok(response) => Json(ApiResponse::success(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )
            .into_response(),
    }
}

// =============================================================================
// WebSocket Proxy Handler
// =============================================================================

/// WebSocket proxy handler that forwards messages between client and Hyperliquid
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: AppState) {
    let ws_url = if state.is_mainnet {
        "wss://api.hyperliquid.xyz/ws"
    } else {
        "wss://api.hyperliquid-testnet.xyz/ws"
    };

    let upstream = match connect_async(ws_url).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            tracing::error!("Failed to connect to upstream WebSocket: {}", e);
            return;
        }
    };

    let (mut client_sender, mut client_receiver) = socket.split();
    let (mut upstream_sender, mut upstream_receiver) = upstream.split();

    // Forward messages from client to upstream
    let client_to_upstream = async {
        while let Some(msg) = client_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if upstream_sender
                        .send(TungsteniteMessage::Text(text.to_string()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(Message::Binary(data)) => {
                    if upstream_sender
                        .send(TungsteniteMessage::Binary(data.to_vec()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(Message::Ping(data)) => {
                    if upstream_sender
                        .send(TungsteniteMessage::Ping(data.to_vec()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(Message::Pong(data)) => {
                    if upstream_sender
                        .send(TungsteniteMessage::Pong(data.to_vec()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
            }
        }
    };

    // Forward messages from upstream to client
    let upstream_to_client = async {
        while let Some(msg) = upstream_receiver.next().await {
            match msg {
                Ok(TungsteniteMessage::Text(text)) => {
                    if client_sender
                        .send(Message::Text(text.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(TungsteniteMessage::Binary(data)) => {
                    if client_sender
                        .send(Message::Binary(data.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(TungsteniteMessage::Ping(data)) => {
                    if client_sender
                        .send(Message::Ping(data.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(TungsteniteMessage::Pong(data)) => {
                    if client_sender
                        .send(Message::Pong(data.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(TungsteniteMessage::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }
    };

    // Run both directions concurrently
    tokio::select! {
        _ = client_to_upstream => {},
        _ = upstream_to_client => {},
    }
}
