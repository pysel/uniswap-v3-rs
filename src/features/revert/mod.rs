//! Revert Lend: use Uniswap V3 positions as collateral in a `V3Vault` and lever them through the
//! vault's `LeverageTransformer`.

mod abi_definitions;
mod deposit;
mod leverage;
mod result;
mod swap;
mod withdraw;

use alloy::primitives::{Address, U256};

use crate::{client::UniswapV3Client, errors::UniswapV3Error};

pub use deposit::RevertDepositResponse;
pub use leverage::{
    LeverageDownResponse, LeverageDownResult, LeverageUpResponse, LeverageUpResult,
};
pub use withdraw::RevertWithdrawResponse;

pub(crate) use abi_definitions::LeverageTransformer as LeverageTransformerContract;
pub(crate) use abi_definitions::RevertVault as VaultContract;
pub(crate) use abi_definitions::UniversalRouterData;

/// Revert Lend deployment: a `V3Vault` and the `LeverageTransformer` it allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RevertLend {
    vault: Address,
    leverage_transformer: Address,
}

impl RevertLend {
    #[must_use]
    pub const fn new(vault: Address, leverage_transformer: Address) -> Self {
        Self {
            vault,
            leverage_transformer,
        }
    }

    #[must_use]
    pub const fn vault(&self) -> Address {
        self.vault
    }

    #[must_use]
    pub const fn leverage_transformer(&self) -> Address {
        self.leverage_transformer
    }
}

/// `V3Vault.loanInfo` values, denominated in the vault asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RevertLoanInfo {
    pub debt: U256,
    pub full_value: U256,
    pub collateral_value: U256,
    pub liquidation_cost: U256,
    pub liquidation_value: U256,
}

impl UniswapV3Client {
    pub async fn revert_loan_info(&self, token_id: U256) -> Result<RevertLoanInfo, UniswapV3Error> {
        let info = self
            .vault_contract()?
            .loanInfo(token_id)
            .call()
            .await
            .map_err(vault_error)?;

        Ok(RevertLoanInfo {
            debt: info.debt,
            full_value: info.fullValue,
            collateral_value: info.collateralValue,
            liquidation_cost: info.liquidationCost,
            liquidation_value: info.liquidationValue,
        })
    }

    pub async fn revert_loan_debt_shares(&self, token_id: U256) -> Result<U256, UniswapV3Error> {
        self.vault_contract()?
            .loans(token_id)
            .call()
            .await
            .map_err(vault_error)
    }

    /// Owner of the loan for `token_id`, or the zero address if it is not in the vault.
    pub async fn revert_loan_owner(&self, token_id: U256) -> Result<Address, UniswapV3Error> {
        self.vault_contract()?
            .ownerOf(token_id)
            .call()
            .await
            .map_err(vault_error)
    }

    /// Token ids of all positions `owner` has deposited in the vault.
    pub async fn revert_loans(&self, owner: Address) -> Result<Vec<U256>, UniswapV3Error> {
        let vault = self.vault_contract()?;
        let count = vault.loanCount(owner).call().await.map_err(vault_error)?;
        let count = usize::try_from(count)
            .map_err(|error| UniswapV3Error::RpcError(format!("loan count too large: {error}")))?;
        let mut token_ids = Vec::with_capacity(count);

        for index in 0..count {
            token_ids.push(
                vault
                    .loanAtIndex(owner, U256::from(index))
                    .call()
                    .await
                    .map_err(vault_error)?,
            );
        }

        Ok(token_ids)
    }

    pub(super) fn vault_contract(
        &self,
    ) -> Result<VaultContract::RevertVaultInstance<&alloy::providers::DynProvider>, UniswapV3Error>
    {
        Ok(VaultContract::new(
            self.require_revert_lend()?.vault(),
            self.provider(),
        ))
    }

    pub(super) async fn ensure_revert_loan_owner(
        &self,
        token_id: U256,
        owner: Address,
    ) -> Result<(), UniswapV3Error> {
        let loan_owner = self.revert_loan_owner(token_id).await?;
        if loan_owner != owner {
            return Err(UniswapV3Error::Revert(format!(
                "loan {token_id} is owned by {loan_owner}, not {owner}"
            )));
        }

        Ok(())
    }
}

/// Maps a vault call error, decoding Revert's custom errors (`NeedsRepay`, `CollateralFail`, ...).
pub(crate) fn vault_error(error: alloy::contract::Error) -> UniswapV3Error {
    match error.as_decoded_interface_error::<VaultContract::RevertVaultErrors>() {
        Some(decoded) => UniswapV3Error::Revert(format!("{decoded:?}")),
        None => UniswapV3Error::RpcError(error.to_string()),
    }
}
