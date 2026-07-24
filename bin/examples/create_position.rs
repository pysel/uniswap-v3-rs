use std::{env, error::Error};

use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::U256;

use uniswap_v3_rs::calltypes::BPS;
use uniswap_v3_rs::client::UniswapV3Client;
use uniswap_v3_rs::objects::{CreatePositionParams, TokenExt, USDC, WETH};

const FEE: u32 = 500;
/// Total width of the minted range around midprice (`50` BPS each side).
const RANGE_WIDTH_BPS: BPS = BPS::new(100);

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

    let pool = client.get_pool(usdc.clone(), weth.clone(), FEE).await?;

    let (tick_lower, tick_upper) = pool
        .get_both_ticks_away_from_mid(client.provider(), BPS::new(RANGE_WIDTH_BPS.get() / 2))
        .await?;

    // token0/token1 are address-sorted; match desired amounts to that order.
    // 1 USDC + 0.001 WETH
    let (amount0_desired, amount1_desired) = (usdc.from_amount(1), weth.from_amount(1) / U256::from(1000));

    let create_position_params = CreatePositionParams::builder(&pool)
        .tick_lower(tick_lower)
        .tick_upper(tick_upper)
        .amount0_desired(amount0_desired)
        .amount1_desired(amount1_desired)
        .recipient(owner)
        .then_default()
        .build()?;

    let response = client.create_position(create_position_params, None).await?;
    println!("mint tx: {}", response.tx_hash);

    let result = response.position.await?;
    println!("created token_id={}", result.token_id);
    println!(
        "  ticks=[{tick_lower}, {tick_upper}] fee={FEE} liquidity={} amount0={} amount1={}",
        result.liquidity, result.amount0, result.amount1
    );
    println!(
        "close with: cargo run -p uniswap-v3-rs-bin --example close_position -- {}",
        result.token_id
    );

    Ok(())
}
