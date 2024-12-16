use cosmwasm_schema::cw_serde;
use valence_middleware_utils::canonical_types::{
    bank::balance::ValenceBankBalance, pools::xyk::ValenceXykPool,
};

pub mod bank_balance;
pub mod gamm_pool;

#[cw_serde]
pub enum ValenceType {
    ValenceXykPool(ValenceXykPool),
    ValenceBankBalance(ValenceBankBalance),
}
