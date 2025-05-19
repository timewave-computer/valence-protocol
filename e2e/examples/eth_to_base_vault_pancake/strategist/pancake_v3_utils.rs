use alloy::primitives::{U160, U256};
use log::info;
use std::error::Error;
use std::ops::{Div, Mul};
use uniswap_v3_math::tick_math::get_sqrt_ratio_at_tick;

/// Calculates the maximum amounts of token0 and token1 that can be used for the position along with the tick range
///
/// # Arguments
/// * `amount0_desired` - The desired amount of token0 to provide as liquidity
/// * `amount1_desired` - The desired amount of token1 to provide as liquidity
/// * `current_sqrt_price_x96` - The current square root price in Q96.64 format
/// * `current_tick` - The current tick of the pool
/// * `tick_spacing` - The tick spacing of the pool (depends on fee tier)
/// * `tick_price_range_percent` - The desired price range as a percentage (e.g., 5.0 for 5%)
pub fn calculate_max_amounts_position(
    amount0_desired: U256,
    amount1_desired: U256,
    current_sqrt_price_x96: U160,
    current_tick: i32,
    tick_spacing: i32,
    tick_price_range_percent: f64,
) -> Result<(i32, i32, U256, U256), Box<dyn Error + Send + Sync>> {
    // Calculate how many ticks correspond to the desired price range percentage
    // 1.0001 is the tick spacing constant in Uniswap V3 (each tick represents a 0.01% price change)
    let tick_range = ((1.0 + tick_price_range_percent / 100.0).ln() / 1.0001_f64.ln()) as i32;
    info!("Tick range: {}", tick_range);

    // Calculate lower and upper ticks that are valid according to the pool's tick spacing
    // Rounds down to the nearest valid tick below the range and adds another spacing for buffer
    let lower_tick = (((current_tick - tick_range) / tick_spacing) * tick_spacing) - tick_spacing;
    // Rounds up to the nearest valid tick above the range and adds another spacing for buffer
    let upper_tick = (((current_tick + tick_range) / tick_spacing) * tick_spacing) + tick_spacing;
    info!("Lower tick: {}", lower_tick);
    info!("Upper tick: {}", upper_tick);

    // Convert ticks to square root price ratios in Q96.64 format
    // These represent the price boundaries of the position
    let sqrt_ratio_a_x96 = get_sqrt_ratio_at_tick(lower_tick)?;
    let sqrt_ratio_b_x96 = get_sqrt_ratio_at_tick(upper_tick)?;
    info!("Sqrt ratio A: {}", sqrt_ratio_a_x96);
    info!("Sqrt ratio B: {}", sqrt_ratio_b_x96);

    // Calculate the maximum liquidity that can be provided with the desired amounts
    // for both token0 and token1
    let liquidity0 = get_liquidity_for_amount0(amount0_desired, sqrt_ratio_a_x96, sqrt_ratio_b_x96);
    let liquidity1 = get_liquidity_for_amount1(amount1_desired, sqrt_ratio_a_x96, sqrt_ratio_b_x96);
    info!("Liquidity for amount0: {}", liquidity0);
    info!("Liquidity for amount1: {}", liquidity1);

    // Choose the smaller liquidity value to ensure we don't exceed either desired amount
    // This is because we can only provide as much liquidity as the limiting token allows
    let liquidity = if liquidity0 < liquidity1 {
        liquidity0
    } else {
        liquidity1
    };

    // Convert current price to U256 for calculation compatibility
    let sqrt_price = U256::from(current_sqrt_price_x96);
    info!("Sqrt price: {}", sqrt_price);

    // Calculate the actual amounts of token0 and token1 that will be used
    // given the chosen liquidity value and the position's price range
    let amount0 = get_amount0_for_liquidity(sqrt_price, sqrt_ratio_b_x96, liquidity);
    let amount1 = get_amount1_for_liquidity(sqrt_ratio_a_x96, sqrt_price, liquidity);

    // Calculate scaling factors to use maximum available tokens while maintaining the ratio
    let scale_factor0 = amount0_desired.mul(U256::from(1_000_000)).div(amount0);
    let scale_factor1 = amount1_desired.mul(U256::from(1_000_000)).div(amount1);

    // Use the smaller scaling factor to ensure we don't exceed either balance
    let scale_factor = if scale_factor0 < scale_factor1 {
        scale_factor0
    } else {
        scale_factor1
    };

    // Scale the amounts to use maximum possible liquidity
    let final_amount0 = amount0.mul(scale_factor).div(U256::from(1_000_000));
    let final_amount1 = amount1.mul(scale_factor).div(U256::from(1_000_000));

    Ok((lower_tick, upper_tick, final_amount0, final_amount1))
}

/// Calculates the maximum liquidity that can be provided for a given amount of token0
///
/// # Arguments
/// * `amount0` - The amount of token0 available
/// * `sqrt_a_x96` - The lower square root price bound in Q96.64 format
/// * `sqrt_b_x96` - The upper square root price bound in Q96.64 format
fn get_liquidity_for_amount0(amount0: U256, sqrt_a_x96: U256, sqrt_b_x96: U256) -> U256 {
    // Implementation of the Uniswap V3 formula for calculating liquidity from token0 amount
    let numerator = amount0 * sqrt_a_x96 * sqrt_b_x96;
    let denominator = (sqrt_b_x96 - sqrt_a_x96) * U256::from(1u128 << 96);
    numerator / denominator
}

/// Calculates the maximum liquidity that can be provided for a given amount of token1
///
/// # Arguments
/// * `amount1` - The amount of token1 available
/// * `sqrt_a_x96` - The lower square root price bound in Q96.64 format
/// * `sqrt_b_x96` - The upper square root price bound in Q96.64 format
fn get_liquidity_for_amount1(amount1: U256, sqrt_a_x96: U256, sqrt_b_x96: U256) -> U256 {
    // Implementation of the Uniswap V3 formula for calculating liquidity from token1 amount
    let numerator = amount1 * U256::from(1u128 << 96);
    let denominator = sqrt_b_x96 - sqrt_a_x96;
    numerator / denominator
}

/// Calculates the amount of token0 that would be used for a given liquidity
///
/// # Arguments
/// * `sqrt_lower` - The current or lower square root price in Q96.64 format
/// * `sqrt_upper` - The upper square root price in Q96.64 format
/// * `liquidity` - The amount of liquidity to calculate for
fn get_amount0_for_liquidity(sqrt_lower: U256, sqrt_upper: U256, liquidity: U256) -> U256 {
    // Implementation of the Uniswap V3 formula for calculating token0 amount from liquidity
    let numerator = liquidity * U256::from(1u128 << 96) * (sqrt_upper - sqrt_lower);
    let denominator = sqrt_upper * sqrt_lower;
    numerator / denominator
}

/// Calculates the amount of token1 that would be used for a given liquidity
///
/// # Arguments
/// * `sqrt_lower` - The lower square root price in Q96.64 format
/// * `sqrt_upper` - The current or upper square root price in Q96.64 format
/// * `liquidity` - The amount of liquidity to calculate for
fn get_amount1_for_liquidity(sqrt_lower: U256, sqrt_upper: U256, liquidity: U256) -> U256 {
    // Implementation of the Uniswap V3 formula for calculating token1 amount from liquidity
    liquidity * (sqrt_upper - sqrt_lower) / U256::from(1u128 << 96)
}
