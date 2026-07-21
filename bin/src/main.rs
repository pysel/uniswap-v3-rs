fn main() {
    println!(
        "\
uniswap-v3-rs local examples (requires Anvil + .env):

  cargo run -p uniswap-v3-rs-bin --example list_positions
  cargo run -p uniswap-v3-rs-bin --example create_position
  cargo run -p uniswap-v3-rs-bin --example close_position -- <token_id>
  cargo run -p uniswap-v3-rs-bin --example swap
"
    );
}
