use std::{collections::HashSet, env, error::Error};

use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::U256;
use uniswap_sdk_core::prelude::BaseCurrency;

use uniswap_v3_rs::client::UniswapV3Client;
use uniswap_v3_rs::objects::{CreatePositionParams, TokenExt, USDC, WETH};

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

    usdc.approve_unlimited(client.provider(), npm.address())
        .await?;
    weth.approve_unlimited(client.provider(), npm.address())
        .await?;
    println!("approved USDC + WETH for NPM");

    let existing_ids: HashSet<_> = client
        .get_positions(owner)
        .await?
        .into_iter()
        .map(|position| position.token_id())
        .collect();

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
    // 1 USDC + 0.001 WETH
    let (amount0_desired, amount1_desired) = if pool.token0().address() == usdc.address() {
        (usdc.from_amount(1), weth.from_amount(1) / U256::from(1000))
    } else {
        (weth.from_amount(1) / U256::from(1000), usdc.from_amount(1))
    };

    let create_position_params = CreatePositionParams::builder(&pool)
        .tick_lower(tick_lower)
        .tick_upper(tick_upper)
        .amount0_desired(amount0_desired)
        .amount1_desired(amount1_desired)
        .recipient(owner)
        .then_default()
        .build()?;

    let mint_tx = client.create_position(create_position_params, None).await?;
    println!("mint tx: {mint_tx:?}");

    let created = client
        .get_positions(owner)
        .await?
        .into_iter()
        .find(|position| !existing_ids.contains(&position.token_id()))
        .expect("minted position NFT not found for owner");

    let state = created.state(client.provider()).await?;
    println!("created token_id={}", created.token_id());
    println!(
        "  ticks=[{}, {}] fee={} liquidity={}",
        created.tick_lower(),
        created.tick_upper(),
        created.fee(),
        state.liquidity
    );
    println!(
        "close with: cargo run -p uniswap-v3-rs-bin --example close_position -- {}",
        created.token_id()
    );

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
