use alloy::{
    primitives::{
        Address, U256, U512,
        aliases::{I24, U160},
    },
    providers::Provider,
};
use uniswap_sdk_core::prelude::{BaseCurrency, Error, Token};
use uniswap_v3_math::tick_math::get_sqrt_ratio_at_tick;

use crate::{calltypes::BPS, errors::UniswapV3Error};

use super::{Factory, PoolContract, TokenExt};

/// Uniswap V3 tick bounds from `TickMath`.
const MIN_TICK: i32 = -887_272;
const MAX_TICK: i32 = 887_272;
const BPS_DENOMINATOR: i64 = 10_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pool {
    // Immutables
    factory: Factory,
    token0: Token,
    token1: Token,
    fee: u32,
    tick_spacing: i32,
}

impl Pool {
    pub fn new(
        factory: Factory,
        token_a: Token,
        token_b: Token,
        fee: u32,
        tick_spacing: i32,
    ) -> Result<Self, Error> {
        let token_a_sorts_before = factory.validate_pool_key(&token_a, &token_b, fee)?;
        if tick_spacing <= 0 {
            return Err(Error::Invalid("TICK_SPACING"));
        }

        let (token0, token1) = if token_a_sorts_before {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };

        Ok(Self {
            factory,
            token0,
            token1,
            fee,
            tick_spacing,
        })
    }

    pub async fn from_address<P: Provider>(
        address: Address,
        provider: &P,
    ) -> Result<Self, UniswapV3Error> {
        let contract = PoolContract::new(address, provider);

        let chain_id = provider
            .get_chain_id()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;
        let factory_address = contract
            .factory()
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;
        let token0_address = contract
            .token0()
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;
        let token1_address = contract
            .token1()
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;
        let fee = contract
            .fee()
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?
            .try_into()
            .map_err(|error| UniswapV3Error::RpcError(format!("invalid fee: {error}")))?;
        let tick_spacing = contract
            .tickSpacing()
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?
            .try_into()
            .map_err(|error| UniswapV3Error::RpcError(format!("invalid tick spacing: {error}")))?;

        let factory = Factory::new(chain_id, factory_address)
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;
        let token0 = Token::from_address(token0_address, chain_id, provider).await?;
        let token1 = Token::from_address(token1_address, chain_id, provider).await?;
        let pool = Self::new(factory, token0, token1, fee, tick_spacing)
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        if pool.address() != address {
            return Err(UniswapV3Error::RpcError(
                "address does not match the canonical pool key".to_string(),
            ));
        }

        Ok(pool)
    }

    pub const fn min_tick() -> i32 {
        MIN_TICK
    }

    pub const fn max_tick() -> i32 {
        MAX_TICK
    }

    pub fn validate_ticks(
        &self,
        tick_lower: i32,
        tick_upper: i32,
    ) -> Result<(I24, I24), UniswapV3Error> {
        if tick_lower >= tick_upper {
            return Err(UniswapV3Error::RequiredFieldMissing(
                "TICK_ORDER".to_string(),
            ));
        }
        if tick_lower < Self::min_tick() || tick_upper > Self::max_tick() {
            return Err(UniswapV3Error::RequiredFieldMissing(
                "TICK_BOUNDS".to_string(),
            ));
        }
        if tick_lower % self.tick_spacing != 0 || tick_upper % self.tick_spacing != 0 {
            return Err(UniswapV3Error::RequiredFieldMissing(
                "TICK_SPACING".to_string(),
            ));
        }

        let tick_lower_i24 = I24::try_from(tick_lower)
            .map_err(|error| UniswapV3Error::RequiredFieldMissing(error.to_string()))?;
        let tick_upper_i24 = I24::try_from(tick_upper)
            .map_err(|error| UniswapV3Error::RequiredFieldMissing(error.to_string()))?;

        Ok((tick_lower_i24, tick_upper_i24))
    }

    #[must_use]
    pub const fn factory(&self) -> &Factory {
        &self.factory
    }

    #[must_use]
    pub const fn token0(&self) -> &Token {
        &self.token0
    }

    #[must_use]
    pub const fn token1(&self) -> &Token {
        &self.token1
    }

    #[must_use]
    pub const fn fee(&self) -> u32 {
        self.fee
    }

    #[must_use]
    pub const fn tick_spacing(&self) -> i32 {
        self.tick_spacing
    }

    #[must_use]
    pub const fn chain_id(&self) -> u64 {
        self.factory.chain_id()
    }

    /// Number of usable ticks at this pool's spacing between aligned `MIN_TICK`/`MAX_TICK`.
    #[must_use]
    pub const fn num_ticks(&self) -> u32 {
        // Truncate to the nearest multiple of `tick_spacing`
        let min_tick = (MIN_TICK / self.tick_spacing) * self.tick_spacing;
        let max_tick = (MAX_TICK / self.tick_spacing) * self.tick_spacing;
        ((max_tick - min_tick) / self.tick_spacing + 1) as u32
    }

    /// Matches on-chain `maxLiquidityPerTick` = `type(uint128).max / numTicks`.
    #[must_use]
    pub const fn max_liquidity_per_tick(&self) -> u128 {
        u128::MAX / self.num_ticks() as u128
    }

