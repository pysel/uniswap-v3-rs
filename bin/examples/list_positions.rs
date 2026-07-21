use std::{env, error::Error};

use alloy::signers::local::PrivateKeySigner;

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

    let owner = client.wallet().unwrap().default_signer().address();
    let positions = client.get_positions(owner).await?;

    println!("owner: {owner}");
    println!("positions: {}", positions.len());
    for position in &positions {
        let liquidity = position.liquidity(client.provider()).await?;
        let owed = position.tokens_owed(client.provider()).await?;
        let collectable = position.collectable_amounts(client.provider()).await?;
        println!(
            "  token_id={} ticks=[{}, {}] fee={} liquidity={liquidity}",
            position.token_id(),
            position.tick_lower(),
            position.tick_upper(),
            position.fee()
        );
        println!(
            "    tokens_owed=({}, {}) collectable=({}, {})",
            owed.amount0, owed.amount1, collectable.amount0, collectable.amount1
        );
    }

    Ok(())
}
