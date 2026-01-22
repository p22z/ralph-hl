# Hyperliquid Rust SDK

A comprehensive Rust SDK for interacting with the [Hyperliquid](https://hyperliquid.xyz) API, including info endpoints, exchange endpoints, and WebSocket subscriptions.

## Features

- **Info Endpoints** - Read-only queries for market data, user state, staking, vaults, and more
- **Exchange Endpoints** - Authenticated trading actions including orders, TWAP, leverage, transfers, staking, and vaults
- **WebSocket Client** - Real-time subscriptions with automatic reconnection and heartbeat
- **EIP-712 Signing** - Full authentication support for exchange endpoints
- **Type-Safe API** - Comprehensive Rust types for all request/response structures
- **Demo Dashboard** - Bloomberg-style web UI demonstrating all SDK functions

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hyperliquid-sdk = { path = "." }
```

## Quick Start

### Info Endpoints (Read-Only)

```rust
use hyperliquid_sdk::{Client, Network};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client for mainnet
    let client = Client::new(Network::Mainnet)?;

    // Get all perpetual DEXs
    let dexs = client.perp_dexs().await?;
    println!("Perp DEXs: {:?}", dexs);

    // Get perpetual metadata
    let meta = client.meta(None).await?;
    println!("Assets: {:?}", meta.universe);

    // Get L2 order book
    let book = client.l2_book("BTC", None, None).await?;
    println!("Best bid: {:?}", book.levels.0.first());

    // Get user clearinghouse state
    let state = client.clearinghouse_state("0x...", None).await?;
    println!("Positions: {:?}", state.asset_positions);

    Ok(())
}
```

### Exchange Endpoints (Authenticated)

```rust
use hyperliquid_sdk::{
    Client, Network, Wallet, LimitOrderBuilder, TimeInForce, OrderGrouping
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(Network::Mainnet)?;

    // Create wallet from private key
    let wallet = Wallet::from_private_key("0x...")?;

    // Build a limit order
    let order = LimitOrderBuilder::new(0, true, "50000.0", "0.01")
        .time_in_force(TimeInForce::Gtc)
        .reduce_only(false)
        .build();

    // Place the order
    let response = client.place_order(&wallet, order, OrderGrouping::Na).await?;
    println!("Order response: {:?}", response);

    // Cancel an order
    let cancel_response = client.cancel_order(&wallet, 0, 12345).await?;
    println!("Cancel response: {:?}", cancel_response);

    Ok(())
}
```

### WebSocket Subscriptions

```rust
use hyperliquid_sdk::{WsClient, Subscription};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create WebSocket client
    let ws = WsClient::mainnet();

    // Connect
    ws.connect().await?;

    // Subscribe to all mid prices
    ws.subscribe_all_mids().await?;

    // Subscribe to L2 book updates
    ws.subscribe_l2_book("BTC").await?;

    // Subscribe to trades
    ws.subscribe_trades("ETH").await?;

    // Receive messages
    loop {
        if let Ok(Some(msg)) = ws.try_recv().await {
            println!("Received: {:?}", msg);
        }
    }
}
```

## API Coverage

### Info Endpoints (48 endpoints)

| Category | Endpoints |
|----------|-----------|
| Perpetuals | `perp_dexs`, `meta`, `meta_and_asset_ctxs`, `all_perp_metas` |
| Spot | `spot_meta`, `spot_meta_and_asset_ctxs`, `token_details`, `spot_deploy_state`, `spot_pair_deploy_auction_status` |
| Market Data | `all_mids`, `l2_book`, `candle` |
| User State | `clearinghouse_state`, `spot_clearinghouse_state`, `open_orders`, `frontend_open_orders`, `historical_orders`, `order_status` |
| Fills | `user_fills`, `user_fills_by_time`, `user_twap_slice_fills` |
| Funding | `user_funding`, `funding_history`, `predicted_fundings` |
| Account | `user_non_funding_ledger_updates`, `rate_limits`, `portfolio`, `user_fees`, `user_role`, `subaccounts` |
| Staking | `staking_delegations`, `staking_summary`, `staking_history`, `staking_rewards` |
| Vaults | `vault_details`, `user_vaults`, `builder_fee_approval`, `referral`, `hip3_state`, `active_asset_data`, `perps_at_open_interest_cap`, `perp_dex_limits`, `perp_dex_status`, `perp_deploy_auction_status` |
| Borrow/Lend | `borrow_lend_user_state`, `borrow_lend_reserve_state`, `all_borrow_lend_reserve_states`, `aligned_quote_token` |

### Exchange Endpoints (27 endpoints)

| Category | Endpoints |
|----------|-----------|
| Orders | `place_order`, `place_batch_orders`, `cancel_order`, `cancel_order_by_cloid`, `cancel_batch_orders`, `cancel_batch_orders_by_cloid`, `modify_order`, `modify_batch_orders` |
| TWAP | `place_twap_order`, `cancel_twap_order` |
| Leverage | `update_leverage`, `update_isolated_margin`, `schedule_cancel` |
| Transfers | `usd_transfer`, `spot_transfer`, `withdraw`, `spot_perp_transfer` |
| Staking | `stake_deposit`, `stake_withdraw`, `delegate`, `undelegate` |
| Vaults | `vault_deposit`, `vault_withdraw` |
| Approvals | `approve_agent`, `approve_builder_fee` |
| Misc | `reserve_request_weight`, `noop`, `set_hip3_enabled` |

### WebSocket Subscriptions (19 channels)

| Category | Subscriptions |
|----------|---------------|
| Market Data | `allMids`, `l2Book`, `trades`, `candle`, `bbo`, `activeAssetCtx` |
| User Data | `notification`, `webData3`, `twapStates`, `clearinghouseState`, `openOrders` |
| User Events | `orderUpdates`, `userEvents`, `userFills`, `userFundings`, `userNonFundingLedgerUpdates`, `activeAssetData`, `userTwapSliceFills`, `userTwapHistory` |

## Running the Demo

The SDK includes a Bloomberg-style demo dashboard that showcases all API endpoints.

### Prerequisites

- Rust 1.70 or later
- Cargo

### Build and Run

```bash
# Build the demo server with the demo feature
cargo build --features demo

