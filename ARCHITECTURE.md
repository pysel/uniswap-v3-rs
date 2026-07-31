# Architecture

Opinionated Uniswap V3 SDK crate. Designed for agents and contributors to navigate quickly.

## Stack

- **Alloy** — HTTP provider, signer/wallet, contract bindings (`sol!`)
- **`uniswap-sdk-core`** — offline primitives (`Token`, amounts, addresses); no RPC
- **Tokio** — async runtime; the optional strategy module uses channels and background tasks

## Features

| Feature | Default | Notes |
| --- | --- | --- |
| `strategies` | yes | Enables strategy interfaces, Binance/Stable price streams, and `tracing` logs. |

SwapRouter02, QuoterV2, and NPM APIs are always compiled.

## Layout

```text
Cargo.toml               # lib package + workspace (members: ., bin)
bin/                     # local examples binary
  Cargo.toml
  src/main.rs            # runs ConstantWindowStrategy on local Anvil USDC/WETH
  examples/
    list_positions.rs    # list owner NPM positions
    create_position.rs   # mint a USDC/WETH position NFT
    increase_liquidity.rs # add liquidity to an existing position NFT
    close_position.rs    # close (decrease+collect+burn) by token_id
    swap_with_quote.rs   # quote → swap with slippage
    swap_without_quote.rs # exact-input swap with default min-out
    exact_output.rs      # exact-output single-hop + multi-hop with quote
src/
  lib.rs                 # public modules: calltypes, client, errors, objects, strategies (feature-gated)
  client.rs              # UniswapV3Client (+ builder)
  errors.rs              # UniswapV3Error
  calltypes/
    mod.rs               # re-exports parameter, response, and transaction-future types
    npm/
      mod.rs             # re-exports NPM calltypes
      mint_params.rs
      increase_liquidity_params.rs
      decrease_liquidity_params.rs
      collect_params.rs
      close_position_params.rs
      burn_position_response.rs
      create_and_initialize_pool_response.rs
    bps.rs               # BPS newtype (u16) with percent/neg helpers
    path.rs              # V3 Path/path! construction and packed ABI encoding
    quoter/              # one file per QuoterV2 parameter/result pair
    router/              # one file per SwapRouter02 parameter/response pair
    slippage.rs          # quote→swap amount bound adjustments by BPS
    transaction_future.rs # boxed future returned inside transaction responses
  objects/
    mod.rs               # ABI aliases, public param structs, Factory/Pool/SwapRouter/Position/NPM/tokens
    factory.rs           # Factory: CREATE2 pool address, pool() helper
    npm/
      mod.rs             # exports NonfungiblePositionManager and internal result helpers
      manager.rs         # NonfungiblePositionManager definition and RPC methods
      result.rs          # receipt-backed NPM transaction result futures
    pool.rs              # Pool: immutables + RPC state getters
    position.rs           # Position NFT immutable metadata + live on-chain state methods
    quoter/
      mod.rs             # exports QuoterV2
      quoter.rs          # QuoterV2 definition and eth_call quote methods
    router/
      mod.rs             # exports SwapRouter and internal result helpers
      router.rs          # SwapRouter02 definition and exact-input/output methods
      result.rs          # receipt-backed swap amount futures
    token/
      mod.rs             # re-exports TokenExt + USDC/USDT/WBTC/... registries
      token_extension.rs # TokenExt: RPC metadata, balance/allowance reads, approvals
      usdc.rs            # USDC::on_chain from Uniswap default-token-list
      ...                # usdt, wbtc, uni, usde, usdg, usdt0, link, dai, cbbtc, bnb
    abi_definitions.rs   # Alloy sol! bindings for V3Pool / V3Factory / SwapRouter02 / QuoterV2 / NPM / Erc20 (incl. balanceOf/allowance)
  strategies/            # optional strategy abstractions (feature-gated)
    mod.rs                # Strategy trait + re-exports
    errors.rs             # StrategyError
    utils.rs              # BPS price adjustment helpers
    constant_window/
      mod.rs              # re-exports strategy types
      strategy.rs         # ConstantWindowStrategy + builder + run loop
      position.rs         # in-memory open-price / NFT / tick bookkeeping
    price_source/         # PriceSource, BinancePriceSource, StablePriceSource, PriceSourceError
artifacts/               # JSON ABIs consumed by sol! (pool, factory, SwapRouter02, QuoterV2, NPM)
scripts/
  anvil.sh               # mainnet fork via Anvil
  fund.sh                # fund Anvil account with WETH/USDC/USDT/WBTC
.env                     # local secrets (gitignored)
```

## Object model

