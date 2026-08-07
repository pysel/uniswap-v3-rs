use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::U256;
use std::{env, error::Error};

use uniswap_v3_rs::calltypes::{
    BPS, ExactOutputParamsBuilder, ExactOutputSingleParamsBuilder, QuoteExactOutputParams,
    QuoteExactOutputSingleParams,
};
use uniswap_v3_rs::client::UniswapV3Client;
use uniswap_v3_rs::objects::{TokenExt, USDC, USDT, WETH};
use uniswap_v3_rs::path;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv()?;

    let rpc_url = env::var("RPC_URL")?;
    let signer: PrivateKeySigner = env::var("TEST_PRIVATE_KEY")?.parse()?;

    let client = UniswapV3Client::builder()
        .rpc_url(rpc_url)
        .signer(signer)
        .build()
        .await?;

    let chain_id = client.get_chain_id().await?;
    let owner = client.wallet().unwrap().default_signer().address();
    let router = client
        .swap_router()
        .expect("no SwapRouter02 for this chain");

    let usdc = USDC::on_chain(chain_id).expect("USDC not deployed on chain");
    let weth = WETH::on_chain(chain_id).expect("WETH9 not deployed on chain");
    let usdt = USDT::on_chain(chain_id).expect("USDT not deployed on chain");

    usdc.approve_unlimited(&client, router.address()).await?;

    let slippage = BPS::from_percent(1)?;

    // Single-hop exact-output: pay USDC, receive exactly 0.001 WETH.
    let single_path = path!(usdc.clone(), 500, weth.clone())?;
    let single_quote = client
        .quote_exact_output_single(
            QuoteExactOutputSingleParams::builder(&single_path)
                .amount_out(weth.from_amount(1.0) / U256::from(1_000))
                .then_default()
                .build()?,
        )
        .await?;
    println!("single-hop quote amount_in={}", single_quote.amount_in);

    let single_params = ExactOutputSingleParamsBuilder::from(single_quote)
        .recipient(owner)
        .apply_amount_in_slippage(slippage)?
        .build()?;
    let single_response = client.swap_exact_output_single(single_params, None).await?;
    println!("single-hop swap tx: {}", single_response.tx_hash);
    println!("single-hop amount_in: {}", single_response.amount_in.await?);

    // Multi-hop exact-output: pay USDC via WETH, receive exactly 1 USDT.
    let multi_path = path!(usdc, 500, weth, 500, usdt.clone())?;
    let multi_quote = client
        .quote_exact_output(
            QuoteExactOutputParams::builder(&multi_path)
                .amount_out(usdt.from_amount(1.0))
                .build()?,
        )
        .await?;
    println!("multi-hop quote amount_in={}", multi_quote.amount_in);

    let multi_params = ExactOutputParamsBuilder::from(multi_quote)
        .recipient(owner)
        .apply_amount_in_slippage(slippage)?
        .build()?;
    let multi_response = client.swap_exact_output(multi_params, None).await?;
    println!("multi-hop swap tx: {}", multi_response.tx_hash);
    println!("multi-hop amount_in: {}", multi_response.amount_in.await?);

    Ok(())
}
