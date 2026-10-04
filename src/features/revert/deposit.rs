use alloy::{
    primitives::{Address, TxHash, U256},
    sol_types::SolValue,
};

use crate::{
    calltypes::TransactionFuture,
    client::UniswapV3Client,
    errors::UniswapV3Error,
    objects::{NpmContract, send_with_gas_multiplier_map_err},
    strategies::Position,
};

use super::{result::deposit_result, vault_error};

pub struct RevertDepositResponse {
    pub tx_hash: TxHash,
    pub token_id: TransactionFuture<U256>,
}

impl UniswapV3Client {
    /// Deposits an NPM position into the configured Revert Lend vault as collateral.
    ///
    /// Sends `safeTransferFrom(signer, vault, tokenId, abi.encode(owner))`; the vault's
    /// `onERC721Received` opens a debt-free loan owned by `owner` (defaults to the signer).
    /// One transaction, no NFT approval needed.
    pub async fn deposit_to_revert(
        &self,
        position: &Position,
        owner: Option<Address>,
    ) -> Result<RevertDepositResponse, UniswapV3Error> {
        self.require_position_manager()?;

        let vault = self.require_revert_lend()?.vault();
        let signer = self.require_signer()?;

        let data = owner.unwrap_or(signer).abi_encode();
        let npm_address = self
            .position_manager()
            .unwrap() // checked
            .address();

        let contract = NpmContract::new(npm_address, self.provider());
        let call = contract.safeTransferFrom_1(signer, vault, position.position_id, data.into());
        let pending =
            send_with_gas_multiplier_map_err(call, self.gas_multiplier(), vault_error).await?;

        Ok(RevertDepositResponse {
            tx_hash: *pending.tx_hash(),
            token_id: deposit_result(pending, vault),
        })
    }
}
