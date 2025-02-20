// Since astroport is using an old CosmWasm version still, to make it compatible with our packages, we are going to redefine the messages here using Cosmwasm 2.x that we need
// for our contract
// The content of this file is taken from the 'astroport' crate, specifically version 5.0.0

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{coin, Addr, Binary, Coin, Decimal, DepsMut, StdError, StdResult, Uint128};
use valence_library_utils::error::LibraryError;

pub fn query_pool(deps: &DepsMut, pool_addr: &str) -> Result<Vec<Asset>, LibraryError> {
    let response: PoolResponse = deps
        .querier
        .query_wasm_smart(pool_addr, &PoolQueryMsg::Pool {})?;
    Ok(response.assets)
}

/// This structure holds the parameters that are returned from a swap simulation response
#[cw_serde]
pub struct SimulationResponse {
    /// The amount of ask assets returned by the swap
    pub return_amount: Uint128,
    /// The spread used in the swap operation
    pub spread_amount: Uint128,
    /// The amount of fees charged by the transaction
    pub commission_amount: Uint128,
}

/// This struct is used to return a query result with the total amount of LP tokens and assets in a specific pool.
#[cw_serde]
pub struct PoolResponse {
    /// The assets in the pool together with asset amounts
    pub assets: Vec<Asset>,
    /// The total amount of LP tokens currently issued
    pub total_share: Uint128,
}

/// This enum describes a Terra asset (native or CW20).
#[cw_serde]
pub struct Asset {
    /// Information about an asset stored in a [`AssetInfo`] struct
    pub info: AssetInfo,
    /// A token amount
    pub amount: Uint128,
}

impl Asset {
    pub fn as_coin(&self) -> StdResult<Coin> {
        match &self.info {
            AssetInfo::Token { .. } => {
                Err(StdError::generic_err("Cannot convert token asset to coin"))
            }
            AssetInfo::NativeToken { denom } => Ok(coin(self.amount.u128(), denom)),
        }
    }
}

#[cw_serde]
#[derive(Hash, Eq)]
pub enum AssetInfo {
    /// Non-native Token
    Token { contract_addr: Addr },
    /// Native token
    NativeToken { denom: String },
}

