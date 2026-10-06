use alloy::{
    network::Ethereum,
    primitives::{Address, Bytes, TxHash, U256},
    providers::PendingTransactionBuilder,
    sol_types::SolCall,
};
use uniswap_sdk_core::prelude::Token;
use uniswap_v3_math::{
    full_math::{mul_div, mul_div_rounding_up},
    tick_math::get_sqrt_ratio_at_tick,
};

use crate::{
    calltypes::{
        BPS, Path, QuoteExactInputSingleParams, TransactionFuture, apply_negative_slippage,
        default_deadline,
    },
    client::UniswapV3Client,
    errors::UniswapV3Error,
    objects::{Pool, TokenAmounts, send_with_gas_multiplier_map_err, token_amounts_for_liquidity},
    strategies::Position,
};

use super::{
    LeverageTransformerContract, RevertLend,
    result::{leverage_down_result, leverage_up_result},
    swap::universal_router_swap_data,
    vault_error,
};

use LeverageTransformerContract::{LeverageDownParams, LeverageUpParams};

/// Liquidity used to sample the position's token ratio at the current price.
const REFERENCE_LIQUIDITY: u128 = 1 << 96;
/// Repay target as a percentage of the debt (101 = debt + 1%).
const LEVERAGE_DOWN_LIQUIDITY_BUFFER_SCALER: u64 = 101;
const ONE_USDC: U256 = U256::from_limbs([1_000_000, 0, 0, 0]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeverageUpResult {
    pub borrowed: U256,
    pub liquidity: u128,
    pub amount0: U256,
    pub amount1: U256,
}

pub struct LeverageUpResponse {
    pub tx_hash: TxHash,
    pub result: TransactionFuture<LeverageUpResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeverageDownResult {
    pub removed0: U256,
    pub removed1: U256,
    pub repaid: U256,
}

pub struct LeverageDownResponse {
    pub tx_hash: TxHash,
    pub result: TransactionFuture<LeverageDownResult>,
}

/// On-chain state both leverage directions need.
struct LeverageContext {
    revert: RevertLend,
    npm: Address,
    signer: Address,
    pool: Pool,
    asset_is_token0: bool,
    sqrt_price_x96: U256,
    universal_router: Address,
    deadline: U256,
}

impl LeverageContext {
    /// `(vault asset, other pool token)`.
    fn asset_and_other(&self) -> (&Token, &Token) {
        if self.asset_is_token0 {
            (self.pool.token0(), self.pool.token1())
        } else {
            (self.pool.token1(), self.pool.token0())
        }
    }

    /// Places `leg` on the non-asset side: `(leg0, leg1)`.
    fn legs(&self, leg: SwapLeg) -> (SwapLeg, SwapLeg) {
        if self.asset_is_token0 {
            (SwapLeg::default(), leg)
        } else {
            (leg, SwapLeg::default())
        }
    }
}

#[derive(Clone, Debug, Default)]
struct SwapLeg {
    amount_in: U256,
    amount_out_min: U256,
    data: Bytes,
}

impl UniswapV3Client {
    /// Borrows `borrow_amount` of the vault asset against a deposited position and adds it back
    /// as liquidity.
    ///
    /// The borrowed asset is split to match the position's current token ratio; the non-asset
    /// share is swapped through the position's own pool via the Universal Router. `slippage`
    /// bounds both the swap output and the added amounts. Leftovers go to the signer.
    pub async fn leverage_up(
        &self,
        position: &Position,
        borrow_amount: U256,
        slippage: BPS,
    ) -> Result<LeverageUpResponse, UniswapV3Error> {
        if borrow_amount.is_zero() {
            return Err(UniswapV3Error::Revert(
                "borrow amount must be non-zero".to_string(),
            ));
        }

        let ctx = self.leverage_context(position).await?;
        let (asset, other) = ctx.asset_and_other();
        let per_liquidity = token_amounts_for_liquidity(
            ctx.sqrt_price_x96,
            position.lower_tick,
            position.upper_tick,
            REFERENCE_LIQUIDITY,
        )?;
        let swap_in = swap_amount(
            borrow_amount,
            per_liquidity,
            ctx.sqrt_price_x96,
            ctx.asset_is_token0,
        )?;

        let swap = if swap_in.is_zero() {
            SwapLeg::default()
        } else {
            self.swap_leg(&ctx, asset, other, swap_in, slippage).await?
        };

        let kept = borrow_amount - swap_in;
        let (add0, add1) = if ctx.asset_is_token0 {
            (kept, swap.amount_out_min)
        } else {
            (swap.amount_out_min, kept)
        };
        let liquidity = liquidity_for_amounts(
            ctx.sqrt_price_x96,
            get_sqrt_ratio_at_tick(position.lower_tick)?,
            get_sqrt_ratio_at_tick(position.upper_tick)?,
            add0,
            add1,
        )?;
        let expected = token_amounts_for_liquidity(
            ctx.sqrt_price_x96,
            position.lower_tick,
            position.upper_tick,
            liquidity,
        )?;

        let (leg0, leg1) = ctx.legs(swap);
        let params = LeverageUpParams {
            tokenId: position.position_id,
            borrowAmount: borrow_amount,
            amountIn0: leg0.amount_in,
            amountOut0Min: leg0.amount_out_min,
            swapData0: leg0.data,
            amountIn1: leg1.amount_in,
            amountOut1Min: leg1.amount_out_min,
            swapData1: leg1.data,
            amountAddMin0: apply_negative_slippage(expected.amount0, slippage)?,
            amountAddMin1: apply_negative_slippage(expected.amount1, slippage)?,
            recipient: ctx.signer,
            deadline: ctx.deadline,
        };

        let data = LeverageTransformerContract::leverageUpCall { params }.abi_encode();
        let pending = self
            .send_transform(&ctx, position.position_id, data)
            .await?;

        Ok(LeverageUpResponse {
            tx_hash: *pending.tx_hash(),
            result: leverage_up_result(pending, ctx.revert.vault(), ctx.npm),
        })
    }

    /// Removes the slice of a deposited position's liquidity worth its debt (plus a buffer),
    /// capped at `liquidity_bps` of the current liquidity, swaps that slice's non-asset token to
    /// the vault asset and repays debt with it.
    ///
    /// The remaining liquidity stays in the position. The vault caps repayment at the
    /// outstanding debt; any surplus goes to the signer.
    /// Uncollected fees stay in the position.
    pub async fn leverage_down(
        &self,
        position: &Position,
        liquidity_bps: BPS,
        slippage: BPS,
    ) -> Result<LeverageDownResponse, UniswapV3Error> {
        let bps = liquidity_bps.get();
        if bps == 0 || bps > BPS::denominator() {
            return Err(UniswapV3Error::Revert(format!(
                "liquidity bps must be in 1..={}, got {bps}",
                BPS::denominator()
            )));
        }

        let ctx = self.leverage_context(position).await?;
        let debt = self.revert_loan_info(position.position_id).await?.debt;
        if debt.is_zero() {
            return Err(UniswapV3Error::Revert(format!(
                "loan {} has no debt to repay",
                position.position_id
            )));
        }

        let current = self
            .get_position(position.position_id)
            .await?
            .liquidity(self.provider())
            .await?;

        // Repay target with headroom for swap fees, slippage and accrued interest; the vault caps
        // repayment at the actual debt and returns the excess.
        let target = mul_div(
            debt,
            U256::from(LEVERAGE_DOWN_LIQUIDITY_BUFFER_SCALER),
            U256::from(100),
        )?;

        // Position value in token1 units; convert to the vault asset when that is token0.
        let mut position_value = self
            .estimate_position_value_from_pool_price(position.pool, position.position_id)
            .await?;
        
        if ctx.asset_is_token0 {
            position_value = mul_div(
                mul_div(position_value, q96(), ctx.sqrt_price_x96)?,
                q96(),
                ctx.sqrt_price_x96,
            )?;
        }

        // Amounts are linear in liquidity at a fixed price, so the slice worth `target` is
        // `current * target / value`. Rounded up so the slice never comes out short.
        let debt_liquidity = if target >= position_value {
            current
        } else {
            mul_div_rounding_up(U256::from(current), target, position_value)?.to::<u128>()
        };

        let liquidity = scale_liquidity(current, liquidity_bps).min(debt_liquidity);
        if liquidity == 0 {
            return Err(UniswapV3Error::Revert(format!(
                "position {} has no liquidity to remove",
                position.position_id
            )));
        }

        let expected = token_amounts_for_liquidity(
            ctx.sqrt_price_x96,
            position.lower_tick,
            position.upper_tick,
            liquidity,
        )?;
        
        let remove_min0 = apply_negative_slippage(expected.amount0, slippage)?;
        let remove_min1 = apply_negative_slippage(expected.amount1, slippage)?;

        // Swap only what is guaranteed to be collected, so the swap never exceeds the balance.
        let (asset, other) = ctx.asset_and_other();
        let other_in = if ctx.asset_is_token0 {
            remove_min1
        } else {
            remove_min0
        };
        let swap = if other_in.is_zero() {
            SwapLeg::default()
        } else {
            self.swap_leg(&ctx, other, asset, other_in, slippage)
                .await?
        };

        let (leg0, leg1) = ctx.legs(swap);
        let params = LeverageDownParams {
            tokenId: position.position_id,
            liquidity,
            amountRemoveMin0: remove_min0,
            amountRemoveMin1: remove_min1,
            feeAmount0: 0,
            feeAmount1: 0,
            amountIn0: leg0.amount_in,
            amountOut0Min: leg0.amount_out_min,
            swapData0: leg0.data,
            amountIn1: leg1.amount_in,
            amountOut1Min: leg1.amount_out_min,
            swapData1: leg1.data,
            recipient: ctx.signer,
            deadline: ctx.deadline,
        };

        let data = LeverageTransformerContract::leverageDownCall { params }.abi_encode();
        let pending = self
            .send_transform(&ctx, position.position_id, data)
            .await?;

        Ok(LeverageDownResponse {
            tx_hash: *pending.tx_hash(),
            result: leverage_down_result(pending, ctx.revert.vault(), ctx.npm),
        })
    }

    async fn leverage_context(
        &self,
        position: &Position,
    ) -> Result<LeverageContext, UniswapV3Error> {
        let npm = self.require_position_manager()?.address();
        let revert = *self.require_revert_lend()?;
        let signer = self.require_signer()?;
        self.ensure_revert_loan_owner(position.position_id, signer)
            .await?;

        let vault = self.vault_contract()?;
        let allowed = vault
            .transformerAllowList(revert.leverage_transformer())
            .call()
            .await
            .map_err(vault_error)?;
        if !allowed {
            return Err(UniswapV3Error::Revert(format!(
                "leverage transformer {} is not allowed by vault {}",
                revert.leverage_transformer(),
                revert.vault()
            )));
        }

        let pool = Pool::from_address(position.pool, self).await?;
        let asset = vault.asset().call().await.map_err(vault_error)?;
        let asset_is_token0 = if asset == pool.token0().address {
            true
        } else if asset == pool.token1().address {
            false
        } else {
            return Err(UniswapV3Error::Revert(format!(
                "vault asset {asset} is not a token of position {}",
                position.position_id
            )));
        };

        let sqrt_price_x96 = U256::from(pool.sqrt_price_x96(self.provider()).await?);

        let universal_router =
            LeverageTransformerContract::new(revert.leverage_transformer(), self.provider())
                .universalRouter()
                .call()
                .await
                .map_err(|error| UniswapV3Error::RpcError(error.to_string()))?;

        let deadline = default_deadline()
            .ok_or_else(|| UniswapV3Error::Revert("system clock before unix epoch".to_string()))?;

        Ok(LeverageContext {
            revert,
            npm,
            signer,
            pool,
            asset_is_token0,
            sqrt_price_x96,
            universal_router,
            deadline,
        })
    }

    /// Quotes `amount_in` of `token_in` → `token_out` through the position's pool and encodes it
    /// as transformer `swapData`.
    async fn swap_leg(
        &self,
        ctx: &LeverageContext,
        token_in: &Token,
        token_out: &Token,
        amount_in: U256,
        slippage: BPS,
    ) -> Result<SwapLeg, UniswapV3Error> {
        let mut path = Path::new(token_in.clone());
        path.add_hop(token_out.clone(), ctx.pool.fee())?;

        let quote = self
            .quote_exact_input_single(
                QuoteExactInputSingleParams::builder(&path)
                    .amount_in(amount_in)
                    .then_default()
                    .build()?,
            )
            .await?;
        let amount_out_min = apply_negative_slippage(quote.amount_out, slippage)?;
        let data = universal_router_swap_data(
            ctx.universal_router,
            &path,
            amount_in,
            amount_out_min,
            ctx.revert.leverage_transformer(),
            ctx.deadline,
        )?;

        Ok(SwapLeg {
            amount_in,
            amount_out_min,
            data,
        })
    }

    async fn send_transform(
        &self,
        ctx: &LeverageContext,
        token_id: U256,
        data: Vec<u8>,
    ) -> Result<PendingTransactionBuilder<Ethereum>, UniswapV3Error> {
        let contract = self.vault_contract()?;
        let call = contract.transform(token_id, ctx.revert.leverage_transformer(), data.into());
        send_with_gas_multiplier_map_err(call, self.gas_multiplier(), vault_error).await
    }
}

fn q96() -> U256 {
    U256::from(1) << 96
}

/// Share of `borrow_amount` (vault asset) to swap into the other token so the split matches the
/// value ratio of `per_liquidity` at `sqrt_price_x96`.
fn swap_amount(
    borrow_amount: U256,
    per_liquidity: TokenAmounts,
    sqrt_price_x96: U256,
    asset_is_token0: bool,
) -> Result<U256, UniswapV3Error> {
    let (asset_value, other_value) = if asset_is_token0 {
        // token1 → token0: amount1 / price
        let other = mul_div(
            mul_div(per_liquidity.amount1, q96(), sqrt_price_x96)?,
            q96(),
            sqrt_price_x96,
        )?;
        (per_liquidity.amount0, other)
    } else {
        // token0 → token1: amount0 * price
        let other = mul_div(
            mul_div(per_liquidity.amount0, sqrt_price_x96, q96())?,
            sqrt_price_x96,
            q96(),
        )?;
        (per_liquidity.amount1, other)
    };

    let total = asset_value
        .checked_add(other_value)
        .ok_or_else(|| UniswapV3Error::Math("position value overflowed".to_string()))?;
    if total.is_zero() {
        return Err(UniswapV3Error::Math(
            "position range has no value at current price".to_string(),
        ));
    }

    Ok(mul_div(borrow_amount, other_value, total)?)
}

/// `LiquidityAmounts.getLiquidityForAmounts`: max liquidity addable with `amount0`/`amount1`.
fn liquidity_for_amounts(
    sqrt_price_x96: U256,
    sqrt_price_lower_x96: U256,
    sqrt_price_upper_x96: U256,
    amount0: U256,
    amount1: U256,
) -> Result<u128, UniswapV3Error> {
    let liquidity0 = |lower: U256, upper: U256| -> Result<U256, UniswapV3Error> {
        let intermediate = mul_div(lower, upper, q96())?;
        Ok(mul_div(amount0, intermediate, upper - lower)?)
    };
    let liquidity1 = |lower: U256, upper: U256| -> Result<U256, UniswapV3Error> {
        Ok(mul_div(amount1, q96(), upper - lower)?)
    };

    let liquidity = if sqrt_price_x96 <= sqrt_price_lower_x96 {
        liquidity0(sqrt_price_lower_x96, sqrt_price_upper_x96)?
    } else if sqrt_price_x96 < sqrt_price_upper_x96 {
        liquidity0(sqrt_price_x96, sqrt_price_upper_x96)?
            .min(liquidity1(sqrt_price_lower_x96, sqrt_price_x96)?)
    } else {
        liquidity1(sqrt_price_lower_x96, sqrt_price_upper_x96)?
    };

    Ok(u128::try_from(liquidity).unwrap_or(u128::MAX))
}

fn scale_liquidity(liquidity: u128, bps: BPS) -> u128 {
    let scaled = U256::from(liquidity) * U256::from(bps.get()) / U256::from(BPS::denominator());
    // `bps <= denominator`, so `scaled <= liquidity`.
    scaled.to::<u128>()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TICK_LOWER: i32 = -600;
    const TICK_UPPER: i32 = 600;

    fn sqrt(tick: i32) -> U256 {
        get_sqrt_ratio_at_tick(tick).unwrap()
    }

    fn per_liquidity(tick: i32) -> TokenAmounts {
        token_amounts_for_liquidity(sqrt(tick), TICK_LOWER, TICK_UPPER, REFERENCE_LIQUIDITY)
            .unwrap()
    }

    #[test]
    fn swaps_about_half_at_range_center() {
        let borrow = U256::from(1_000_000);
        for asset_is_token0 in [true, false] {
            let swap = swap_amount(borrow, per_liquidity(0), sqrt(0), asset_is_token0).unwrap();
            let diff = swap.abs_diff(U256::from(500_000));
            assert!(diff <= U256::from(1_000), "swap={swap}");
        }
    }

    #[test]
    fn swaps_nothing_or_everything_out_of_range() {
        let borrow = U256::from(1_000_000);

        // Below range: position is all token0.
        let below = per_liquidity(-1_200);
        assert_eq!(
            swap_amount(borrow, below, sqrt(-1_200), true).unwrap(),
            U256::ZERO
        );
        assert_eq!(
            swap_amount(borrow, below, sqrt(-1_200), false).unwrap(),
            borrow
        );

        // Above range: position is all token1.
        let above = per_liquidity(1_200);
        assert_eq!(
            swap_amount(borrow, above, sqrt(1_200), true).unwrap(),
            borrow
        );
        assert_eq!(
            swap_amount(borrow, above, sqrt(1_200), false).unwrap(),
            U256::ZERO
        );
    }

    #[test]
    fn liquidity_for_amounts_round_trips() {
        let liquidity = 10_u128.pow(18);
        let amounts =
            token_amounts_for_liquidity(sqrt(100), TICK_LOWER, TICK_UPPER, liquidity).unwrap();
        let recovered = liquidity_for_amounts(
            sqrt(100),
            sqrt(TICK_LOWER),
            sqrt(TICK_UPPER),
            amounts.amount0,
            amounts.amount1,
        )
        .unwrap();

        assert!(recovered <= liquidity);
        assert!(liquidity - recovered <= liquidity / 1_000_000);
    }

    #[test]
    fn scales_liquidity_by_bps() {
        assert_eq!(scale_liquidity(1_000, BPS::new(10_000)), 1_000);
        assert_eq!(scale_liquidity(1_000, BPS::new(2_500)), 250);
        assert_eq!(scale_liquidity(u128::MAX, BPS::new(10_000)), u128::MAX);
    }
}
