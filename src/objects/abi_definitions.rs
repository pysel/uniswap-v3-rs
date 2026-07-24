use alloy::sol;

sol!(
    #[sol(rpc)]
    V3Pool,
    "artifacts/UniswapV3Pool.json"
);

sol!(
    #[sol(rpc)]
    V3Factory,
    "artifacts/UniswapV3Factory.json"
);

sol!(
    #[sol(rpc)]
    SwapRouter02,
    "artifacts/SwapRouter02.json"
);

#[cfg(feature = "swap")]
sol!(
    #[sol(rpc)]
    QuoterV2,
    "artifacts/QuoterV2.json"
);

#[cfg(feature = "positions")]
sol!(
    #[sol(rpc)]
    NonfungiblePositionManager,
    "artifacts/NonfungiblePositionManager.json"
);

sol! {
    #[sol(rpc)]
    interface Erc20 {
        function decimals() external view returns (uint8);
        function symbol() external view returns (string);
        function name() external view returns (string);
        function approve(address spender, uint256 amount) external returns (bool);
    }
}