/// This structure describes the execute messages available in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Swap performs a swap in the pool
    Swap {
        offer_asset: Asset,
        ask_asset_info: Option<AssetInfo>,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
    },
    /// ProvideLiquidity allows someone to provide liquidity in the pool
    ProvideLiquidity {
        /// The assets available in the pool
        assets: Vec<Asset>,
        /// The slippage tolerance that allows liquidity provision only if the price in the pool doesn't move too much
        slippage_tolerance: Option<Decimal>,
        /// Determines whether the LP tokens minted for the user is auto_staked in the Incentives contract
        auto_stake: Option<bool>,
        /// The receiver of LP tokens
        receiver: Option<String>,
        min_lp_to_receive: Option<Uint128>,
    },
    /// WithdrawLiquidity allows someone to withdraw liquidity from the pool
    WithdrawLiquidity {
        #[serde(default)]
        assets: Vec<Asset>,
        min_assets_to_receive: Option<Vec<Asset>>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum FactoryQueryMsg {
    /// Pair returns information about a specific pair according to the specified assets.
    #[returns(PairInfo)]
    Pair {
        /// The assets for which we return a pair
        asset_infos: Vec<AssetInfo>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum PoolQueryMsg {
    /// Returns information about a pair in an object of type [`super::asset::PairInfo`].
    #[returns(PairInfo)]
    Pair {},
    /// Returns information about a pool in an object of type [`PoolResponse`].
    #[returns(PoolResponse)]
    Pool {},
    #[returns(SimulationResponse)]
    Simulation {
        offer_asset: Asset,
        ask_asset_info: Option<AssetInfo>,
    },
    /// Returns information about the share of the pool in a vector that contains objects of type [`Asset`].
    #[returns(Vec<Asset>)]
    Share { amount: Uint128 },
}

#[derive(Eq)]
#[cw_serde]
pub enum PairType {
    /// XYK pair type
    Xyk {},
    /// Stable pair type
    Stable {},
    /// Custom pair type
    Custom(String),
}

/// This structure stores the main parameters for an Astroport pair
#[cw_serde]
pub struct PairInfo {
    /// Asset information for the assets in the pool
    pub asset_infos: Vec<AssetInfo>,
    /// Pair contract address
    pub contract_addr: Addr,
    /// Pair LP token denom
    pub liquidity_token: String,
    /// The pool type (xyk, stableswap etc) available in [`PairType`]
    pub pair_type: PairType,
}

#[cw_serde]
pub struct FactoryInstantiateMsg {
    /// IDs of contracts that are allowed to instantiate pairs
    pub pair_configs: Vec<PairConfig>,
    /// CW20 token contract code identifier
    pub token_code_id: u64,
    /// Contract address to send governance fees to (the Maker)
    pub fee_address: Option<String>,
    /// Address of contract that is used to auto_stake LP tokens once someone provides liquidity in a pool
    pub generator_address: Option<String>,
    /// Address of owner that is allowed to change factory contract parameters
    pub owner: String,
    /// CW1 whitelist contract code id used to store 3rd party rewards for staking Astroport LP tokens
    pub whitelist_code_id: u64,
    /// The address of the contract that contains the coins and their accuracy
    pub coin_registry_address: String,
    /// Config for the tracking contract
    pub tracker_config: Option<TrackerConfig>,
}

#[cw_serde]
pub struct TrackerConfig {
    /// Tracking contract code id
    pub code_id: u64,
    /// Token factory module address
    pub token_factory_addr: String,
}

#[cw_serde]
pub struct PairConfig {
    /// ID of contract which is allowed to create pairs of this type
    pub code_id: u64,
    /// The pair type (provided in a [`PairType`])
    pub pair_type: PairType,
    /// The total fees (in bps) charged by a pair of this type
    pub total_fee_bps: u16,
    /// The amount of fees (in bps) collected by the Maker contract from this pair type
    pub maker_fee_bps: u16,
    /// Whether a pair type is disabled or not. If it is disabled, new pairs cannot be
    /// created, but existing ones can still read the pair configuration
    /// Default is false.
    #[serde(default)]
    pub is_disabled: bool,
    /// Setting this to true means that pairs of this type will not be able
    /// to get an ASTRO generator
    /// Default is false.
    #[serde(default)]
    pub is_generator_disabled: bool,
    /// If pool type is permissioned, only factory owner can create pairs of this type.
    /// Default is false.
    #[serde(default)]
    pub permissioned: bool,
}

#[cw_serde]
pub enum FactoryExecuteMsg {
    CreatePair {
        /// The pair type (exposed in [`PairType`])
        pair_type: PairType,
        /// The assets to create the pool for
        asset_infos: Vec<AssetInfo>,
        /// Optional binary serialised parameters for custom pool types
        init_params: Option<Binary>,
    },
}

#[cw_serde]
pub enum FactoryQueries {
    Pair {
        /// The assets for which we return a pair
        asset_infos: Vec<AssetInfo>,
    },
}

// The content of this file is taken from the 'astroport' crate, specifically version 5.7.0

#[cw_serde]
pub struct NativeCoinRegistryInstantiateMsg {
    /// Address allowed to change contract parameters
    pub owner: String,
}

#[cw_serde]
pub enum NativeCoinRegistryExecuteMsg {
    /// Adds or updates native assets with specified precisions.
    /// Only the current owner can execute this.
    /// Sender doesn't need to send any tokens.
    Add { native_coins: Vec<(String, u8)> },
    // emitting the rest
}

/// This structure holds concentrated pool parameters.
#[cw_serde]
pub struct ConcentratedPoolParams {
    /// Amplification coefficient affects trades close to price_scale
    pub amp: Decimal,
    /// Affects how gradual the curve changes from constant sum to constant product
    /// as price moves away from price scale. Low values mean more gradual.
    pub gamma: Decimal,
    /// The minimum fee, charged when pool is fully balanced
    pub mid_fee: Decimal,
    /// The maximum fee, charged when pool is imbalanced
    pub out_fee: Decimal,
    /// Parameter that defines how gradual the fee changes from fee_mid to fee_out
    /// based on distance from price_scale.
    pub fee_gamma: Decimal,
    /// Minimum profit before initiating a new repeg
    pub repeg_profit_threshold: Decimal,
    /// Minimum amount to change price_scale when repegging.
    pub min_price_scale_delta: Decimal,
    /// 1 x\[0] = price_scale * x\[1].
    pub price_scale: Decimal,
    /// Half-time used for calculating the price oracle.
    pub ma_half_time: u64,
    /// Whether asset balances are tracked over blocks or not.
    /// They will not be tracked if the parameter is ignored.
    /// It can not be disabled later once enabled.
    pub track_asset_balances: Option<bool>,
    /// The config for swap fee sharing
    pub fee_share: Option<FeeShareConfig>,
}

/// Holds the configuration for fee sharing
#[cw_serde]
pub struct FeeShareConfig {
    /// The fee shared with the address
    pub bps: u16,
    /// The share is sent to this address on every swap
    pub recipient: Addr,
}
