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
    pub query_type: String,
}

#[cw_serde]
pub struct QueryReconstructionResponse {
    pub json_value: Value,
}

#[cw_serde]
pub struct QueryRegistrationInfoRequest {
    /// module here refers to some string identifier of the query we want to perform.
    /// one useful identifier is that of the proto type, e.g. `/osmosis.gamm.v1beta1.Pool`.
    /// basically describes what type we are dealing with
    pub module: String,
    /// params here describe the parameters to be passed into our query request.
    /// if module above describes the what, these params describe the how.
    pub params: serde_json::Map<String, Value>,
}

#[cw_serde]
pub struct QueryRegistrationInfoResponse {
    pub registration_msg: NeutronMsg,
    pub reply_id: u64,
}

#[cw_serde]
pub struct PendingQueryIdConfig {
    pub associated_domain_registry: String,
    pub query_type: String,
}
