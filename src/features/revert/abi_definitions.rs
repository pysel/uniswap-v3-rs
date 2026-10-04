#![allow(clippy::too_many_arguments)] // generated V3Vault constructor/deploy bindings

use alloy::sol;

sol!(
    #[sol(rpc, extra_derives(Debug))]
    RevertVault,
    "artifacts/revert/V3Vault.json"
);

sol!(
    #[sol(rpc)]
    LeverageTransformer,
    "artifacts/revert/LeverageTransformer.json"
);

sol! {
    /// Payload Revert's `Swapper` decodes for Universal Router swaps.
    struct UniversalRouterData {
        bytes commands;
        bytes[] inputs;
        uint256 deadline;
    }
}
