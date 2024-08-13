use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, CosmosMsg};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String, // Initial owner of the contract
}

#[cw_serde]
pub enum ExecuteMsg {
    TransferAdmin { new_admin: String }, // Transfer ownership to new address (only admin)
    ApproveService { service: String },  // Add service to approved list (only admin)
    RemoveService { service: String },   // Remove service from approved list (only admin)
    ExecuteMsg { msgs: Vec<CosmosMsg> }, // Execute any CosmosMsg (approved services or admin)
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    GetOwner {}, // Get current owner
    #[returns(Vec<String>)]
    ListApprovedServices {}, // Get list of approved services
}
