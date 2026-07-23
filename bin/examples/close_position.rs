use std::{env, error::Error, time::SystemTime};

use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::U256;

use uniswap_v3_rs::calltypes::ClosePositionParams;
use uniswap_v3_rs::client::UniswapV3Client;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv()?;

    let token_id = env::args()
        .nth(1)
        .ok_or("usage: cargo run -p uniswap-v3-rs-bin --example close_position -- <token_id>")?
        .parse::<U256>()?;

    let rpc_url = env::var("LOCAL_RPC_URL")?;
    let signer: PrivateKeySigner = env::var("TEST_PRIVATE_KEY")?.parse()?;

    let client = UniswapV3Client::builder()
        .rpc_url(rpc_url)
        .signer(signer)
        .build()
        .await?;

    let owner = client.wallet().unwrap().default_signer().address();
    let position = client.get_position(token_id).await?;
    let liquidity = position.liquidity(client.provider()).await?;

    println!(
        "closing token_id={} ticks=[{}, {}] fee={} liquidity={liquidity}",
        position.token_id(),
        position.tick_lower(),
        position.tick_upper(),
        position.fee()
    );

    let deadline = U256::from(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            + 600,
    );

    let response = client
        .close_position(
            &position,
            ClosePositionParams::new(owner, U256::ZERO, U256::ZERO, deadline),
        )
        .await?;
    println!("close tx: {}", response.tx_hash);

    let amounts = response.amounts.await?;
    println!(
        "collected amount0={} amount1={}",
        amounts.amount0, amounts.amount1
    );

    Ok(())
}