| Type | Owns | Notes |
| --- | --- | --- |
| `UniswapV3Client` | `rpc_url`, Alloy `DynProvider`, optional wallet, optional `gas_multiplier`, `Factory`, optional `SwapRouter`, optional `QuoterV2`, optional `NonfungiblePositionManager` | Entry point. Builder resolves factory (required) and optional deployments from RPC chain id. When `gas_multiplier` is set, client-submitted txs estimate gas then pad the limit (e.g. `1.3`) before send. |
| `Factory` | `chain_id`, factory `address` | Offline CREATE2 derivation; `pool()` loads a `Pool` via provider. |
| `Pool` | factory, sorted `token0`/`token1`, `fee`, `tick_spacing` | Address is **derived**, not stored. Mutable state (e.g. `sqrt_price_x96`) fetched via RPC; provides external human-price conversion and directional tick-spacing alignment, and can select a spacing-aligned tick within a conservative signed bps distance from the live token1/token0 midprice. |
| `SwapRouter` | `chain_id`, router `address` | Resolves SwapRouter02 deployments and submits exact-input/output transactions. |
| `QuoterV2` | `chain_id`, quoter `address` | Resolves QuoterV2 deployments and estimates exact-input/output execution with `eth_call`. |
| `NonfungiblePositionManager` | `chain_id`, NPM `address` | Resolves official NPM deployments and submits direct position lifecycle transactions. |
| `Position` | NPM identity, `token_id`, token addresses, fee, immutable tick range | NFT-backed position identity. Liquidity, owed tokens, owner, and collectable amounts are always fetched from chain. |
| `Path` | initial token, ordered token/fee hops | Builds and encodes exact-input or reversed exact-output V3 paths. |
| `Token` | from `uniswap-sdk-core` | Foreign type; RPC hydrate via `TokenExt` (orphan-rule extension trait). `TokenExt` reads/approvals take `&UniswapV3Client`. |
| `USDC` / `USDT` / … | unit structs | Offline `on_chain(chain_id)` registries sourced from Uniswap default-token-list for mainnet/arbitrum/base/avalanche/optimism/polygon/tempo. |
| `BinancePriceSource` | no fields | Optional Spot `BASEUSDT@bookTicker` source; keeps the latest bid/ask midpoint in a Tokio `watch` channel. |
| `StablePriceSource` | no fields | Optional constant `1.0` USD source for supported stablecoins via a Tokio `watch` channel. |

### Construction paths

1. **Offline / known metadata** — `token!` / `Token::new`, `Factory::from_chain`, `Pool::new`, `SwapRouter::from_chain`, `QuoterV2::from_chain`, `NonfungiblePositionManager::from_chain`
2. **From chain** — `Pool::from_address`, `Token::from_address` (needs client); client `get_pool(token_a, token_b, fee)` → factory CREATE2 → `Pool::from_address`
3. **Position NFTs** — client `get_position(token_id)` reads NPM once for immutable NFT metadata. `Position::state`, `Position::liquidity`, `Position::tokens_owed`, and `Position::collectable_amounts` refetch mutable state every call.

Pool address derivation: `CREATE2(factory, keccak256(abi.encode(token0, token1, fee)), init_code_hash)` with `token0 < token1`. Init-code hash is an internal constant (zkSync uses a different hash / CREATE2 scheme).

Position lifecycle: `create_position` mints a new NFT, `increase_position_liquidity` adds liquidity to the same immutable tick range, `decrease_position_liquidity` credits withdrawn amounts to NPM owed balances, `collect_position` transfers owed balances, and `close_position` atomically decreases all current liquidity, collects, and burns the empty NFT.

Write methods return typed responses as soon as the transaction is accepted by the provider. Each response exposes `tx_hash` immediately and a typed future (for example, `amount_out`, `position`, or `amounts`) that waits for the receipt and resolves the actual event-backed Solidity result.

QuoterV2 returns the estimated amount, per-pool post-swap price, crossed initialized ticks, and simulated gas usage without a signer or a transaction. Its ABI marks quote methods non-view because they internally simulate `pool.swap`; the SDK always uses `eth_call`, never `send`. Exact-output paths are encoded in reverse by `Path`.

Quote results convert into the matching SwapRouter02 builders (`QuoteExactInputResult` → `ExactInputParamsBuilder`, and the single/exact-output analogues). Each result carries the original request path/amount (and single-hop price limit) plus the quoted outputs, so `from(quote)` seeds path/amount/`sqrtPriceLimitX96` and the quoted bound (`amountOutMinimum` / `amountInMaximum`). Call `apply_amount_out_slippage(BPS)` or `apply_amount_in_slippage(BPS)` to widen/tighten that bound, then set `recipient` before `build()`. Client `quote_exact_*` methods take `impl Into<...>`, so `client.quote_exact_input(&quote_params)` works without consuming the quote params.

Router parameter builders provide direct amount-bound setters. `then_default()` deliberately leaves swaps unprotected (`amountOutMinimum = 0`, `amountInMaximum = U256::MAX`); derive bounds from a fresh quote and an application-defined slippage policy.

NPM calltypes follow the same builder pattern (`CreatePositionParams`, `IncreaseLiquidityParams`, `DecreaseLiquidityParams`, `CollectParams`, `ClosePositionParams`). Seed from a `Pool` or `Position` where applicable, set required amounts/ticks/recipient, then `then_default()` for permissive mins (`0`), collect-all maxes (`u128::MAX`), and a ~30-day deadline.

