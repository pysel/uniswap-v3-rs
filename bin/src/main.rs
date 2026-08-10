use std::{env, error::Error};

use alloy::signers::local::PrivateKeySigner;
use tokio_util::sync::CancellationToken;
use uniswap_sdk_core::prelude::BaseCurrency;

use uniswap_v3_rs::{
    calltypes::BPS,
    client::UniswapV3Client,
    hedger::{BaseUrl, Hedger, HyperliquidHedger},
    objects::{TokenExt, USDC, WETH},
    strategies::{BinancePriceSource, ConstantWindowStrategy, StablePriceSource, Strategy},
};

use tracing::info;

const FEE: u32 = 3000;
const WINDOW_BPS: BPS = BPS::new(100);
const REBALANCE_BPS: BPS = BPS::new(50);
const MAX_HEDGE_LEVERAGE: f64 = 3.0;
const REHEDGE_INTERVAL_SECONDS: u64 = 60;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let rpc_url = env::var("RPC_URL")?;
    let private_key = env::var("PRIVATE_KEY")?;
    let signer: PrivateKeySigner = private_key.parse()?;

    let hedger_private_key = env::var("HEDGER_PRIVATE_KEY")?;
    let hedge_signer: PrivateKeySigner = hedger_private_key.parse()?;

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

    usdc.approve_unlimited(&client, npm.address()).await?;
    weth.approve_unlimited(&client, npm.address()).await?;
    println!("approved USDC + WETH for NPM");

    let cancellation_token = CancellationToken::new();

    let strategy = ConstantWindowStrategy::builder()
        .length_below_mid(WINDOW_BPS)
        .length_above_mid(WINDOW_BPS)
        .rebalance_below_threshold(REBALANCE_BPS)
        .rebalance_above_threshold(REBALANCE_BPS)
        .max_token0_amount_as_portfolio_fraction(1.0)
        .max_token1_amount_as_portfolio_fraction(1.0)
        .price_source_token0(BinancePriceSource::new())
        .price_source_token1(StablePriceSource::new())
        .cancellation_token(cancellation_token.clone())
        .build()?;

    println!(
        "starting constant-window strategy (window={WINDOW_BPS:?} rebalance={REBALANCE_BPS:?})"
    );

    let (mut strategy_handle, mut position_rx) = strategy.run(client.clone(), pool.address())?;

    let hedger_address = hedge_signer.address();
    let hedger = HyperliquidHedger::builder()
        .client(client.clone())
        .private_key(hedge_signer)
        .position(position_rx.clone())
        .base_url(BaseUrl::Testnet)
        .max_leverage(MAX_HEDGE_LEVERAGE)
        .rehedge_interval_seconds(REHEDGE_INTERVAL_SECONDS)
        .cancellation_token(cancellation_token.clone())
        .build()
        .await?;

    let (hedger_handle, mut hedge_rx) = hedger.hedge()?;
    info!(
        %hedger_address,
        max_leverage = MAX_HEDGE_LEVERAGE,
        rehedge_interval_seconds = REHEDGE_INTERVAL_SECONDS,
        "started Hyperliquid testnet hedger"
    );
    println!("hedger: {hedger_address}");
    println!("press Ctrl+C to abort");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                cancellation_token.cancel();
                strategy_handle.await??;
                hedger_handle.await?;
                println!("aborted");
                return Ok(());
            }
            changed = hedge_rx.changed() => {
                changed?;
                let status = hedge_rx.borrow_and_update().clone();
                info!(?status, "hedge status");
            }
            changed = position_rx.changed() => {
                changed?;
                let position = *position_rx.borrow();
                info!(?position, "position changed");
            }
            result = &mut strategy_handle => {
                result??;
                cancellation_token.cancel();

                hedger_handle.await?;

                return Ok(());
            }
        }
    }
}
