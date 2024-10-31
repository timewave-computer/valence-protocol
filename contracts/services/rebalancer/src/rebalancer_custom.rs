use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Uint128};

#[derive(
    ::cosmwasm_schema::serde::Serialize,
    ::cosmwasm_schema::serde::Deserialize,
    ::std::clone::Clone,
    ::std::fmt::Debug,
    ::cosmwasm_schema::schemars::JsonSchema,
)]
#[allow(clippy::derive_partial_eq_without_eq)] // Allow users of `#[cw_serde]` to not implement Eq without clippy complaining
#[serde(deny_unknown_fields, crate = "::cosmwasm_schema::serde")]
#[schemars(crate = "::cosmwasm_schema::schemars")]
#[derive(Eq)]
pub struct Target {
    /// The name of the denom
    pub denom: String,
    /// The percentage of the total balance we want to have in this denom
    pub bps: u64,
    /// The minimum balance the account should hold for this denom.
    /// Can only be a single one for an account
    pub min_balance: Option<Uint128>,
}

impl PartialEq for Target {
    fn eq(&self, other: &Target) -> bool {
        self.denom == other.denom
    }
}

impl Hash for Target {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.denom.hash(state);
    }
}

#[cw_serde]
pub struct RebalancerData {
    /// The trustee address that can pause/resume the service
    pub trustee: Option<String>,
    /// Base denom we will be calculating everything based on
    pub base_denom: String,
    /// List of targets to rebalance for this account
    pub targets: HashSet<Target>,
    /// PID parameters the account want to calculate the rebalance with
    pub pid: PID,
    /// The max limit in percentage the rebalancer is allowed to sell in cycle
    pub max_limit_bps: Option<u64>, // BPS
    /// The strategy to use when overriding targets
    pub target_override_strategy: TargetOverrideStrategy,
    #[serde(default)]
    pub account_type: RebalancerAccountType,
}

#[cw_serde]
pub enum TargetOverrideStrategy {
    Proportional,
    Priority,
}

#[cw_serde]
#[derive(Default)]
pub enum RebalancerAccountType {
    #[default]
    Regular,
    Workflow,
}

#[cw_serde]
pub struct PID {
    pub p: String,
    pub i: String,
    pub d: String,
}

#[cw_serde]
pub enum ServicesManagerExecuteMsg {
    /// Register sender to a service.
    RegisterToService {
        service_name: ValenceServices,
        data: Option<Binary>,
    },
}

#[cw_serde]
#[derive(Copy)]
pub enum ValenceServices {
    /// The rebalancer service
    Rebalancer,
    // /// A boilerplate placeholder for a future services
    // // also look at service management tests
    // Test,
}
