# Architecture

Opinionated Uniswap V3 SDK crate. Designed for agents and contributors to navigate quickly.

## Stack

- **Alloy** — HTTP provider, signer/wallet, contract bindings (`sol!`)
- **`uniswap-sdk-core`** — offline primitives (`Token`, amounts, addresses); no RPC
- **Tokio** — async runtime for the binary / RPC calls

## Features

| Feature | Default | Notes |
| --- | --- | --- |
| `swap` | no | Enables `UniswapV3Client::swap_exact_*` helpers. Not default for lib dependents. Enabled by `bin/` for local examples. |
| `positions` | no | Enables NPM position reads and lifecycle helpers (`create`, `increase`, `decrease`, `collect`, `close`). Enabled by `bin/` for local examples. |

## Layout

```text
Cargo.toml               # lib package + workspace (members: ., bin)
bin/                     # local examples binary; depends on lib with features=["swap", "positions"]
  Cargo.toml
  src/main.rs            # prints example commands
  examples/
    list_positions.rs    # list owner NPM positions
    create_position.rs   # mint a USDC/WETH position NFT
    close_position.rs    # close (decrease+collect+burn) by token_id
    swap.rs              # exact-input swap
src/
  lib.rs                 # public modules: calltypes, client, errors, objects
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
    path.rs              # V3 Path/path! construction and packed ABI encoding
    router/              # one file per SwapRouter02 parameter/response pair
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
    router/
      mod.rs             # exports SwapRouter and internal result helpers
      router.rs          # SwapRouter02 definition and exact-input/output methods
      result.rs          # receipt-backed swap amount futures
    token/
      mod.rs             # re-exports TokenExt + USDC/USDT/WBTC/... registries
      token.rs           # TokenExt: RPC metadata loading and ERC-20 approvals
      usdc.rs            # USDC::on_chain from Uniswap default-token-list
      ...                # usdt, wbtc, uni, usde, usdg, usdt0, link, dai, cbbtc, bnb
    abi_definitions.rs   # Alloy sol! bindings for V3Pool / V3Factory / SwapRouter02 / NPM / Erc20
artifacts/               # JSON ABIs consumed by sol! (pool, factory, SwapRouter02, NPM)
scripts/
  anvil.sh               # mainnet fork via Anvil
  fund.sh                # fund Anvil account with WETH/USDC/USDT/WBTC
.env                     # local secrets (gitignored)
```

## Object model

| Type | Owns | Notes |
| --- | --- | --- |
| `UniswapV3Client` | `rpc_url`, Alloy `DynProvider`, optional wallet, `Factory`, optional `SwapRouter`, optional `NonfungiblePositionManager` | Entry point. Builder resolves factory (required) and optional deployments from RPC chain id. |
| `Factory` | `chain_id`, factory `address` | Offline CREATE2 derivation; `pool()` loads a `Pool` via provider. |
| `Pool` | factory, sorted `token0`/`token1`, `fee`, `tick_spacing` | Address is **derived**, not stored. Mutable state (e.g. `sqrt_price_x96`) fetched via RPC; can select a spacing-aligned tick within a conservative signed bps distance from the live token1/token0 midprice. |
| `SwapRouter` | `chain_id`, router `address` | Resolves SwapRouter02 deployments and submits exact-input/output transactions. |
| `NonfungiblePositionManager` | `chain_id`, NPM `address` | Resolves official NPM deployments and submits direct position lifecycle transactions. |
| `Position` | NPM identity, `token_id`, token addresses, fee, immutable tick range | NFT-backed position identity. Liquidity, owed tokens, owner, and collectable amounts are always fetched from chain. |
| `Path` | initial token, ordered token/fee hops | Builds and encodes exact-input or reversed exact-output V3 paths. |
| `Token` | from `uniswap-sdk-core` | Foreign type; RPC hydrate via `TokenExt` (orphan-rule extension trait). |
| `USDC` / `USDT` / … | unit structs | Offline `on_chain(chain_id)` registries sourced from Uniswap default-token-list for mainnet/arbitrum/base/avalanche/optimism/polygon/tempo. |

