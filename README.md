# Uniswap V3 SDK for Rust

[![CI](https://github.com/pysel/uniswap-v3-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/pysel/uniswap-v3-rs/actions/workflows/ci.yml)
[![Security audit](https://github.com/pysel/uniswap-v3-rs/actions/workflows/security.yml/badge.svg?branch=main)](https://github.com/pysel/uniswap-v3-rs/actions/workflows/security.yml)
[![Crates.io](https://img.shields.io/crates/v/uniswap-v3-rs.svg)](https://crates.io/crates/uniswap-v3-rs)

An opinionated, Alloy-native Uniswap V3 SDK for Rust.

The goal: make swaps and concentrated-liquidity positions feel like normal Rust
instead of making every application rebuild contract calls, path encoding, tick math, receipt
decoding, and deployment address lookup from scratch.

## Table of contents

- [Install](#install)
- [Estimate a swap](#estimate-a-swap)
- [Swap USDC for WETH](#swap-usdc-for-weth)
- [Create a liquidity position](#create-a-liquidity-position)
- [Strategies](#strategies)
  - [Constant window LP](#constant-window-lp)
- [More examples](#more-examples)

## Install

```toml
[dependencies]
uniswap-v3-rs = "0.3.1"
```

Swap, quote, position, and strategy APIs are available by default. Disable strategies and their
WebSocket dependencies with `default-features = false` when you only need the core client:

```toml
[dependencies]
uniswap-v3-rs = { version = "0.3.1", default-features = false }
```

Create a client with an Alloy signer:

```rust
use alloy::signers::local::PrivateKeySigner;
use uniswap_v3_rs::client::UniswapV3Client;

let signer: PrivateKeySigner = std::env::var("PRIVATE_KEY")?.parse()?;
let client = UniswapV3Client::builder()
    .rpc_url(std::env::var("RPC_URL")?)
    .signer(signer)
    .gas_multiplier(1.3) // optional; pads eth_estimateGas before send
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

usdc.approve_unlimited(&client, router.address()).await?;

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

usdc.approve_unlimited(&client, npm.address()).await?;
weth.approve_unlimited(&client, npm.address()).await?;

let pool = client.get_pool(usdc.clone(), weth.clone(), 500).await?;
let (tick_lower, tick_upper) = pool
    .get_both_ticks_away_from_mid(client.provider(), BPS::new(50))
    .await?;

// NPM amounts are token0/token1 ordered, not symbol ordered.
let (amount0, amount1) = if pool.token0().address() == usdc.address() {
    (usdc.from_amount(1.0), weth.from_amount(1.0) / U256::from(1_000))
} else {
    (weth.from_amount(1.0) / U256::from(1_000), usdc.from_amount(1.0))
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

## Strategies

The `strategies` feature (on by default) provides shared strategy and price-source interfaces.

- `BinancePriceSource` — Spot lowercase `baseusdt@bookTicker` stream; latest bid/ask midpoint in a
  Tokio `watch` channel (reconnects if the socket drops).
- `StablePriceSource` — constant `1.0` USD for supported stables via the same `watch` pattern.
- `ConstantWindowStrategy` — keeps a concentrated LP range centered on an external mid
  (`price0_usd / price1_usd`), rebalancing when that mid drifts beyond configured BPS thresholds.

`Strategy::run` returns a Tokio `JoinHandle<Result<(), StrategyError>>`. Callers can await
failures or `abort()` the handle. Aborting the task does **not** close any live NFT.

### Constant window LP

Wire price sources in **pool token order** (token0 / token1), approve the NPM, then run. Each mint
sizes `amount*_desired` as `balance * portfolio_fraction` at open time.

On startup the strategy:

1. Hydrates the pool and both price feeds
2. Closes every owner NFT matching the pool key `(token0, token1, fee)`
3. Validates fractions in `(0, 1]`, rebalance thresholds strictly inside window lengths, prices,
   nonzero sized balances, and existing NPM allowances (it never changes approvals)
4. Loops: mint when none is tracked; hold while mid stays within inclusive rebalance thresholds of
   `open_price`; otherwise close and reopen on the next mid

Example for a WETH/USDC pool where token0 is WETH and token1 is USDC (common on Base):

```rust
use uniswap_v3_rs::{
    calltypes::BPS,
    client::UniswapV3Client,
    objects::{TokenExt, USDC, WETH},
    strategies::{
        BinancePriceSource, ConstantWindowStrategy, StablePriceSource, Strategy,
    },
};

const FEE: u32 = 500;
const WINDOW_BPS: BPS = BPS::new(100);     // ±100 bps around external mid
const REBALANCE_BPS: BPS = BPS::new(50);   // reopen after ±50 bps drift

let chain_id = client.get_chain_id().await?;
let npm = client
    .position_manager()
    .expect("no NonfungiblePositionManager for this chain");
let usdc = USDC::on_chain(chain_id).expect("USDC not deployed on chain");
let weth = WETH::on_chain(chain_id).expect("WETH not deployed on chain");
let pool = client.get_pool(weth.clone(), usdc.clone(), FEE).await?;

// Price sources must match pool token0 / token1 order.
assert_eq!(pool.token0().address(), weth.address());
assert_eq!(pool.token1().address(), usdc.address());

usdc.approve_unlimited(&client, npm.address()).await?;
weth.approve_unlimited(&client, npm.address()).await?;

let mut strategy = ConstantWindowStrategy::builder()
    .length_below_mid(WINDOW_BPS)
    .length_above_mid(WINDOW_BPS)
    .rebalance_below_threshold(REBALANCE_BPS)
    .rebalance_above_threshold(REBALANCE_BPS)
    .max_token0_amount_as_portfolio_fraction(0.95)
    .max_token1_amount_as_portfolio_fraction(0.95)
    .price_source_token0(BinancePriceSource::new()) // WETH → ETHUSDT
    .price_source_token1(StablePriceSource::new())  // USDC → 1.0
    .build()?;

let handle = strategy.run(client, pool.address())?;
let abort = handle.abort_handle();

tokio::select! {
    _ = tokio::signal::ctrl_c() => {
        abort.abort(); // does not close the live NFT
    }
    result = handle => {
        result??;
    }
}
```

If your pool sorts the other way (token0 = USDC, token1 = WETH), swap the price-source assignments
accordingly.

## More examples

There are more focused runnable examples in [`bin/examples`](bin/examples), including listing and
closing positions. The SDK is still young and there are definitely rough edges, but the core swap
and LP flows are here and usable.
