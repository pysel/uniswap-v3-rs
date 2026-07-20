use std::{env, error::Error};
use tokio;

use alloy::signers::local::PrivateKeySigner;
use uniswap_sdk_core::{prelude::*, token};

use uniswap_v3_rs::client::UniswapV3Client;

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

    let chain_id = client.get_chain_id().await?;
    let usdc = token!(
        chain_id,
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        6,
        "USDC",
        "USD Coin"
    );
    let weth = WETH9::on_chain(chain_id).expect("WETH9 not deployed on chain");

    let pool = client.get_pool(usdc, weth, 500).await?;

    println!("pool: {:?}", pool);

    Ok(())
}
