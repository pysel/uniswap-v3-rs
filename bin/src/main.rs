use std::{collections::HashSet, env, error::Error, time::SystemTime};

use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::U256;
use uniswap_sdk_core::prelude::BaseCurrency;

use uniswap_v3_rs::calltypes::ClosePositionParams;
use uniswap_v3_rs::client::UniswapV3Client;
use uniswap_v3_rs::objects::{MintParams, TokenExt, USDC, WETH};

const FEE: u32 = 500;
/// Width of the minted range in tick-spacing units on each side of the current tick.
const RANGE_WIDTH_SPACINGS: i32 = 10;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv()?;

    let rpc_url = env::var("LOCAL_RPC_URL")?;
    let signer: PrivateKeySigner = env::var("TEST_PRIVATE_KEY")?.parse()?;

    let client = UniswapV3Client::builder()
        .rpc_url(rpc_url)
        .signer(signer)
        .build()
        .await?;

    let owner = client.wallet().unwrap().default_signer().address();
    let chain_id = client.get_chain_id().await?;
    let npm = client
        .position_manager()
        .expect("no NonfungiblePositionManager for this chain");

    let usdc = USDC::on_chain(chain_id).expect("USDC not deployed on chain");
    let weth = WETH::on_chain(chain_id).expect("WETH9 not deployed on chain");

    println!("owner: {owner}");
    println!("npm:   {}", npm.address());

    // --- approve NPM -------------------------------------------------------
    usdc.approve_unlimited(client.provider(), npm.address())
        .await?;
    weth.approve_unlimited(client.provider(), npm.address())
        .await?;
    println!("approved USDC + WETH for NPM");

    // --- list existing positions -------------------------------------------
    let existing = client.get_positions(owner).await?;
    println!("existing positions: {}", existing.len());
    for position in &existing {
        let liquidity = position.liquidity(client.provider()).await?;
        println!(
            "  token_id={} ticks=[{}, {}] fee={} liquidity={liquidity}",
            position.token_id(),
            position.tick_lower(),
            position.tick_upper(),
            position.fee()
        );
    }
    let existing_ids: HashSet<_> = existing.iter().map(|position| position.token_id()).collect();

    // --- resolve pool + ticks around current price -------------------------
    let pool = client.get_pool(usdc.clone(), weth.clone(), FEE).await?;
    let tick = pool.tick(client.provider()).await?.as_i32();
    let spacing = pool.tick_spacing();
    let aligned = floor_div(tick, spacing) * spacing;
    let tick_lower = aligned - spacing * RANGE_WIDTH_SPACINGS;
    let tick_upper = aligned + spacing * RANGE_WIDTH_SPACINGS;

    println!(
        "pool {} fee={FEE} tick={tick} spacing={spacing} mint range=[{tick_lower}, {tick_upper}]",
        pool.address()
    );

    // token0/token1 are address-sorted; match desired amounts to that order.
    let (amount0_desired, amount1_desired) = if pool.token0().address() == usdc.address() {
        (U256::from(1_000_000u64), U256::from(10u64).pow(U256::from(15u64))) // 1 USDC, 0.001 WETH
    } else {
        (U256::from(10u64).pow(U256::from(15u64)), U256::from(1_000_000u64))
    };

    let deadline = U256::from(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            + 600,
    );

    // --- mint --------------------------------------------------------------
    let mint_params = MintParams::new(
        &pool,
        tick_lower,
        tick_upper,
        amount0_desired,
        amount1_desired,
        U256::ZERO,
        U256::ZERO,
        owner,
        deadline,
    )?;
    let mint_tx = client.create_position(mint_params, None).await?;
    println!("mint tx: {mint_tx:?}");

    let created = client
        .get_positions(owner)
        .await?
        .into_iter()
        .find(|position| !existing_ids.contains(&position.token_id()))
        .expect("minted position NFT not found for owner");

    let state = created.state(client.provider()).await?;
    let owed = created.tokens_owed(client.provider()).await?;
    let collectable = created.collectable_amounts(client.provider()).await?;

    println!("created token_id={}", created.token_id());
    println!(
        "  immutable: token0={} token1={} fee={} ticks=[{}, {}]",
        created.token0(),
        created.token1(),
        created.fee(),
        created.tick_lower(),
        created.tick_upper()
    );
    println!(
        "  live: liquidity={} tokens_owed=({}, {}) collectable=({}, {})",
        state.liquidity, owed.amount0, owed.amount1, collectable.amount0, collectable.amount1
    );

    // --- close (decrease all + collect + burn) -----------------------------
    let close_tx = client
        .close_position(
            &created,
            ClosePositionParams::new(owner, U256::ZERO, U256::ZERO, deadline),
        )
        .await?;
    println!("close tx: {close_tx:?}");

    let remaining = client.get_position_count(owner).await?;
    println!("positions remaining for owner: {remaining}");

    Ok(())
}

/// Division toward −∞ (matches Uniswap tick compression for negatives).
fn floor_div(value: i32, divisor: i32) -> i32 {
    let quotient = value / divisor;
    let remainder = value % divisor;
    if remainder != 0 && value < 0 {
        quotient - 1
    } else {
        quotient
    }
}
