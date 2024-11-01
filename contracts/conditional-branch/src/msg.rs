use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
use cw_ownable::cw_ownable_execute;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
}

#[cw_serde]
pub enum QueryInstruction {
    BalanceQuery {
        address: String,
        denom: String,
    },
    WasmQuery {
        contract_addr: String,
        msg: Binary,
        value_path: Vec<String>,
    },
    #[cfg(feature = "icq_queries")]
    IcqBalanceQuery {
        execution_id: u64,
        callback_address: String,
        connection_id: String,
        address: String,
        denom: String,
        update_period: u64,
    },
}

#[cw_serde]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    CompareAndBranch {
        query: QueryInstruction,
        operator: ComparisonOperator,
        rhs_operand: Binary,
        true_branch: Option<Binary>,
        false_branch: Option<Binary>,
    },
}

#[cw_serde]
pub enum QueryMsg {}
