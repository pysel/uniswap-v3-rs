# Uniswap V3 SDK for Rust

[![CI](https://github.com/pysel/uniswap-v3-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/pysel/uniswap-v3-rs/actions/workflows/ci.yml)
[![Security audit](https://github.com/pysel/uniswap-v3-rs/actions/workflows/security.yml/badge.svg?branch=main)](https://github.com/pysel/uniswap-v3-rs/actions/workflows/security.yml)

An opinionated, Alloy-native Uniswap V3 SDK for Rust.

The goal: make swaps and concentrated-liquidity positions feel like normal Rust
instead of making every application rebuild contract calls, path encoding, tick math, receipt
decoding, and deployment address lookup from scratch.

## Install

```toml
[dependencies]
uniswap-v3-rs = { git = "https://github.com/pysel/uniswap-v3-rs", features = ["swap", "positions"] }
```

Create a client with an Alloy signer:

```rust
use alloy::signers::local::PrivateKeySigner;
use uniswap_v3_rs::client::UniswapV3Client;

let signer: PrivateKeySigner = std::env::var("PRIVATE_KEY")?.parse()?;
let client = UniswapV3Client::builder()
    .rpc_url(std::env::var("RPC_URL")?)
    .signer(signer)
    .build()
    .await?;
```

## Swap USDC for WETH

```rust
use alloy_primitives::U256;
use uniswap_v3_rs::{
    objects::{ExactInputParams, TokenExt, USDC, WETH},
    path,
};

let chain_id = client.get_chain_id().await?;
let owner = client.signer_address().expect("client needs a signer");
let usdc = USDC::on_chain(chain_id).expect("USDC is not configured for this chain");
let weth = WETH::on_chain(chain_id).expect("WETH is not configured for this chain");
let router = client.swap_router().expect("no SwapRouter02 for this chain");

usdc.approve_unlimited(client.provider(), router.address()).await?;

let path = path!(usdc, 500, weth)?;
let params = ExactInputParams::builder(&path)
    .recipient(owner)
    .amount_in(U256::from(1_000_000)) // 1 USDC
    .amount_out_minimum(U256::ZERO)   // demo only: use a quote and real slippage protection
    .build()?;

let response = client.swap_exact_input(params, None).await?;
println!("submitted {}", response.tx_hash);

let amount_out = response.amount_out.await?;
println!("received {amount_out} wei of WETH");
```

The transaction hash is available as soon as the node accepts the transaction. `amount_out` waits
for the receipt and resolves from the actual pool swap event, not an optimistic local estimate.

## Create a liquidity position

This creates a USDC/WETH position spanning roughly 100 bps around the current pool price:

```rust
use alloy_primitives::U256;
use uniswap_sdk_core::prelude::BaseCurrency;
use uniswap_v3_rs::objects::{CreatePositionParams, TokenExt, USDC, WETH};

let chain_id = client.get_chain_id().await?;
let owner = client.signer_address().expect("client needs a signer");
let usdc = USDC::on_chain(chain_id).expect("USDC is not configured for this chain");
let weth = WETH::on_chain(chain_id).expect("WETH is not configured for this chain");
let npm = client
    .position_manager()
    .expect("no NonfungiblePositionManager for this chain");

usdc.approve_unlimited(client.provider(), npm.address()).await?;
weth.approve_unlimited(client.provider(), npm.address()).await?;

let pool = client.get_pool(usdc.clone(), weth.clone(), 500).await?;
let (tick_lower, tick_upper) = pool
    .get_both_ticks_away_from_mid(client.provider(), 50)
    .await?;

// NPM amounts are token0/token1 ordered, not symbol ordered.
let (amount0, amount1) = if pool.token0().address() == usdc.address() {
    (usdc.from_amount(1), weth.from_amount(1) / U256::from(1_000))
} else {
    (weth.from_amount(1) / U256::from(1_000), usdc.from_amount(1))
};

let params = CreatePositionParams::builder(&pool)
    .tick_lower(tick_lower)
    .tick_upper(tick_upper)
    .amount0_desired(amount0)
    .amount1_desired(amount1)
    .recipient(owner)
    .then_default()
    .build()?;

let response = client.create_position(params, None).await?;
println!("submitted {}", response.tx_hash);

let position = response.position.await?;
println!(
    "minted position #{} with {} liquidity",
    position.token_id, position.liquidity
);
```

Position NFTs keep immutable metadata locally—tokens, fee tier, and tick bounds—while mutable values
such as liquidity, ownership, and owed tokens are fetched from chain when requested. This split has
worked out nicely in practice: fewer pointless RPC calls, without handing callers stale position
state.

There are more focused runnable examples in [`bin/examples`](bin/examples), including listing and
closing positions. The SDK is still young and there are definitely rough edges, but the core swap
and LP flows are here and usable.
