use std::{env, error::Error};

use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use alloy_primitives::U256;

use uniswap_v3_rs::calltypes::BPS;
use uniswap_v3_rs::client::UniswapV3Client;
use uniswap_v3_rs::strategies::Position;

/// Deposit → leverage up → leverage down (100%) → withdraw, against a Revert Lend vault.
///
/// Env: `RPC_URL`, `TEST_PRIVATE_KEY`, `REVERT_VAULT`, `REVERT_LEVERAGE_TRANSFORMER`.
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv()?;

    let mut args = env::args().skip(1);
    let usage = "usage: cargo run -p uniswap-v3-rs-bin --example revert_leverage -- <pool> <token_id> <borrow_amount>";
    let pool = args.next().ok_or(usage)?.parse::<Address>()?;
    let token_id = args.next().ok_or(usage)?.parse::<U256>()?;
    let borrow_amount = args.next().ok_or(usage)?.parse::<U256>()?;

    let rpc_url = env::var("RPC_URL")?;
    let signer: PrivateKeySigner = env::var("TEST_PRIVATE_KEY")?.parse()?;
    let vault: Address = env::var("REVERT_VAULT")?.parse()?;
    let transformer: Address = env::var("REVERT_LEVERAGE_TRANSFORMER")?.parse()?;

    let client = UniswapV3Client::builder()
        .rpc_url(rpc_url)
        .signer(signer)
        .revert_lend(vault, transformer)
        .build()
        .await?;

    let npm_position = client.get_position(token_id).await?;
    let position = Position {
        open_price: 0.0, // unused by the Revert calls
        position_id: token_id,
        pool,
        chain_id: client.chain_id(),
        lower_tick: npm_position.tick_lower(),
        upper_tick: npm_position.tick_upper(),
    };
    let slippage = BPS::new(50);

    let deposit = client.deposit_to_revert(&position, None).await?;
    println!("deposit tx: {}", deposit.tx_hash);
    println!("deposited token_id={}", deposit.token_id.await?);

    let up = client
        .leverage_up(&position, borrow_amount, slippage)
        .await?;
    println!("leverage up tx: {}", up.tx_hash);
    println!("leveraged up: {:?}", up.result.await?);
    println!("loan: {:?}", client.revert_loan_info(token_id).await?);

    let down = client
        .leverage_down(&position, BPS::new(10_000), slippage)
        .await?;

    println!("leverage down tx: {}", down.tx_hash);
    println!("leveraged down: {:?}", down.result.await?);
    println!("loan: {:?}", client.revert_loan_info(token_id).await?);

    let withdraw = client.withdraw_from_revert(&position, None).await?;
    println!("withdraw tx: {}", withdraw.tx_hash);
    println!("withdrawn token_id={}", withdraw.token_id.await?);

    Ok(())
}
