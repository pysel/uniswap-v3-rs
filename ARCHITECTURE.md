# Architecture

Opinionated Uniswap V3 SDK crate. Designed for agents and contributors to navigate quickly.

## Stack

- **Alloy** — HTTP provider, signer/wallet, contract bindings (`sol!`)
- **`uniswap-sdk-core`** — offline primitives (`Token`, amounts, addresses); no RPC
- **Tokio** — async runtime for the binary / RPC calls

## Layout

```text
src/
  lib.rs                 # public modules: client, errors, objects
  client.rs              # UniswapV3Client (+ builder)
  errors.rs              # UniswapV3Error
  main.rs                # local manual test binary (loads .env)
  objects/
    mod.rs               # re-exports Factory, Pool, TokenExt
    factory.rs           # Factory: CREATE2 pool address, pool() helper
    pool.rs              # Pool: immutables + RPC state getters
    token.rs             # TokenExt: Token::from_address via ERC-20 metadata
    abi_definitions.rs   # Alloy sol! bindings for V3Pool / V3Factory / Erc20Metadata
artifacts/               # JSON ABIs consumed by sol!
scripts/
  anvil.sh               # mainnet fork via Anvil
  fund.sh                # fund Anvil account with WETH/USDC/USDT/WBTC
.env                     # local secrets (gitignored)
```

## Object model

| Type | Owns | Notes |
| --- | --- | --- |
| `UniswapV3Client` | `rpc_url`, Alloy `DynProvider`, optional wallet, `Factory` | Entry point. Builder resolves factory from RPC chain id. |
| `Factory` | `chain_id`, factory `address` | Offline CREATE2 derivation; `pool()` loads a `Pool` via provider. |
| `Pool` | factory, sorted `token0`/`token1`, `fee`, `tick_spacing` | Address is **derived**, not stored. Mutable state (e.g. `sqrt_price_x96`) fetched via RPC. |
| `Token` | from `uniswap-sdk-core` | Foreign type; RPC hydrate via `TokenExt` (orphan-rule extension trait). |

### Construction paths

1. **Offline / known metadata** — `token!` / `Token::new`, `Factory::from_chain`, `Pool::new`
2. **From chain** — `Pool::from_address`, `Token::from_address` (needs provider); client `get_pool(token_a, token_b, fee)` → factory CREATE2 → `Pool::from_address`

Pool address derivation: `CREATE2(factory, keccak256(abi.encode(token0, token1, fee)), init_code_hash)` with `token0 < token1`. Init-code hash is an internal constant (zkSync uses a different hash / CREATE2 scheme).

## Design rules

- Keep object fields **minimal and private**; prefer getters and derived methods (`address()`, `num_ticks()`, `max_liquidity_per_tick()`).
- Do not store values that are pure functions of other fields (e.g. pool address, `maxLiquidityPerTick`).
- RPC methods take a `Provider` (or use the client’s provider) and return `Result<T, UniswapV3Error>` — not Alloy `contract::Result`.
- Do not `impl` inherent methods on foreign types (`Token`); use extension traits in this crate.
- ABI bindings live only in `objects/abi_definitions.rs`; JSON sources stay under `artifacts/`.

## Errors

`UniswapV3Error` in `errors.rs`: build failures, RPC failures, invalid pool, and converted `uniswap-sdk-core::Error`.

## Local testing

1. `./scripts/anvil.sh` — fork Ethereum mainnet
2. `./scripts/fund.sh` — fund the Anvil test account
3. `cargo run` — `main.rs` loads `.env` (`LOCAL_RPC_URL`, `TEST_PRIVATE_KEY`) and exercises client → pool

## CI

- `.github/workflows/ci.yml` — fmt, clippy, test, docs
- `.github/workflows/security.yml` — `cargo audit`
- `.github/dependabot.yml` — weekly Cargo / Actions updates

## Docs in repo

- `README.md` — one-line product summary + badges
- `UNISWAP_V3_API_TECHNICAL_REFERENCE.md` — on-chain V3 API reference (not crate docs)
- `ARCHITECTURE.md` — this file
