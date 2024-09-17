use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, StdError, StdResult, WasmMsg};
use services_utils::ServiceAccountType;

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

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    GetOwner {}, // Get current owner
    #[returns(Vec<String>)]
    ListApprovedServices {}, // Get list of approved services
}

pub fn execute_on_behalf_of(
    msgs: Vec<CosmosMsg>,
    account: &ServiceAccountType,
) -> StdResult<CosmosMsg> {
    match account {
        ServiceAccountType::AccountAddr(account) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: account.clone(),
            msg: to_json_binary(&ExecuteMsg::ExecuteMsg { msgs })?,
            funds: vec![],
        })),
        ServiceAccountType::AccountId(_) => {
            Err(StdError::generic_err("Account type is not an address"))
        }
    }
}
