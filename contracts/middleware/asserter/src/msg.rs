use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
use valence_middleware_utils::type_registry::queries::ValencePrimitive;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    Assert {
        a: AssertionValue,
        predicate: Predicate,
        b: AssertionValue,
    },
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
    // constant valence primitive value
    Constant(ValencePrimitive),
}
