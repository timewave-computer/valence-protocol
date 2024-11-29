use cosmwasm_schema::{cw_serde, QueryResponses};
use neutron_sdk::bindings::{msg::NeutronMsg, types::InterchainQueryResult};
use serde_json::Value;

#[cw_serde]
pub struct InstantiateMsg {
    // connection id of associated chain
    pub connection_id: String,
}

#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(QueryRegistrationInfoResponse)]
    GetRegistrationConfig(QueryRegistrationInfoRequest),

    #[returns(QueryReconstructionResponse)]
    ReconstructQuery(QueryReconstructionRequest),
}

#[cw_serde]
pub struct QueryReconstructionRequest {
    pub icq_result: InterchainQueryResult,
    pub query_type: QueryResult,
}

#[cw_serde]
pub struct QueryReconstructionResponse {
    pub json_value: Value,
}

#[cw_serde]
pub struct QueryRegistrationInfoRequest {
    pub module: String,
    pub query: String,
}

#[cw_serde]
pub struct QueryRegistrationInfoResponse {
    pub registration_msg: NeutronMsg,
    pub reply_id: u64,
    pub query_type: QueryResult,
}

#[cw_serde]
pub enum QueryResult {
    Gamm { result_type: GammResultTypes },
    Bank { result_type: BankResultTypes },
}

#[cw_serde]
pub enum GammResultTypes {
    Pool,
}

#[cw_serde]
pub enum BankResultTypes {
    AccountDenomBalance,
}

#[cw_serde]
pub struct PendingQueryIdConfig {
    pub associated_domain_registry: String,
    pub query_type: QueryResult,
}