    pub async fn sqrt_price_x96<P: Provider>(&self, provider: &P) -> Result<U160, UniswapV3Error> {
        Ok(self.slot0(provider).await?.sqrtPriceX96)
    }

    pub async fn tick<P: Provider>(&self, provider: &P) -> Result<I24, UniswapV3Error> {
        Ok(self.slot0(provider).await?.tick)
    }

    pub async fn get_both_ticks_away_from_mid<P: Provider>(
        &self,
        provider: &P,
        bps: BPS,
    ) -> Result<(i32, i32), UniswapV3Error> {
        let lower_tick = self.get_tick_away_from_mid(provider, bps.neg()).await?;
        let upper_tick = self
            .get_tick_away_from_mid(provider, bps.get().into())
            .await?;
        Ok((lower_tick, upper_tick))
    }

    /// Returns a spacing-aligned tick within `bps` of the live token1/token0 midprice.
    ///
    /// Positive values select a tick above the midprice; negative values select one below it.
    /// The returned tick never exceeds the requested price distance.
    pub async fn get_tick_away_from_mid<P: Provider>(
        &self,
        provider: &P,
        bps: i32,
    ) -> Result<i32, UniswapV3Error> {
        let factor = BPS_DENOMINATOR + i64::from(bps);
        if factor <= 0 {
            return Err(UniswapV3Error::RequiredFieldMissing("BPS".to_string()));
        }

        let mid_sqrt_price = U256::from(self.sqrt_price_x96(provider).await?);
        let target_factor = U512::from(factor as u64);
        let mid_price = U512::from(mid_sqrt_price) * U512::from(mid_sqrt_price);
        let target_price = mid_price * target_factor;
        let denominator = U512::from(BPS_DENOMINATOR as u64);

        let min_index = MIN_TICK.div_euclid(self.tick_spacing)
            + i32::from(MIN_TICK.rem_euclid(self.tick_spacing) != 0);
        let max_index = MAX_TICK.div_euclid(self.tick_spacing);
        let mut lower = min_index;
        let mut upper = max_index;

        while lower < upper {
            let middle = lower + (upper - lower + 1) / 2;
            let tick = middle * self.tick_spacing;
            let sqrt_price = get_sqrt_ratio_at_tick(tick)?;
            let price = U512::from(sqrt_price) * U512::from(sqrt_price);

            if price * denominator <= target_price {
                lower = middle;
            } else {
                upper = middle - 1;
            }
        }

        let lower_tick = lower * self.tick_spacing;
        let lower_sqrt_price = get_sqrt_ratio_at_tick(lower_tick)?;
        let lower_price = U512::from(lower_sqrt_price) * U512::from(lower_sqrt_price);
        let tick = if bps < 0 && lower_price * denominator < target_price {
            lower_tick
                .checked_add(self.tick_spacing)
                .ok_or_else(|| UniswapV3Error::RequiredFieldMissing("TICK_BOUNDS".to_string()))?
        } else {
            lower_tick
        };

        let tick_sqrt_price = get_sqrt_ratio_at_tick(tick)?;
        let tick_price = U512::from(tick_sqrt_price) * U512::from(tick_sqrt_price);
        let is_above_mid = tick_price > mid_price;
        let is_below_mid = tick_price < mid_price;
        let within_target = if bps >= 0 {
            tick_price * denominator <= target_price
        } else {
            tick_price * denominator >= target_price
        };

        if !within_target
            || (bps > 0 && !is_above_mid)
            || (bps < 0 && !is_below_mid)
            || (bps == 0 && tick_price != mid_price)
        {
            return Err(UniswapV3Error::TickNotFoundWithinBps);
        }

        Ok(tick)
    }

    pub async fn observation_index<P: Provider>(
        &self,
        provider: &P,
    ) -> Result<u16, UniswapV3Error> {
        Ok(self.slot0(provider).await?.observationIndex)
    }

    pub async fn observation_cardinality<P: Provider>(
        &self,
        provider: &P,
    ) -> Result<u16, UniswapV3Error> {
        Ok(self.slot0(provider).await?.observationCardinality)
    }

    pub async fn observation_cardinality_next<P: Provider>(
        &self,
        provider: &P,
    ) -> Result<u16, UniswapV3Error> {
        Ok(self.slot0(provider).await?.observationCardinalityNext)
    }

    pub async fn fee_protocol<P: Provider>(&self, provider: &P) -> Result<u8, UniswapV3Error> {
        Ok(self.slot0(provider).await?.feeProtocol)
    }

    #[must_use]
    pub fn address(&self) -> Address {
        self.factory
            .derive_pool_address(self.token0.address(), self.token1.address(), self.fee)
    }

    #[must_use]
    pub fn involves_token(&self, token: &Token) -> bool {
        self.token0.equals(token) || self.token1.equals(token)
    }

    async fn slot0<P: Provider>(
        &self,
        provider: &P,
    ) -> Result<PoolContract::slot0Return, UniswapV3Error> {
        PoolContract::new(self.address(), provider)
            .slot0()
            .call()
            .await
            .map_err(|error| UniswapV3Error::RpcError(error.to_string()))
    }
}
