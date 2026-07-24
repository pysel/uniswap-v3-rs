use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::U256;
use std::{env, error::Error};

use uniswap_v3_rs::calltypes::{BPS, ExactInputParamsBuilder, QuoteExactInputParams};
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

    // First: get a quote. This is optional, can use swap params directly, but quote gives better control over slippage. 
    let quote_params = QuoteExactInputParams::builder(&path)
        .amount_in(U256::from(1_000_000_000))
        .build()?;

    let quote = client.quote_exact_input(&quote_params).await?;
    println!("quote: {:#?}", quote.amount_out);

    let slippage = BPS::from_percent(0)?;
    let params = ExactInputParamsBuilder::from(quote)
        .recipient(owner)
        .apply_amount_out_slippage(slippage)?
        .build()?;

    let response = client.swap_exact_input(params, None).await?;
    println!("swap tx: {}", response.tx_hash);

    println!("amount out: {}", response.amount_out.await?);

    Ok(())
}
