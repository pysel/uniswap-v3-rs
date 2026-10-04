use alloy::primitives::{Address, Bytes, TxHash, U256};

use crate::{
    calltypes::TransactionFuture, client::UniswapV3Client, errors::UniswapV3Error,
    objects::send_with_gas_multiplier_map_err, strategies::Position,
};

use super::{result::withdraw_result, vault_error};

pub struct RevertWithdrawResponse {
    pub tx_hash: TxHash,
    pub token_id: TransactionFuture<U256>,
}

impl UniswapV3Client {
    /// Removes a debt-free position from the Revert Lend vault and sends the NFT to `recipient`
    /// (defaults to the signer).
    ///
    /// Fails if the loan still has debt; remove leverage with
    /// [`leverage_down`](UniswapV3Client::leverage_down) first.
    pub async fn withdraw_from_revert(
        &self,
        position: &Position,
        recipient: Option<Address>,
    ) -> Result<RevertWithdrawResponse, UniswapV3Error> {
        let token_id = position.position_id;
        let vault = self.require_revert_lend()?.vault();
        let signer = self.require_signer()?;
        self.ensure_revert_loan_owner(token_id, signer).await?;

        let debt_shares = self.revert_loan_debt_shares(token_id).await?;
        if !debt_shares.is_zero() {
            return Err(UniswapV3Error::Revert(format!(
                "loan {token_id} still has {debt_shares} debt shares; remove leverage first"
            )));
        }

        let contract = self.vault_contract()?;
        let call = contract.remove(token_id, recipient.unwrap_or(signer), Bytes::new());
        let pending =
            send_with_gas_multiplier_map_err(call, self.gas_multiplier(), vault_error).await?;

        Ok(RevertWithdrawResponse {
            tx_hash: *pending.tx_hash(),
            token_id: withdraw_result(pending, vault),
        })
    }
}