# Run the demo server
cargo run --bin demo-server --features demo
```

The demo server will start at `http://127.0.0.1:3000`.

### Demo Features

- **Info Endpoints** - Interactive forms to call all 48 info endpoints with JSON response display
- **Exchange Endpoints** - Forms for all 27 exchange actions with private key input (testnet recommended)
- **WebSocket Demo** - Live connection to Hyperliquid WebSocket with 19 subscription types

### Security Warning

Never use a mainnet private key with real funds in the demo environment. The demo is intended for testing with testnet keys only.

## Running Tests

```bash
# Run all unit tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## Project Structure

```
src/
├── lib.rs           # Public API exports
├── client.rs        # HTTP client wrapper (mainnet/testnet)
├── error.rs         # Custom error types
├── auth.rs          # EIP-712 signing for exchange endpoints
├── types.rs         # Request/response type definitions
├── info/            # Info endpoint implementations
│   ├── perpetuals.rs
│   ├── spot.rs
│   ├── market.rs
│   ├── user.rs
│   ├── orders.rs
│   ├── fills.rs
│   ├── funding.rs
│   ├── account.rs
│   ├── staking.rs
│   ├── vault.rs
│   └── borrow_lend.rs
├── exchange/        # Exchange endpoint implementations
│   ├── orders.rs
│   ├── cancel.rs
│   ├── modify.rs
│   ├── twap.rs
│   ├── schedule_cancel.rs
│   ├── leverage.rs
│   ├── transfer.rs
│   ├── staking.rs
│   ├── vault.rs
│   ├── approval.rs
│   └── misc.rs
├── websocket/       # WebSocket client
│   ├── client.rs
│   └── subscription.rs
└── bin/
    └── demo_server/ # Demo dashboard server
        ├── main.rs
        ├── state.rs
        └── routes.rs
```

## Dependencies

- `reqwest` - HTTP client
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `thiserror` - Error handling
- `ethers` - EIP-712 signing
- `tokio-tungstenite` - WebSocket client
- `rmp-serde` - MessagePack for action hashing
- `axum` (demo only) - Web server framework
- `tower-http` (demo only) - CORS and static files

## License

MIT
