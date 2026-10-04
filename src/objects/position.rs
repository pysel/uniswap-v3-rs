use alloy::{
    primitives::{Address, U256},
    providers::Provider,
};
use uniswap_v3_math::{
    sqrt_price_math::{_get_amount_0_delta, _get_amount_1_delta},
    tick_math::get_sqrt_ratio_at_tick,
};

use crate::{errors::UniswapV3Error, objects::PositionsReturn};

use super::NonfungiblePositionManager;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    manager: NonfungiblePositionManager,
    token_id: U256,
    token0: Address,
    token1: Address,
    fee: u32,
    tick_lower: i32,
    tick_upper: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PositionState {
    pub nonce: U256,
    pub operator: Address,
    pub liquidity: u128,
    pub fee_growth_inside_0_last_x128: U256,
    pub fee_growth_inside_1_last_x128: U256,
    pub tokens_owed_0: u128,
    pub tokens_owed_1: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenAmounts {
    pub amount0: U256,
    pub amount1: U256,
}

impl Position {
    pub(crate) async fn from_manager_token_id<P: Provider>(
        manager: NonfungiblePositionManager,
        token_id: U256,
        provider: &P,
    ) -> Result<Self, UniswapV3Error> {
        let raw = manager.positions(provider, token_id).await?;
        Ok(Self::from_raw(manager, token_id, raw))
    }

    pub(crate) fn from_raw(
        manager: NonfungiblePositionManager,
        token_id: U256,
        raw: PositionsReturn,
    ) -> Self {
        Self {
            manager,
            token_id,
            token0: raw.token0,
            token1: raw.token1,
            fee: raw.fee.to(),
            tick_lower: raw.tickLower.as_i32(),
            tick_upper: raw.tickUpper.as_i32(),
        }
    }

    #[cfg(test)]
    pub(crate) const fn new_unchecked(
        manager: NonfungiblePositionManager,
        token_id: U256,
        token0: Address,
        token1: Address,
        fee: u32,
        tick_lower: i32,
        tick_upper: i32,
    ) -> Self {
        Self {
            manager,
            token_id,
            token0,
            token1,
            fee,
            tick_lower,
            tick_upper,
        }
    }

    #[must_use]
    pub const fn manager(&self) -> NonfungiblePositionManager {
        self.manager
    }

    #[must_use]
    pub const fn manager_address(&self) -> Address {
        self.manager.address()
    }

    #[must_use]
    pub const fn token_id(&self) -> U256 {
        self.token_id
    }

    #[must_use]
    pub const fn token0(&self) -> Address {
        self.token0
    }

    #[must_use]
    pub const fn token1(&self) -> Address {
        self.token1
    }

    #[must_use]
    pub const fn fee(&self) -> u32 {
        self.fee
    }

    #[must_use]
    pub const fn tick_lower(&self) -> i32 {
        self.tick_lower
    }

    #[must_use]
    pub const fn tick_upper(&self) -> i32 {
        self.tick_upper
    }

    pub async fn owner<P: Provider>(&self, provider: &P) -> Result<Address, UniswapV3Error> {
        self.manager.owner_of(provider, self.token_id).await
    }

    pub async fn state<P: Provider>(&self, provider: &P) -> Result<PositionState, UniswapV3Error> {
        let raw = self.manager.positions(provider, self.token_id).await?;
        Ok(PositionState {
            nonce: U256::from(raw.nonce),
            operator: raw.operator,
            liquidity: raw.liquidity,
            fee_growth_inside_0_last_x128: raw.feeGrowthInside0LastX128,
            fee_growth_inside_1_last_x128: raw.feeGrowthInside1LastX128,
            tokens_owed_0: raw.tokensOwed0,
            tokens_owed_1: raw.tokensOwed1,
        })
    }

    pub async fn liquidity<P: Provider>(&self, provider: &P) -> Result<u128, UniswapV3Error> {
        Ok(self.state(provider).await?.liquidity)
    }

    pub async fn tokens_owed<P: Provider>(
        &self,
        provider: &P,
    ) -> Result<TokenAmounts, UniswapV3Error> {
        let state = self.state(provider).await?;
        Ok(TokenAmounts {
            amount0: U256::from(state.tokens_owed_0),
            amount1: U256::from(state.tokens_owed_1),
        })
    }

    /// Returns current amounts collectable by `collect(max, max)` in an `eth_call`.
    ///
    /// This includes stored `tokensOwed` plus currently unaccounted fees. After a liquidity
    /// decrease, NPM stores withdrawn principal and fees in the same owed buckets, so this method
    /// intentionally returns collectable token amounts rather than trying to classify their source.
    pub async fn collectable_amounts<P: Provider>(
        &self,
        provider: &P,
    ) -> Result<TokenAmounts, UniswapV3Error> {
        let (amount0, amount1) = self
            .manager
            .collectable_amounts(provider, self.token_id)
            .await?;
        Ok(TokenAmounts { amount0, amount1 })
    }
}

/// Principal token amounts held by `liquidity` over `[tick_lower, tick_upper)` at `sqrt_price_x96`.
pub(crate) fn token_amounts_for_liquidity(
    sqrt_price_x96: U256,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
) -> Result<TokenAmounts, UniswapV3Error> {
    let sqrt_price_lower_x96 = get_sqrt_ratio_at_tick(tick_lower)?;
    let sqrt_price_upper_x96 = get_sqrt_ratio_at_tick(tick_upper)?;

    let (amount0, amount1) = if sqrt_price_x96 <= sqrt_price_lower_x96 {
        (
            _get_amount_0_delta(sqrt_price_lower_x96, sqrt_price_upper_x96, liquidity, false)?,
            U256::ZERO,
        )
    } else if sqrt_price_x96 < sqrt_price_upper_x96 {
        (
            _get_amount_0_delta(sqrt_price_x96, sqrt_price_upper_x96, liquidity, false)?,
            _get_amount_1_delta(sqrt_price_lower_x96, sqrt_price_x96, liquidity, false)?,
        )
    } else {
        (
            U256::ZERO,
            _get_amount_1_delta(sqrt_price_lower_x96, sqrt_price_upper_x96, liquidity, false)?,
        )
    };

    Ok(TokenAmounts { amount0, amount1 })
}
