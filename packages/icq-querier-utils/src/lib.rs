use cosmwasm_schema::{cw_serde, QueryResponses};
use neutron_sdk::bindings::msg::NeutronMsg;

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
}
