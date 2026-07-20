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

sol! {
    #[sol(rpc)]
    interface Erc20Metadata {
        function decimals() external view returns (uint8);
        function symbol() external view returns (string);
        function name() external view returns (string);
    }
}
