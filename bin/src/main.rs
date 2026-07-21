use alloy_primitives::U256;
use std::{env, error::Error};

use alloy::signers::local::PrivateKeySigner;

use uniswap_v3_rs::calltypes::ExactInputParams;
use uniswap_v3_rs::client::UniswapV3Client;
use uniswap_v3_rs::objects::{TokenExt, USDC, WETH};
use uniswap_v3_rs::path;

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
    let usdc = USDC::on_chain(chain_id).expect("USDC not deployed on chain");
    let weth = WETH::on_chain(chain_id).expect("WETH9 not deployed on chain");

    usdc.approve_unlimited(client.provider(), client.swap_router().unwrap().address())
        .await?;
    weth.approve_unlimited(client.provider(), client.swap_router().unwrap().address())
        .await?;

    let path = path!(usdc, 500, weth)?;

    let params = ExactInputParams::new(
        &path,
        client.wallet().unwrap().default_signer().address(),
        U256::from(1000000),
        U256::from(0),
    )?;
    let result = client.swap_exact_input(params, None).await?;

    println!("result: {:?}", result);

    Ok(())
}
