use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(())]
    Assert(AssertionConfig),
}

#[cw_serde]
pub enum Predicate {
    LT,
    LTE,
    EQ,
    GT,
    GTE,
}

#[cw_serde]
pub struct QueryInfo {
    // addr of the storage account
    storage_account: String,
    // key to access the value in the storage account
    storage_slot_key: String,
    // b64 encoded query
    query: Binary,
}

#[cw_serde]

pub enum AssertionValue {
    // storage account slot query
    Variable(QueryInfo),
    // serialized constant value
    Constant(String),
}

// type that both values are expected to be
#[cw_serde]
pub enum ValueType {
    Decimal,
    Uint64,
    Uint128,
    Uint256,
    String,
}

#[cw_serde]
pub struct AssertionConfig {
    pub a: AssertionValue,
    pub predicate: Predicate,
    pub b: AssertionValue,
    pub ty: ValueType,
}
