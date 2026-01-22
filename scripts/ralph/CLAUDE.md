# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Hyperliquid Rust SDK & Demo Dashboard - A Rust SDK for interacting with the Hyperliquid API (exchange, info, and WebSocket endpoints) with a Bloomberg-style demo website.

## Development Workflow

This project uses a task-driven workflow managed by `prd.json`:
1. Read `prd.json` for user stories and their status
2. Pick highest priority story where `passes: false`
3. Implement that ONE story
4. Run typecheck and tests
5. Commit: `feat: [ID] - [Title]`
6. Update `prd.json`: set `passes: true`
7. Append learnings to `scripts/ralph/progress.txt`

## Build & Test Commands

Once the Rust SDK is initialized:
```bash
# Build
cargo build

# Run tests
cargo test

# Run single test
cargo test test_name

# Type check
cargo check

# Format
cargo fmt

# Lint
cargo clippy
```

For the demo website (Phase 5):
```bash
# Run backend server
cargo run --bin demo-server
```

## Architecture

### SDK Structure (Planned)
```
src/
├── lib.rs           # Public API re-exports
├── client.rs        # HTTP client wrapper (mainnet/testnet)
├── error.rs         # Custom error types with thiserror
├── types/           # Request/response models with Serde
├── auth.rs          # EIP-712 signing for exchange endpoints
├── info/            # Info endpoint implementations
│   ├── perpetuals.rs
│   ├── spot.rs
│   └── market.rs
├── exchange/        # Exchange action implementations
└── websocket/       # WebSocket subscriptions
```

### API Endpoints
- **Info**: `POST https://api.hyperliquid.xyz/info` (read-only queries)
- **Exchange**: `POST https://api.hyperliquid.xyz/exchange` (authenticated actions)
- **WebSocket**: `wss://api.hyperliquid.xyz/ws` (real-time subscriptions)

### Key Dependencies
- `reqwest` - HTTP client
- `tokio` - Async runtime
- `serde`/`serde_json` - Serialization
- `thiserror` - Error handling
- `ethers` - EIP-712 signing
- `tokio-tungstenite` - WebSocket client

## Ralph Agent Script

Run the automated agent loop:
```bash
./scripts/ralph/ralph.sh --tool claude [max_iterations]
```

The script reads `prd.json`, executes tasks iteratively, and archives completed runs.
