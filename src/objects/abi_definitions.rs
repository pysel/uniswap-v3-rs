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
