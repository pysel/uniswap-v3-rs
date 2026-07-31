use std::{env, error::Error};

use alloy::signers::local::PrivateKeySigner;
use uniswap_sdk_core::prelude::BaseCurrency;

use uniswap_v3_rs::{
    calltypes::BPS,
    client::UniswapV3Client,
    objects::{TokenExt, USDC, WETH},
    strategies::{BinancePriceSource, ConstantWindowStrategy, StablePriceSource, Strategy},
};

use tracing::info;

const FEE: u32 = 3000;
const WINDOW_BPS: BPS = BPS::new(100);
const REBALANCE_BPS: BPS = BPS::new(50);

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let rpc_url = env::var("RPC_URL")?;
    let signer: PrivateKeySigner = env::var("PRIVATE_KEY")?.parse()?;

    let client = UniswapV3Client::builder()
        .rpc_url(rpc_url)
        .signer(signer)
        .gas_multiplier(1.05)
        .build()
        .await?;

    let owner = client
        .signer_address()
        .expect("signer required for constant-window strategy");
    let chain_id = client.get_chain_id().await?;
    let npm = client
        .position_manager()
        .expect("no NonfungiblePositionManager for this chain");

    info!("Starting constant-window strategy on chain {}", chain_id);

    let usdc = USDC::on_chain(chain_id).expect("USDC not deployed on chain");
    let weth = WETH::on_chain(chain_id).expect("WETH9 not deployed on chain");
    let pool = client.get_pool(weth.clone(), usdc.clone(), FEE).await?;

    // Mainnet USDC/WETH sorts as token0=USDC, token1=WETH.
    assert_eq!(pool.token0().address(), weth.address());
    assert_eq!(pool.token1().address(), usdc.address());

    println!("owner: {}", owner);
    println!("pool:  {}", pool.address());
    println!("npm:   {}", npm.address());

    usdc.approve_unlimited(&client, npm.address())
        .await?;
    weth.approve_unlimited(&client, npm.address())
        .await?;
    println!("approved USDC + WETH for NPM");

    let mut strategy = ConstantWindowStrategy::builder()
        .length_below_mid(WINDOW_BPS)
        .length_above_mid(WINDOW_BPS)
        .rebalance_below_threshold(REBALANCE_BPS)
        .rebalance_above_threshold(REBALANCE_BPS)
        .max_token0_amount_as_portfolio_fraction(0.95)
        .max_token1_amount_as_portfolio_fraction(0.95)
        .price_source_token0(BinancePriceSource::new())
        .price_source_token1(StablePriceSource::new())
        .build()?;

    println!(
        "starting constant-window strategy (window={WINDOW_BPS:?} rebalance={REBALANCE_BPS:?})"
    );
    println!("press Ctrl+C to abort (does not close the live NFT)");

    let handle = strategy.run(client, pool.address())?;
    let abort = handle.abort_handle();

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            abort.abort();
            println!("aborted");
            Ok(())
        }
        result = handle => Ok(result??),
    }
}
