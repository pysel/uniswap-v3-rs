# Uniswap V3 SDK for Rust

[![CI](https://github.com/pysel/uniswap-v3-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/pysel/uniswap-v3-rs/actions/workflows/ci.yml)
[![Security audit](https://github.com/pysel/uniswap-v3-rs/actions/workflows/security.yml/badge.svg?branch=main)](https://github.com/pysel/uniswap-v3-rs/actions/workflows/security.yml)
[![Crates.io](https://img.shields.io/crates/v/uniswap-v3-rs.svg)](https://crates.io/crates/uniswap-v3-rs)

An opinionated, Alloy-native Uniswap V3 SDK for Rust.

The goal: make swaps and concentrated-liquidity positions feel like normal Rust
instead of making every application rebuild contract calls, path encoding, tick math, receipt
decoding, and deployment address lookup from scratch.

## Install

```toml
[dependencies]
uniswap-v3-rs = { version = "0.2", features = ["swap", "positions"] }
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

## Estimate a swap

QuoterV2 is available through the same client and does not need a signer. It simulates the V3 pools
with `eth_call`, so it returns an estimate rather than sending a transaction:

```rust
use alloy_primitives::U256;
use uniswap_v3_rs::{calltypes::QuoteExactInputSingleParams, path};

let path = path!(usdc, 500, weth)?;
let quote = client
    .quote_exact_input_single(
        QuoteExactInputSingleParams::builder(&path)
            .amount_in(U256::from(1_000_000))
            .then_default()
            .build()?,
    )
    .await?;

println!(
    "estimated {} WETH wei, crossed {} initialized ticks",
    quote.amount_out, quote.initialized_ticks_crossed
);
```

Use a fresh quote to choose `amount_out_minimum` for exact-input or `amount_in_maximum` for
exact-output. A quote is not slippage protection by itself: the pool can move before the swap lands.

## Swap USDC for WETH

Quote first, turn the quote into a swap builder, then apply slippage before sending:

```rust
use alloy_primitives::U256;
use uniswap_v3_rs::{
    calltypes::{BPS, ExactInputParamsBuilder, QuoteExactInputParams},
    objects::{TokenExt, USDC, WETH},
    path,
};

let chain_id = client.get_chain_id().await?;
let owner = client.signer_address().expect("client needs a signer");
let usdc = USDC::on_chain(chain_id).expect("USDC is not configured for this chain");
let weth = WETH::on_chain(chain_id).expect("WETH is not configured for this chain");
let router = client.swap_router().expect("no SwapRouter02 for this chain");

usdc.approve_unlimited(client.provider(), router.address()).await?;

let path = path!(usdc, 500, weth)?;
let quote = client
    .quote_exact_input(
        QuoteExactInputParams::builder(&path)
            .amount_in(U256::from(1_000_000)) // 1 USDC
            .build()?,
    )
    .await?;

let params = ExactInputParamsBuilder::from(quote)
    .recipient(owner)
    .apply_amount_out_slippage(BPS::from_percent(1)?)?
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
use uniswap_v3_rs::calltypes::BPS;
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
    .get_both_ticks_away_from_mid(client.provider(), BPS::new(50))
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