`strategies` is deliberately an interface layer rather than a full strategy runner framework.
`Strategy::run` takes `&mut self`, a client, and the pool `Address` the strategy should trade,
spawns a Tokio task, and returns `JoinHandle<Result<(), StrategyError>>` so callers can await
failures or `abort()` the handle. Aborting the task does not close any live NFT.
`PriceSource::price` produces a Tokio `watch::Receiver<f64>` holding the latest USD price.
`ConstantWindowStrategy` is parameterized by separate token0/token1 price sources and tracks a
runtime `position` (`open_price`, NFT id, ticks) owned by the single run task.
`ConstantWindowStrategy::price` reads the latest values from token0/token1 USD `watch`
receivers and returns Uniswap-style mid (`token1` per `token0`) as `price0_usd / price1_usd`
(for example Binance for WETH and Stable for USDT). That external mid is authoritative for both
tick centering and rebalance decisions.

`ConstantWindowStrategy::run` lifecycle:

1. Hydrate `Pool::from_address` and subscribe both price sources
2. `pre_run` — close every owner NPM position matching the exact pool key `(token0, token1, fee)`
3. `validate` — portfolio fractions in `(0, 1]`; each rebalance threshold strictly inside its
   window length; positive finite prices; `balance * fraction > 0` for both tokens and existing
   NPM allowances ≥ those sized amounts (no auto-approve)
4. Loop — `None` → `set_position` (mint around external mid using `balance * portfolio_fraction`
   for each token at mint time); `Some` → `check_position` (hold while mid is within inclusive
   rebalance thresholds of `open_price`, else `close_position` and clear bookkeeping so the next
   iteration mints again); after each iteration wait on either price `watch` update

`BinancePriceSource` maps `WETH` to `ETH` and `WBTC` to `BTC`, subscribes to the lowercase
Spot stream `baseusdt@bookTicker`, waits for the first midpoint, then keeps updating a `watch`
channel until the socket, payload parsing, or consumer terminates. Connect and first-tick waits
are bounded. `StablePriceSource` returns a `watch` seeded at `1.0` for supported USD stables.

## Design rules

- Keep object fields **minimal and private**; prefer getters and derived methods (`address()`, `num_ticks()`, `max_liquidity_per_tick()`).
- Do not store values that are pure functions of other fields (e.g. pool address, `maxLiquidityPerTick`).
- RPC methods take a `Provider` (or use the client’s provider) and return `Result<T, UniswapV3Error>` — not Alloy `contract::Result`.
- Do not `impl` inherent methods on foreign types (`Token`); use extension traits in this crate.
- ABI bindings are generated only in `objects/abi_definitions.rs` (private); JSON sources stay under `artifacts/`. Re-export them with crate-local aliases exclusively from `objects/mod.rs` (`PoolContract`, `FactoryContract`, `SwapRouterContract`, `NpmContract`, `Erc20Contract`, plus public param structs). No other module may import `abi_definitions` directly.

## Errors

`UniswapV3Error` in `errors.rs`: build failures, RPC failures, invalid arguments, invalid pool, and converted `uniswap-sdk-core::Error`. `StrategyError` lives under `strategies/errors.rs` and covers already-running starts, invalid/closed prices, invalid configuration, missing signer/NPM, insufficient balance/allowance, wrapped `PriceSourceError`, and wrapped `UniswapV3Error`. `PriceSourceError` lives under `strategies/price_source/errors.rs` and covers missing token symbols, unsupported tokens, and subscription failures.

## Local testing

1. `./scripts/anvil.sh` — fork Ethereum mainnet
2. `./scripts/fund.sh` — fund the Anvil test account
3. Run the constant-window strategy or focused examples (each loads `.env` with
   `LOCAL_RPC_URL`, `TEST_PRIVATE_KEY`):
   - `cargo run -p uniswap-v3-rs-bin`
   - `cargo run -p uniswap-v3-rs-bin --example list_positions`
   - `cargo run -p uniswap-v3-rs-bin --example create_position`
   - `cargo run -p uniswap-v3-rs-bin --example increase_liquidity -- <token_id>`
   - `cargo run -p uniswap-v3-rs-bin --example close_position -- <token_id>`
   - `cargo run -p uniswap-v3-rs-bin --example swap_with_quote`
   - `cargo run -p uniswap-v3-rs-bin --example swap_without_quote`
   - `cargo run -p uniswap-v3-rs-bin --example exact_output`

## CI

- `.github/workflows/ci.yml` — fmt, clippy, test, docs
- `.github/workflows/security.yml` — `cargo audit`
- `.github/dependabot.yml` — weekly Cargo / Actions updates

## Docs in repo

- `README.md` — one-line product summary + badges
- `UNISWAP_V3_API_TECHNICAL_REFERENCE.md` — on-chain V3 API reference (not crate docs)
- `ARCHITECTURE.md` — this file
