use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

/*
    These configurations are sourced from:
    - https://github.com/neutron-org/slinky-vault/tree/main/contracts/mmvault
    - https://github.com/neutron-org/neutron-std/tree/main/packages/neutron-std/src/types/slinky/types

    Eventually we should import them directly.
*/

#[cw_serde]
pub struct CurrencyPair {
    pub base: String,
    pub quote: String,
}

#[cw_serde]
pub struct TokenData {
    pub denom: String,
    pub decimals: u8,
    pub pair: CurrencyPair,
    pub max_blocks_old: u64,
}

#[cw_serde]
pub struct PairData {
    pub token_0: TokenData,
    pub token_1: TokenData,
    pub pair_id: String,
}

#[cw_serde]
pub struct FeeTier {
    pub fee: u64,
    pub percentage: u64,
}

#[cw_serde]
pub struct FeeTierConfig {
    pub fee_tiers: Vec<FeeTier>,
}

#[cw_serde]
pub struct Config {
    /// token and denom information
    pub pair_data: PairData,
    /// the denom of the contract's LP token
    pub lp_denom: String,
    /// total number of LP shares in existance
    pub total_shares: Uint128,
    /// list of addresses that can update the config and run restricted functions like dex_withdrawal and dex_deposit.
    pub whitelist: Vec<Addr>,
    /// maximum amount of dollar value that can be deposited into the contract
    pub deposit_cap: Uint128,
    /// location and weights of Deposits to be created
    pub fee_tier_config: FeeTierConfig,
    /// number of blocks until the contract is deemed stale.
    /// Once stale, the contract will be paused for 1 block before being allowed to execute again.
    pub timestamp_stale: u64,
    /// last block that action was executed to prevent staleness.
    pub last_executed: u64,
    /// The block when the contract was last paused due to stalenesss.
    pub pause_block: u64,
    /// whether the contract is paused. Paused contract cannot perform deposit functionalities.
    pub paused: bool,
    /// the oracle contract address. This contract will be used to get the price of the tokens.
    pub oracle_contract: Addr,
    /// whether to skew the AMM Deposits. If true, the AMM Deposit index will be skewed
    /// makING the over-supplied asset cheeper AND the under-supplied asset more expensive.
    pub skew: bool,
    /// the imbalance Factor indicated the rebalancing aggresiveness.
    pub imbalance: u32,
    /// General skew to add to the final deposit index of the vault
    pub oracle_price_skew: i32,
}
