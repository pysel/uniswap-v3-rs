use std::{env, error::Error};

use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::U256;
use uniswap_sdk_core::prelude::BaseCurrency;

use uniswap_v3_rs::{
    calltypes::BPS,
    client::UniswapV3Client,
    objects::{TokenExt, USDC, WETH},
    strategies::{
        BinancePriceSource, ConstantWindowStrategy, StablePriceSource, Strategy,
    },
};

const FEE: u32 = 500;
const WINDOW_BPS: BPS = BPS::new(100);
const REBALANCE_BPS: BPS = BPS::new(50);

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

    let owner = client
        .signer_address()
        .expect("signer required for constant-window strategy");
    let chain_id = client.get_chain_id().await?;
    let npm = client
        .position_manager()
        .expect("no NonfungiblePositionManager for this chain");

    let usdc = USDC::on_chain(chain_id).expect("USDC not deployed on chain");
    let weth = WETH::on_chain(chain_id).expect("WETH9 not deployed on chain");
    let pool = client.get_pool(usdc.clone(), weth.clone(), FEE).await?;

    // Mainnet USDC/WETH sorts as token0=USDC, token1=WETH.
    assert_eq!(pool.token0().address(), usdc.address());
    assert_eq!(pool.token1().address(), weth.address());

    println!("owner: {}", owner);
    println!("pool:  {}", pool.address());
    println!("npm:   {}", npm.address());

    usdc.approve_unlimited(client.provider(), npm.address())
        .await?;
    weth.approve_unlimited(client.provider(), npm.address())
        .await?;
    println!("approved USDC + WETH for NPM");

    let max_token0_amount = usdc.from_amount(5000);
    let max_token1_amount = weth.from_amount(1);

    let mut strategy = ConstantWindowStrategy::builder()
        .length_below_mid(WINDOW_BPS)
        .length_above_mid(WINDOW_BPS)
        .rebalance_below_threshold(REBALANCE_BPS)
        .rebalance_above_threshold(REBALANCE_BPS)
        .max_token0_amount(max_token0_amount)
        .max_token1_amount(max_token1_amount)
        .price_source_token0(StablePriceSource::new())
        .price_source_token1(BinancePriceSource::new())
        .build()?;

    println!(
        "starting constant-window strategy (window={WINDOW_BPS:?} rebalance={REBALANCE_BPS:?})"
    );
    println!("press Ctrl+C to abort (does not close the live NFT)");

    strategy.run(client, pool.address())?;
    tokio::signal::ctrl_c().await?;
    strategy.abort();
    println!("aborted");

    Ok(())
}
