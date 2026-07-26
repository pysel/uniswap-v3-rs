use std::{env, error::Error};

use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::U256;
use uniswap_sdk_core::prelude::BaseCurrency;

use uniswap_v3_rs::calltypes::IncreaseLiquidityParams;
use uniswap_v3_rs::client::UniswapV3Client;
use uniswap_v3_rs::objects::{TokenExt, USDC, WETH};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv()?;

    let token_id = env::args()
        .nth(1)
        .ok_or(
            "usage: cargo run -p uniswap-v3-rs-bin --example increase_liquidity -- <token_id>",
        )?
        .parse::<U256>()?;

    let rpc_url = env::var("LOCAL_RPC_URL")?;
    let signer: PrivateKeySigner = env::var("TEST_PRIVATE_KEY")?.parse()?;

    let client = UniswapV3Client::builder()
        .rpc_url(rpc_url)
        .signer(signer)
        .build()
        .await?;

    let npm = client
        .position_manager()
        .expect("no NonfungiblePositionManager for this chain");
    let chain_id = client.get_chain_id().await?;
    let usdc = USDC::on_chain(chain_id).expect("USDC not deployed on chain");
    let weth = WETH::on_chain(chain_id).expect("WETH9 not deployed on chain");

    usdc.approve_unlimited(client.provider(), npm.address())
        .await?;
    weth.approve_unlimited(client.provider(), npm.address())
        .await?;

    let position = client.get_position(token_id).await?;
    let liquidity_before = position.liquidity(client.provider()).await?;

    println!(
        "increasing token_id={} ticks=[{}, {}] fee={} liquidity={liquidity_before}",
        position.token_id(),
        position.tick_lower(),
        position.tick_upper(),
        position.fee()
    );

    // Match NPM token0/token1 order (address-sorted), not symbol order.
    let (amount0_desired, amount1_desired) = if position.token0() == usdc.address() {
        (usdc.from_amount(1), weth.from_amount(1) / U256::from(1_000))
    } else {
        (weth.from_amount(1) / U256::from(1_000), usdc.from_amount(1))
    };

    let params = IncreaseLiquidityParams::builder(&position)
        .amount0_desired(amount0_desired)
        .amount1_desired(amount1_desired)
        .then_default()
        .build()?;

    let response = client.increase_position_liquidity(params, None).await?;
    println!("increase tx: {}", response.tx_hash);

    let result = response.result.await?;
    println!(
        "added liquidity={} amount0={} amount1={}",
        result.liquidity, result.amount0, result.amount1
    );

    let liquidity_after = position.liquidity(client.provider()).await?;
    println!("liquidity after={liquidity_after}");

    Ok(())
}
