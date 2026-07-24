use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::U256;
use std::{env, error::Error};

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
    let owner = client.wallet().unwrap().default_signer().address();

    let params = ExactInputParams::builder(&path)
        .recipient(owner)
        .amount_in(U256::from(1_000_000_000))
        .then_default() // Equivalent to .amount_out_minimum(U256::ZERO) and .sqrt_price_limit_x96(U160::ZERO)
        .build()?;

    let response = client.swap_exact_input(params, None).await?;
    println!("swap tx: {}", response.tx_hash);

    println!("amount out: {}", response.amount_out.await?);

    Ok(())
}
