use cosmwasm_schema::cw_serde;
use valence_middleware_utils::canonical_types::{
    bank::balance::ValenceBankBalance, pools::xyk::ValenceXykPool,
};

pub mod bank_balance;
pub mod gamm_pool;

// TODO: embed the previously deployed version identifier there
// to ensure that types declared here implement a 1-1 mapper from
// the outdated version to this one.