### Construction paths

1. **Offline / known metadata** — `token!` / `Token::new`, `Factory::from_chain`, `Pool::new`, `SwapRouter::from_chain`, `NonfungiblePositionManager::from_chain`
2. **From chain** — `Pool::from_address`, `Token::from_address` (needs provider); client `get_pool(token_a, token_b, fee)` → factory CREATE2 → `Pool::from_address`
3. **Position NFTs** — client `get_position(token_id)` reads NPM once for immutable NFT metadata. `Position::state`, `Position::liquidity`, `Position::tokens_owed`, and `Position::collectable_amounts` refetch mutable state every call.

Pool address derivation: `CREATE2(factory, keccak256(abi.encode(token0, token1, fee)), init_code_hash)` with `token0 < token1`. Init-code hash is an internal constant (zkSync uses a different hash / CREATE2 scheme).

Position lifecycle: `create_position` mints a new NFT, `increase_position_liquidity` adds liquidity to the same immutable tick range, `decrease_position_liquidity` credits withdrawn amounts to NPM owed balances, `collect_position` transfers owed balances, and `close_position` atomically decreases all current liquidity, collects, and burns the empty NFT.

Write methods return typed responses as soon as the transaction is accepted by the provider. Each response exposes `tx_hash` immediately and a typed future (for example, `amount_out`, `position`, or `amounts`) that waits for the receipt and resolves the actual event-backed Solidity result.

Router parameter builders provide direct amount-bound setters. `then_default()` deliberately leaves swaps unprotected (`amountOutMinimum = 0`, `amountInMaximum = U256::MAX`) until quoted-slippage support is added.

## Design rules

- Keep object fields **minimal and private**; prefer getters and derived methods (`address()`, `num_ticks()`, `max_liquidity_per_tick()`).
- Do not store values that are pure functions of other fields (e.g. pool address, `maxLiquidityPerTick`).
- RPC methods take a `Provider` (or use the client’s provider) and return `Result<T, UniswapV3Error>` — not Alloy `contract::Result`.
- Do not `impl` inherent methods on foreign types (`Token`); use extension traits in this crate.
- ABI bindings are generated only in `objects/abi_definitions.rs` (private); JSON sources stay under `artifacts/`. Re-export them with crate-local aliases exclusively from `objects/mod.rs` (`PoolContract`, `FactoryContract`, `SwapRouterContract`, `NpmContract`, `Erc20Contract`, plus public param structs). No other module may import `abi_definitions` directly.

## Errors

`UniswapV3Error` in `errors.rs`: build failures, RPC failures, invalid arguments, invalid pool, and converted `uniswap-sdk-core::Error`.

## Local testing

1. `./scripts/anvil.sh` — fork Ethereum mainnet
2. `./scripts/fund.sh` — fund the Anvil test account
3. Run focused examples (each loads `.env` with `LOCAL_RPC_URL`, `TEST_PRIVATE_KEY`):
   - `cargo run -p uniswap-v3-rs-bin --example list_positions`
   - `cargo run -p uniswap-v3-rs-bin --example create_position`
   - `cargo run -p uniswap-v3-rs-bin --example close_position -- <token_id>`
   - `cargo run -p uniswap-v3-rs-bin --example swap`

## CI

- `.github/workflows/ci.yml` — fmt, clippy, test, docs
- `.github/workflows/security.yml` — `cargo audit`
- `.github/dependabot.yml` — weekly Cargo / Actions updates

## Docs in repo

- `README.md` — one-line product summary + badges
- `UNISWAP_V3_API_TECHNICAL_REFERENCE.md` — on-chain V3 API reference (not crate docs)
- `ARCHITECTURE.md` — this file
