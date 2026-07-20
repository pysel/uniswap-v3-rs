# Uniswap V3 API Technical Reference

**Iteration:** 4 (final)  
**Status:** Complete — critique loop stopped (no remaining material V3 API omissions)  
**Sources:** Official Solidity (`v3-core`, `v3-periphery`, `swap-router-contracts`/`IV3SwapRouter`, `v3-staker`), `@uniswap/v3-sdk` / `sdk-core`, Uniswap deployment tables, Trading API (V3 route surface)  
**Scope:** On-chain V3 core + periphery APIs, path encoding, callbacks, quoters, staker, Permit/SelfPermit, multicall, CREATE2 addressing, subgraph/SDK notes, integrator failure modes  
**Doc path:** `/Users/ruslan/github/uni-sdk-rs/UNISWAP_V3_API_TECHNICAL_REFERENCE.md`

---

## Table of Contents

1. [API Surface Taxonomy](#1-api-surface-taxonomy)
2. [Math & Encoding Primitives](#2-math--encoding-primitives)
3. [Factory API](#3-factory-api)
4. [Pool Core API](#4-pool-core-api)
5. [Callbacks](#5-callbacks)
6. [Pool Events](#6-pool-events)
7. [NonfungiblePositionManager](#7-nonfungiblepositionmanager)
8. [Swap Routers](#8-swap-routers)
9. [Quoters](#9-quoters)
10. [Lenses & Periphery Helpers](#10-lenses--periphery-helpers)
11. [SwapRouter02 Extensions](#11-swaprouter02-extensions)
12. [V3 Migrator](#12-v3-migrator)
13. [V3 Staker](#13-v3-staker)
14. [Universal Router V3 Commands](#14-universal-router-v3-commands)
15. [CREATE2 Pool Addressing](#15-create2-pool-addressing)
16. [Tick Bitmap Math](#16-tick-bitmap-math)
17. [Oracle TWAP (`observe`)](#17-oracle-twap-observe)
18. [Deployed Addresses](#18-deployed-addresses)
19. [TypeScript / SDK Surfaces](#19-typescript--sdk-surfaces)
20. [Subgraph GraphQL (V3)](#20-subgraph-graphql-v3)
21. [Error Catalog & Failure Modes](#21-error-catalog--failure-modes)
22. [Non-standard Token Warnings](#22-non-standard-token-warnings)
23. [Worked Integration Sequences](#23-worked-integration-sequences)
24. [Function Selector Cheat-Sheet](#24-function-selector-cheat-sheet)
25. [Mixed-Route Quoters](#25-mixed-route-quoters)
26. [Critique Log](#26-critique-log)

---

## 1. API Surface Taxonomy

| Layer | What it is | Primary consumers | Mutating? |
| --- | --- | --- | --- |
| **Core (`v3-core`)** | Per-pool AMM contracts + factory | Routers, custom integrators | Yes (via callbacks) |
| **Periphery (`v3-periphery`)** | NPM, SwapRouter, Quoter, TickLens, Migrator | Wallets, apps, bots | Yes |
| **SwapRouter02 (`swap-router-contracts`)** | Optimized V3 router (no deadline in params; `amountIn=0` balance tricks) | Modern frontends | Yes |
| **Staker (`v3-staker`)** | Incentive programs over NPM NFTs | Liquidity mining | Yes |
| **Off-chain** | Subgraph, `@uniswap/v3-sdk`, Trading API | Quoting, indexing, UX | REST mutates txs; GraphQL/SDK read |

### Recommended entry points by job

| Job | Prefer |
| --- | --- |
| Wallet / app swap | SwapRouter02 `exactInput(Single)` or Trading API → Universal Router |
| Off-chain quote | QuoterV2 via `eth_call` (never on-chain) |
| LP mint/manage | `NonfungiblePositionManager` |
| Direct pool / flash | `IUniswapV3Pool.swap` / `flash` + callback |
| Pool discovery | Factory `getPool` or CREATE2 `computeAddress` |
| Historical analytics | V3 subgraph |

### Architecture reminder

- **One deployed pool contract** per `(token0, token1, fee)` via Factory CREATE2 clones.
- Tokens always sorted `token0 < token1` by address.
- All mutating pool ops that move tokens use **callbacks** (`swap` / `mint` / `flash`).

---

## 2. Math & Encoding Primitives

### 2.1 `sqrtPriceX96`

\[
\texttt{sqrtPriceX96} = \sqrt{\frac{\texttt{token1}}{\texttt{token0}}} \times 2^{96}
\]

- Type: `uint160`, fixed-point **Q64.96**
- Token order: `token0 < token1` by address

### 2.2 Ticks (`TickMath`)

| Constant | Value |
| --- | --- |
| `MIN_TICK` | `-887272` |
| `MAX_TICK` | `887272` |
| `MIN_SQRT_RATIO` | `4295128739` |
| `MAX_SQRT_RATIO` | `1461446703485210103287273052203988822378723970342` |

- Tick \(i\) ↔ price \(1.0001^i\)
- Swap limits must stay **strictly inside** `(MIN_SQRT_RATIO, MAX_SQRT_RATIO)` → typically `MIN_SQRT_RATIO + 1` / `MAX_SQRT_RATIO - 1`
- Position ticks must be multiples of pool `tickSpacing`
- `slot0.tick` may differ from `getTickAtSqrtRatio(sqrtPriceX96)` when price sits exactly on a boundary

Helpers: `getSqrtRatioAtTick(tick)`, `getTickAtSqrtRatio(sqrtPriceX96)`

### 2.3 Fee units

Fees in **hundredths of a bip** (1 bip = 0.01%):

| Fee | Percent | Default tickSpacing (mainnet) |
| --- | --- | --- |
| `100` | 0.01% | 1 |
| `500` | 0.05% | 10 |
| `3000` | 0.30% | 60 |
| `10000` | 1.00% | 200 |

Constructor initially enabled 500/3000/10000; fee `100` enabled later by governance. Additional fees may be `enableFeeAmount`'d; never removed.

### 2.4 `amountSpecified` sign (pool `swap`)

| Mode | `amountSpecified` |
| --- | --- |
| Exact input | `> 0` |
| Exact output | `< 0` |

Returned `amount0` / `amount1` are **pool balance deltas**: negative = pool received / user paid; positive = pool sent / user received.

### 2.5 Multi-hop path encoding

Exact-input path (token order = swap order):

```
tokenIn (20) | fee (3) | tokenMid (20) | fee (3) | tokenOut (20) | ...
```

Exact-output path is the **reverse** of the intended swap path. Fees are big-endian `uint24`.

From `v3-periphery` `Path` library:

| Constant | Value | Meaning |
| --- | --- | --- |
| `ADDR_SIZE` | 20 | Token address bytes |
| `FEE_SIZE` | 3 | Fee bytes |
| `NEXT_OFFSET` | 23 | `ADDR_SIZE + FEE_SIZE` |
| `POP_OFFSET` | 43 | First pool key span (`NEXT_OFFSET + ADDR_SIZE`) |
| `MULTIPLE_POOLS_MIN_LENGTH` | 66 | Minimum length for ≥2 pools |

`numPools(path) = (path.length - 20) / 23`.

### 2.6 Position key (pool storage)

```
key = keccak256(abi.encodePacked(owner, tickLower, tickUpper))
```

Used by `IUniswapV3PoolState.positions(bytes32)`.

### 2.7 Fee growth

- Global: `feeGrowthGlobal{0,1}X128` — Q128.128 fees per unit liquidity (may overflow `uint256` by design)
- Inside/outside tick accounting via `feeGrowthOutside` + position `feeGrowthInside*Last`

---

## 3. Factory API

**Package:** `@uniswap/v3-core` — `IUniswapV3Factory`

| Function | Signature (abbrev.) | Notes |
| --- | --- | --- |
| `owner` | `() → address` | Governance |
| `feeAmountTickSpacing` | `(fee) → tickSpacing` | `0` if fee not enabled |
| `getPool` | `(tokenA, tokenB, fee) → pool` | Order-insensitive; `address(0)` if missing |
| `createPool` | `(tokenA, tokenB, fee) → pool` | Sorts tokens; reverts if exists / bad fee / equal tokens |
| `setOwner` | `(address)` | Owner only |
| `enableFeeAmount` | `(fee, tickSpacing)` | Owner only; irreversible |

**Events:** `OwnerChanged`, `PoolCreated(token0, token1, fee, tickSpacing, pool)`, `FeeAmountEnabled(fee, tickSpacing)`

---

## 4. Pool Core API

**Interface:** `IUniswapV3Pool` = Immutables ∪ State ∪ DerivedState ∪ Actions ∪ OwnerActions ∪ Events

### 4.1 Immutables (`IUniswapV3PoolImmutables`)

| Function | Returns |
| --- | --- |
| `factory()` | Factory address |
| `token0()` / `token1()` | Sorted pair |
| `fee()` | `uint24` |
| `tickSpacing()` | `int24` |
| `maxLiquidityPerTick()` | `uint128` — `type(uint128).max / numTicks` where `numTicks` counts usable ticks at this spacing between aligned `MIN_TICK`/`MAX_TICK` |

### 4.2 Mutable state (`IUniswapV3PoolState`)

#### `slot0()`

| Field | Type | Meaning |
| --- | --- | --- |
| `sqrtPriceX96` | `uint160` | Current sqrt price |
| `tick` | `int24` | Current tick |
| `observationIndex` | `uint16` | Last written oracle index |
| `observationCardinality` | `uint16` | Current oracle length |
| `observationCardinalityNext` | `uint16` | Next cardinality target |
| `feeProtocol` | `uint8` | Packed nibbles: token0 = low 4 bits, token1 = high 4 bits. Value `N` means protocol takes `1/N` of the pool swap fee (`0` = off). Example: `feeProtocol = 0x54` → token0 uses `4` (1/4), token1 uses `5` (1/5). |
| `unlocked` | `bool` | Reentrancy lock |

Also: `feeGrowthGlobal0X128`, `feeGrowthGlobal1X128`, `protocolFees() → (token0, token1)`, `liquidity()` (in-range only).

#### `ticks(int24)`

Returns: `liquidityGross`, `liquidityNet`, `feeGrowthOutside0/1X128`, `tickCumulativeOutside`, `secondsPerLiquidityOutsideX128`, `secondsOutside`, `initialized`.

**`liquidityNet` sign:** When a tick is used as a position’s **lower** bound, minting adds `+liquidity` to `liquidityNet`; as **upper** bound, minting adds `−liquidity`. On a swap crossing the tick **left → right** (price up), active liquidity changes by `+liquidityNet`; crossing **right → left** (price down) applies `−liquidityNet`.

#### Other

- `tickBitmap(int16 wordPosition) → uint256` — 256 packed initialized flags
- `positions(bytes32 key)` — see §2.6
- `observations(uint256 index)` — raw oracle ring buffer entry

### 4.3 Derived / oracle (`IUniswapV3PoolDerivedState`)

| Function | Notes |
| --- | --- |
| `observe(uint32[] secondsAgos)` | TWAP building blocks: `tickCumulatives`, `secondsPerLiquidityCumulativeX128s` |
| `snapshotCumulativesInside(tickLower, tickUpper)` | Range snapshot for fee/seconds accounting |

### 4.4 Permissionless actions (`IUniswapV3PoolActions`)

| Function | Callback | Notes |
| --- | --- | --- |
| `initialize(sqrtPriceX96)` | — | Once; emits `Initialize` |
| `mint(recipient, tickLower, tickUpper, amount, data)` | `uniswapV3MintCallback` | Pays owed token0/1 in callback |
| `collect(recipient, tickLower, tickUpper, amount0Requested, amount1Requested)` | — | Does **not** poke fees; use `burn(0)` first to update owed |
| `burn(tickLower, tickUpper, amount)` | — | Accounts tokens owed; `amount=0` recalculates fees |
| `swap(recipient, zeroForOne, amountSpecified, sqrtPriceLimitX96, data)` | `uniswapV3SwapCallback` | See §2.4 signs |
| `flash(recipient, amount0, amount1, data)` | `uniswapV3FlashCallback` | Pay back + fee in callback; `0,0` + donate possible |
| `increaseObservationCardinalityNext(uint16)` | — | No-op if already ≥ |

### 4.5 Owner actions (`IUniswapV3PoolOwnerActions`)

Factory owner only:

- `setFeeProtocol(uint8 feeProtocol0, uint8 feeProtocol1)`
- `collectProtocol(address recipient, uint128 amount0Requested, uint128 amount1Requested)`

---

## 5. Callbacks

### 5.1 `IUniswapV3SwapCallback`

```solidity
function uniswapV3SwapCallback(int256 amount0Delta, int256 amount1Delta, bytes calldata data) external;
```

- Positive delta ⇒ callback **MUST** pay that token amount **to the pool**
- Verify `msg.sender` is a canonical pool: `factory.getPool(token0, token1, fee) == msg.sender`
- Pool `unlocked` flag prevents reentrant pool calls during callbacks

### 5.2 `IUniswapV3MintCallback`

```solidity
function uniswapV3MintCallback(uint256 amount0Owed, uint256 amount1Owed, bytes calldata data) external;
```

Pay `amount0Owed` / `amount1Owed` of token0/1 to the pool.

### 5.3 `IUniswapV3FlashCallback`

```solidity
function uniswapV3FlashCallback(uint256 fee0, uint256 fee1, bytes calldata data) external;
```

Repay flashed amounts + fees to the pool before return.

---

## 6. Pool Events

| Event | Key fields |
| --- | --- |
| `Initialize` | `sqrtPriceX96`, `tick` |
| `Mint` | `sender`, `owner`, `tickLower`, `tickUpper`, `amount`, `amount0`, `amount1` |
| `Burn` | `owner`, `tickLower`, `tickUpper`, `amount`, `amount0`, `amount1` |
| `Collect` | `owner`, `recipient`, `tickLower`, `tickUpper`, `amount0`, `amount1` |
| `Swap` | `sender`, `recipient`, `amount0`, `amount1`, `sqrtPriceX96`, `liquidity`, `tick` |
| `Flash` | `sender`, `recipient`, `amount0/1`, `paid0/1` |
| `IncreaseObservationCardinalityNext` | old/new |
| `SetFeeProtocol` | old/new protocol fee nibbles |
| `CollectProtocol` | `sender`, `recipient`, `amount0`, `amount1` |

---

## 7. NonfungiblePositionManager

**Package:** `@uniswap/v3-periphery`  
ERC-721 wrapper for positions. Also inherits: `IPoolInitializer`, `IPeripheryPayments`, `IPeripheryImmutableState`, `IERC721Metadata/Enumerable`, `IERC721Permit`, typically `IMulticall` / `ISelfPermit` on the implementation.

### 7.1 `positions(uint256 tokenId)`

Returns: `nonce`, `operator`, `token0`, `token1`, `fee`, `tickLower`, `tickUpper`, `liquidity`, `feeGrowthInside0/1LastX128`, `tokensOwed0/1`.

### 7.2 Mutators

| Method | Struct highlights | Returns |
| --- | --- | --- |
| `mint(MintParams)` | tokens, fee, ticks, desired/min amounts, recipient, **deadline** | `tokenId`, `liquidity`, `amount0`, `amount1` |
| `increaseLiquidity(IncreaseLiquidityParams)` | `tokenId`, desired/min, deadline | `liquidity`, `amount0`, `amount1` |
| `decreaseLiquidity(DecreaseLiquidityParams)` | `tokenId`, `liquidity`, mins, deadline | `amount0`, `amount1` (accounted owed) |
| `collect(CollectParams)` | `tokenId`, recipient, `amount0/1Max` | collected amounts |
| `burn(tokenId)` | Requires 0 liquidity and fees collected | — |

**Events:** `IncreaseLiquidity`, `DecreaseLiquidity`, `Collect`

### 7.3 Pool initializer helper

`createAndInitializePoolIfNecessary(token0, token1, fee, sqrtPriceX96)` — factory create + `initialize` if needed (on `IPoolInitializer`).

### 7.4 Payments / WETH

Via `IPeripheryPayments`: `unwrapWETH9`, `refundETH`, `sweepToken` (+ fee variants on `IPeripheryPaymentsWithFee`). Native ETH accepted as `msg.value` and wrapped where needed.

### 7.5 Permit

`IERC721Permit.permit` for gasless NFT approval; `ISelfPermit` for ERC-20 permits in the same multicall.

---

## 8. Swap Routers

### 8.1 SwapRouter (v1) — `ISwapRouter`

Structs include **`deadline`**. Selectors differ from SR02.

| Method | Params include |
| --- | --- |
| `exactInputSingle` | tokenIn/Out, fee, recipient, **deadline**, amountIn, amountOutMinimum, sqrtPriceLimitX96 |
| `exactInput` | path, recipient, **deadline**, amountIn, amountOutMinimum |
| `exactOutputSingle` / `exactOutput` | analogous; exact-out path **reversed** |

### 8.2 SwapRouter02 — `IV3SwapRouter`

**No `deadline` in swap structs** (use `multicall` + deadline check via `IMulticallExtended` / checkSettled patterns on the deployment).

Critical SR02 behaviors:

- `amountIn == 0` ⇒ router uses **its own balance** of `tokenIn` (enables pull-then-swap / approveAndCall flows)
- Implements `IUniswapV3SwapCallback`
- Often combined with `IApproveAndCall`, `IMulticallExtended`

| Method | Notes |
| --- | --- |
| `exactInputSingle(ExactInputSingleParams)` | No deadline field |
| `exactInput(ExactInputParams)` | Multi-hop path |
| `exactOutputSingle` / `exactOutput` | Exact-out; leftover input may remain in router — sweep |

**Footgun:** Using V1 selectors/ABI against SwapRouter02 (or vice versa) reverts. SR02 `exactInputSingle` selector commonly cited as `0x04e45aaf`.

### 8.3 Universal Router

Not V3-core, but production wallets often encode V3 swaps as UR commands (`V3_SWAP_EXACT_IN` / `V3_SWAP_EXACT_OUT`) with Permit2. Prefer UR when mixing protocols; prefer SwapRouter02 for pure V3 app integrations.

---

## 9. Quoters

### 9.1 Quoter (v1) — `IQuoter`

Legacy flat ABI (no structs). Prefer QuoterV2 for tick/gas metadata.

| Method | Args | Returns |
| --- | --- | --- |
| `quoteExactInput` | `path`, `amountIn` | `amountOut` |
| `quoteExactInputSingle` | `tokenIn`, `tokenOut`, `fee`, `amountIn`, `sqrtPriceLimitX96` | `amountOut` |
| `quoteExactOutput` | `path` (reversed), `amountOut` | `amountIn` |
| `quoteExactOutputSingle` | `tokenIn`, `tokenOut`, `fee`, `amountOut`, `sqrtPriceLimitX96` | `amountIn` |

Same `eth_call`-only constraint as V2. V1 encodes a simpler revert payload (amount-focused); do not mix V1/V2 addresses or ABIs.

### 9.2 QuoterV2 — `IQuoterV2`

**Not `view`.** Simulates `pool.swap`, reverts with encoded result in callback path; designed for **`eth_call` only** — never send as a real tx expecting success.

#### `quoteExactInputSingle(QuoteExactInputSingleParams)`

Params: `tokenIn`, `tokenOut`, `amountIn`, `fee`, `sqrtPriceLimitX96` (`0` ⇒ default min/max±1).

Returns: `amountOut`, `sqrtPriceX96After`, `initializedTicksCrossed`, `gasEstimate`.

#### `quoteExactInput(path, amountIn)`

Returns lists of per-pool `sqrtPriceX96After` and `initializedTicksCrossed`, plus total `gasEstimate`.

#### Exact-out analogues

`quoteExactOutputSingle` / `quoteExactOutput` — path for multi-hop exact-out must be **reversed**.

#### Revert parsing (integrator note)

Internal `parseRevertReason` expects 96 bytes → `(uint256 amount, uint160 sqrtPriceX96After, int24 tickAfter)` when catching the simulated swap. Modern ethers/viem usually surface the **successful return data** of the outer `quote*` call when the node simulates correctly; if the RPC only returns revert blobs, decode carefully. Always treat a hard revert without quote payload as **insufficient liquidity / bad path**.

---

## 10. Lenses & Periphery Helpers

| Contract | Role |
| --- | --- |
| `TickLens` | `getPopulatedTicksInWord(pool, tickBitmapIndex)` — batch initialized tick data for one bitmap word |
| `Multicall` | `multicall(bytes[] data)` — batch calls; **do not trust `msg.value` per subcall** |
| `SelfPermit` | `selfPermit` / `selfPermitIfNecessary` / `selfPermitAllowed` / `selfPermitAllowedIfNecessary` |
| `PeripheryImmutableState` | `factory()`, `WETH9()` |

---

## 11. SwapRouter02 Extensions

### 11.1 `IMulticallExtended`

Beyond plain `multicall(bytes[])`:

| Method | Purpose |
| --- | --- |
| `multicall(uint256 deadline, bytes[] data)` | Reverts if `block.timestamp > deadline` |
| `multicall(bytes32 previousBlockhash, bytes[] data)` | Reverts unless `blockhash(block.number - 1) == previousBlockhash` (backrun / inclusion guard) |

This is how SR02 recovers **deadline** semantics without putting `deadline` inside each swap struct.

### 11.2 `IApproveAndCall`

Lens + helpers so a multicall can approve the NPM and mint/increase in one tx:

| Method | Notes |
| --- | --- |
| `getApprovalType(token, amount)` | Off-chain lens → `NOT_REQUIRED`, `MAX`, `MAX_MINUS_ONE`, `ZERO_THEN_MAX`, `ZERO_THEN_MAX_MINUS_ONE` |
| `approveMax` / `approveMaxMinusOne` / `approveZeroThenMax*` | Payable approve helpers |
| `callPositionManager(bytes)` | Arbitrary NPM calldata |
| `mint(MintParams)` / `increaseLiquidity(IncreaseLiquidityParams)` | Thin wrappers (mins only; amounts pulled from router balance) |

Combined with `amountIn == 0` swap behavior, SR02 supports “transfer tokens to router → swap/mint” patterns.

---

## 12. V3 Migrator

`IV3Migrator` — migrate Uniswap **V2** liquidity into a V3 NPM position (create/initialize pool if needed, mint NFT). Not used for V3→V4 (app-level remove+mint).

Typical flow: approve V2 LP → `migrate` with percentage, deadline, tick range, slippage mins → receive NPM `tokenId`.

---

## 13. V3 Staker

**Package:** `v3-staker` — `IUniswapV3Staker`

```solidity
struct IncentiveKey {
  IERC20Minimal rewardToken;
  IUniswapV3Pool pool;
  uint256 startTime;
  uint256 endTime;
  address refundee;
}
```

`incentiveId = keccak256(abi.encode(key))`.

| API | Purpose |
| --- | --- |
| `createIncentive(key, reward)` | Fund program |
| `endIncentive(key)` | Refund leftover after end + no stakes |
| `onERC721Received` | Deposit NFT |
| `stakeToken` / `unstakeToken` | Attach/detach incentive |
| `claimReward(rewardToken, to, amountRequested)` | Pull accrued rewards |
| `withdrawToken(tokenId, to, data)` | Exit deposit |
| `transferDeposit` | Change deposit owner |
| Views | `incentives`, `deposits`, `stakes`, `rewards` |
| `getRewardInfo(key, tokenId)` | Returns `(reward, secondsInsideX128)` accrued so far (**not `view`** — may update; call off-chain / static) |
| `claimReward(rewardToken, to, amountRequested)` | `amountRequested == 0` ⇒ claim entire owed balance |

Also: `factory`, `nonfungiblePositionManager`, `maxIncentiveDuration`, `maxIncentiveStartLeadTime`.

Rewards accrue proportional to liquidity × seconds the position’s range is **in-range** during the incentive window (UQ32.128 seconds accounting).

---

## 14. Universal Router V3 Commands

From `Commands.sol` / `Dispatcher` (V2 UR family; verify against the deployed UR version):

| Command | Value | Inputs (conceptual) |
| --- | --- | --- |
| `V3_SWAP_EXACT_IN` | `0x00` | `recipient`, `amountIn`, `amountOutMin`, `path`, `payerIsUser`, optional `minHopPriceX36[]` (newer builds) |
| `V3_SWAP_EXACT_OUT` | `0x01` | `recipient`, `amountOut`, `amountInMax`, `path` (reversed), `payerIsUser`, … |
| `V3_POSITION_MANAGER_PERMIT` | `0x11` | NPM permit |
| `V3_POSITION_MANAGER_CALL` | `0x12` | Arbitrary NPM calldata |

Related: `PERMIT2_PERMIT` (`0x0a`), `WRAP_ETH` (`0x0b`), `UNWRAP_WETH` (`0x0c`), `SWEEP` (`0x04`).

`FLAG_ALLOW_REVERT = 0x80` may be OR'd into the command byte; leftovers must be swept or funds stick in the router.

---

## 15. CREATE2 Pool Addressing

From `v3-periphery` `PoolAddress`:

```
POOL_INIT_CODE_HASH = 0xe34f199b19b2b4f47f68442619d555527d244f78a3297ea89325f843f87b8b54
```

```
pool = address(uint160(uint256(keccak256(abi.encodePacked(
  hex"ff",
  factory,
  keccak256(abi.encode(token0, token1, fee)),
  POOL_INIT_CODE_HASH
)))))
```

Require `token0 < token1`. Forks that changed pool bytecode need a different init code hash (e.g. some zkSync deployments).

---

## 16. Tick Bitmap Math

Initialized ticks are packed 256-per-word keyed by **compressed** tick index (`tick / tickSpacing`, rounding toward −∞ for negatives).

For compressed tick `t`:

```
wordPos = int16(t >> 8)   // t / 256
bitPos  = uint8(t % 256)
```

Pool getter: `tickBitmap(wordPos) → uint256`. Flip uses `1 << bitPos`.

`TickLens.getPopulatedTicksInWord(pool, wordPos)` returns the populated ticks in that word for off-chain indexing.

`nextInitializedTickWithinOneWord` searches at most one word (≤256 compressed ticks) toward lower (`lte=true`) or higher (`lte=false`).

---

## 17. Oracle TWAP (`observe`)

```solidity
function observe(uint32[] secondsAgos)
  returns (int56[] tickCumulatives, uint160[] secondsPerLiquidityCumulativeX128s);
```

Example: 1-hour TWAP tick:

1. Ensure `observationCardinality` is large enough (`increaseObservationCardinalityNext` ahead of time — cardinality grows on subsequent writes).
2. Call `observe([3600, 0])`.
3. `tickTWAP = (tickCumulatives[1] - tickCumulatives[0]) / 3600` (integer division toward zero; handle negatives carefully off-chain).
4. Convert tick → price via `TickMath.getSqrtRatioAtTick` / SDK helpers.

`snapshotCumulativesInside(tickLower, tickUpper)` supports fee/seconds accounting for a range without iterating ticks.

---

## 18. Deployed Addresses

### 18.1 Ethereum mainnet (canonical)

| Contract | Address |
| --- | --- |
| UniswapV3Factory | `0x1F98431c8aD98523631AE4a59f267346ea31F984` |
| Multicall2 (legacy docs) | `0x5BA1e12693Dc8F9c48aAD8770482f4739bEeD696` |
| Multicall3 (sdk-core default map) | `0x1F98415757620B543A52E61c46B32eB19261F984` |
| TickLens | `0xbfd8137f7d1516D3ea5cA83523914859ec47F573` |
| Quoter | `0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6` |
| QuoterV2 | `0x61fFE014bA17989E743c5F6cB21bF9697530B21e` |
| SwapRouter | `0xE592427A0AEce92De3Edee1F18E0157C05861564` |
| SwapRouter02 | `0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45` |
| NonfungiblePositionManager | `0xC36442b4a4522E871399CD717aBDD847Ab11FE88` |
| V3Migrator | `0xA5644E29708357803b5A882D272c41cC0dF92B34` |
| V3Staker | `0x1f98407aaB862cdDeF78Ed252D6f577aF10dD012` |
| Permit2 | `0x000000000022D473030F116dDEE9F6B43aC78BA3` |
| WETH9 | `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` |

Many L2s (Optimism, Arbitrum, Polygon, …) reuse the **same** Factory / QuoterV2 / NPM / TickLens addresses as mainnet for early deployments — but **Base, BSC, Celo, Avalanche, Blast, Unichain, zkSync, Worldchain, etc. differ**. Prefer `uniswap-sdk-core` `ChainAddresses` or official Uniswap docs per chain.

### 18.2 Selected non-default Factory addresses

| Chain | V3 Factory |
| --- | --- |
| Base | `0x33128a8fC17869897dcE68Ed026d694621f6FDfD` |
| BSC | `0xdB1d10011AD0Ff90774D0C6Bb92e5C5c8b4461F7` |
| Celo | `0xAfE208a311B21f13EF87E33A90049fC17A7acDEc` |
| Avalanche | `0x740b1c1de25031C31FF4fC9A62f554A55cdC1baD` |
| Blast | `0x792edAdE80af5fC680d96a2eD80A44247D2Cf6Fd` |
| Unichain | `0x1f98400000000000000000000000000000000003` |
| zkSync | `0x8FdA5a7a8dCA67BBcDd10F02Fa0649A937215422` |

Always confirm QuoterV2 / SR02 / NPM on that chain before production use.

---

## 19. TypeScript / SDK Surfaces

| Package | Role |
| --- | --- |
| `@uniswap/sdk-core` | `Token`, `CurrencyAmount`, `Price`, `Percent` |
| `@uniswap/v3-sdk` | `Pool`, `Position`, `Route`, `Trade`, tick math, path encoding, NPM/router calldata |
| `@uniswap/smart-order-router` | Off-chain routing |
| `@uniswap/universal-router-sdk` | UR command encoding |
| `@uniswap/permit2-sdk` | Permit2 typed data |

### `@uniswap/v3-sdk` highlights

- `Pool.getAddress` — CREATE2
- `getOutputAmount` / `getInputAmount` — off-chain sim (V3 signs)
- `Position.fromAmounts` / `nearestUsableTick` / `encodeRouteToPath`
- Calldata helpers: NPM, SwapRouter, SelfPermit, SwapQuoter

Rust: `uniswap-sdk-core` + `uniswap-v3-sdk` (community).

### NPM ERC-721 permit

```
PERMIT_TYPEHASH = keccak256("Permit(address spender,uint256 tokenId,uint256 nonce,uint256 deadline)")
             = 0x49ecf333e5b8c95c40fdafc95c1ad136e8914a8fb55e9dc8bb01eaa83a2df9ad
```

EIP-712 domain: `name` / `version` from NPM constructor (`Uniswap V3 Positions NFT-V1` / `1` on canonical deployments), `chainId`, `verifyingContract = NPM`.

Digest: `keccak256("\x19\x01" ‖ DOMAIN_SEPARATOR ‖ keccak256(abi.encode(TYPEHASH, spender, tokenId, nonce, deadline)))`.

---

## 20. Subgraph GraphQL (V3)

| Entity | Notable fields |
| --- | --- |
| `Factory` | poolCount, volume/TVL aggregates |
| `Pool` | id (address), token0/1, feeTier, liquidity, sqrtPrice, tick, volumeUSD, feesUSD, TVL, day/hour data |
| `Token` | symbol, decimals, volumeUSD, poolCount, TVL |
| `Swap` | id (`txHash#index`), amounts, amountUSD, sqrtPriceX96, tick |
| `Mint` / `Burn` / `Collect` | LP events |
| `Tick` | liquidityGross/Net |

Endpoints are chain-specific (The Graph hosted/decentralized + Alchemy/Goldsky mirrors). Pin subgraph version in production; USD fields are indexer-derived.

---

## 21. Error Catalog & Failure Modes

### 21.1 Core pool require strings (`UniswapV3Pool.sol`)

| Code | Meaning |
| --- | --- |
| `LOK` | Pool locked (reentrancy / not unlocked) |
| `TLU` | `tickLower >= tickUpper` |
| `TLM` | `tickLower < MIN_TICK` |
| `TUM` | `tickUpper > MAX_TICK` |
| `AI` | Already initialized (`initialize` when `sqrtPriceX96 != 0`) |
| `AS` | `amountSpecified == 0` |
| `SPL` | Price limit / direction invariant failed |
| `IIA` | Insufficient input asset paid in swap callback |
| `M0` / `M1` | Mint callback underpaid token0/1 |
| `L` | Flash with zero in-range liquidity |
| `F0` / `F1` | Flash fee not paid for token0/1 |

Factory owner checks use plain `require(msg.sender == owner)` without a short code.

### 21.2 Periphery / integrator failures

| Failure | Cause |
| --- | --- |
| Deadline | NPM / V1 router / `multicall(deadline, …)` expired |
| Slippage | `amountOutMinimum` / `amountInMaximum` / mint mins |
| Wrong router ABI | V1 vs SR02 struct mismatch |
| Quoter revert | No liquidity / bad path; or sending Quoter as a real tx |
| Uninitialized pool | `sqrtPriceX96 == 0` |
| Tick misalignment | Tick not multiple of `tickSpacing` |
| Callback spoof | Paying wrong pool |
| NFT burn revert | Liquidity or `tokensOwed*` still non-zero |
| Staker constraints | Timing / still staked |
| Permit expired | NPM / ERC-20 permit deadline |

### 21.3 Gas heuristics

| Factor | Effect |
| --- | --- |
| Initialized tick crosses | Dominant variable swap gas |
| Cold token/pool storage | First-touch premium |
| Low oracle cardinality | Cheap until `increaseObservationCardinalityNext` expands |
| Multicall batching | Amortizes base tx cost; watch stack depth |

---

## 22. Non-standard Token Warnings

Uniswap V3 **assumes** standard ERC-20 semantics:

- `transfer` / `transferFrom` return `bool` or revert; balance deltas match amounts
- **No fee-on-transfer** (deflationary) — callback balance checks (`IIA`/`M0`/`M1`) will fail or silently mis-account
- **No rebase** tokens in-pool — inventory desync
- Tokens that withhold from `address(0)` / blacklists can brick swaps mid-route
- Some tokens require `approve(0)` before reset — use SR02 `approveZeroThenMax*`

Native ETH is **not** a pool currency; use WETH (+ router wrap/unwrap).

---

## 23. Worked Integration Sequences

### 23.1 Exact-in via SwapRouter02

1. Approve tokenIn → SR02 (or `selfPermit` / Permit2)
2. QuoterV2 `quoteExactInputSingle` via `eth_call`; apply slippage → `amountOutMinimum`
3. Optionally wrap as `multicall(deadline, [exactInputSingle_data, unwrapWETH9_data, refundETH_data])`
4. `exactInputSingle` with `sqrtPriceLimitX96 = 0` unless you need a limit

### 23.2 Exact-in via Universal Router

1. Permit2 permit (if needed)
2. `execute(commands, inputs, deadline)` with `V3_SWAP_EXACT_IN` path bytes
3. `SWEEP` / `UNWRAP_WETH` as needed

### 23.3 Pool-level swap (custom contract)

1. Resolve pool (Factory / CREATE2)
2. `pool.swap(recipient, zeroForOne, amountSpecified, limit, data)`
3. Callback: verify `factory.getPool(token0,token1,fee) == msg.sender`; pay positive deltas

### 23.4 Mint LP NFT

1. `createAndInitializePoolIfNecessary` if needed
2. Approve NPM; `mint(MintParams)` with spaced ticks
3. Exit: `decreaseLiquidity` → `collect` → `burn`

### 23.5 Fee poke + collect

1. `burn(..., 0)` on pool **or** NPM decrease/collect flow to update `tokensOwed*`
2. `collect` with `amount*Max = type(uint128).max` to pull all

### 23.6 Flash

1. `pool.flash(recipient, amount0, amount1, data)`
2. Callback: use funds; repay `amount + fee`

### 23.7 TWAP read

1. Ensure cardinality; `observe([window, 0])`; divide cumulatives; convert tick → price

---

## 24. Function Selector Cheat-Sheet

| Target | Signature (abbrev.) | Selector |
| --- | --- | --- |
| Pool `slot0` | `slot0()` | `0x3850c7bd` |
| Pool `swap` | `swap(address,bool,int256,uint160,bytes)` | `0x128acb08` |
| NPM `positions` | `positions(uint256)` | `0x99fbab88` |
| NPM `mint` | `mint((…MintParams))` | `0x88316456` |
| SwapRouter (v1) `exactInputSingle` | struct **with** deadline | `0x414bf389` |
| SwapRouter (v1) `exactInput` | struct **with** deadline | `0xc04b8d59` |
| SwapRouter02 `exactInputSingle` | struct **without** deadline | `0x04e45aaf` |
| SwapRouter02 `exactInput` | struct **without** deadline | `0xb858183f` |
| QuoterV2 `quoteExactInputSingle` | struct params | `0xc6a5026a` |
| QuoterV2 `quoteExactInput` | `(bytes,uint256)` | `0xcdca1753` |

---

## 25. Mixed-Route Quoters

`MixedRouteQuoterV1` / `V2` (periphery deployments on some chains) quote paths that mix **V2 + V3** pools for smart-order routers. Not required for pure V3 integrations; addresses live in `uniswap-sdk-core` `ChainAddresses.mixed_route_quoter_v*`. Same off-chain-only calling convention as QuoterV2.

---

## 26. Critique Log

### Iteration 1

Delivered core/periphery skeleton.

### Iteration 2

Filled errors, SR02 extensions, bitmap, TWAP, permit, UR commands, FOT warnings, multi-chain factories, gas notes.

### Iteration 3

**Filled:**

1. QuoterV1 full method table
2. `liquidityNet` cross direction semantics
3. Protocol fee nibble example (`0x54`)
4. Staker `getRewardInfo` / `claimReward(0)` notes + accrual intuition
5. Selector cheat-sheet (pool / NPM / V1 vs SR02 / QuoterV2)
6. Mixed-route quoter pointer

**Intentionally deferred (non-material for a V3 API reference / better as generated artifacts):**

1. Exhaustive every-chain every-contract address dump → use `uniswap-sdk-core` / official docs as SoT
2. Subgraph URL matrix (changes often; pin per deployment)
3. Newest UR `minHopPriceX36` deep dive (UR-version-specific; see UR release notes)
4. Permit2 full EIP-712 (belongs in Permit2 reference; V3 only consumes it via UR)
5. Bit-level Quoter revert hex walkthrough (redundant with §9 when using typed `eth_call` returns)

### Iteration 4

**Filled:** `Path` library size constants / `numPools`; `maxLiquidityPerTick` formula.

**Verdict:** Material V3 integrator API surface is covered (core, periphery, SR02, quoters, staker, UR V3 cmds, math, errors, selectors). Remaining items are generated address tables, subgraph URL churn, or adjacent protocols (full Permit2 / UR version diffs) — out of scope for this V3 reference.

**STOP.**

---

*End of iteration 4 — complete.*
