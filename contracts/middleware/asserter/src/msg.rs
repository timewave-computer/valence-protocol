use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(String)]
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

impl Predicate {
    pub fn eval<T: PartialOrd + PartialEq>(&self, a: T, b: T) -> bool {
        match self {
            Predicate::LT => a < b,
            Predicate::LTE => a <= b,
            Predicate::EQ => a == b,
            Predicate::GT => a > b,
            Predicate::GTE => a >= b,
        }
    }
}

#[cw_serde]
pub struct QueryInfo {
    // addr of the storage account
    pub storage_account: String,
    // key to access the value in the storage account
    pub storage_slot_key: String,
    // b64 encoded query
    pub query: Binary,
}

#[cw_serde]
pub enum AssertionValue {
    // storage account slot query
    Variable(QueryInfo),
    // b64 encoded constant value
    Constant(Binary),
}

/// supported evaluation types. both assertion values must be of this type
/// in order to evaluate the condition.
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
