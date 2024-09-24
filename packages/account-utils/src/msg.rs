use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::CosmosMsg;
use cw_ownable::cw_ownable_query;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String, // Initial owner of the contract
    pub approved_services: Vec<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    TransferAdmin { new_admin: String }, // Transfer ownership to new address (only admin)
    ApproveService { service: String },  // Add service to approved list (only admin)
    RemoveService { service: String },   // Remove service from approved list (only admin)
    ExecuteMsg { msgs: Vec<CosmosMsg> }, // Execute any CosmosMsg (approved services or admin)
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<String>)]
    ListApprovedServices {}, // Get list of approved services
}
