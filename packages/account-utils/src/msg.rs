use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    from_json, to_json_binary, Binary, CosmosMsg, Reply, StdError, StdResult, SubMsg, SubMsgResult,
};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String, // Initial owner of the contract
    pub approved_services: Vec<String>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    ApproveService { service: String }, // Add service to approved list (only admin)
    RemoveService { service: String },  // Remove service from approved list (only admin)
    ExecuteMsg { msgs: Vec<CosmosMsg> }, // Execute any CosmosMsg (approved services or admin)
    ExecuteSubmsgs { msgs: Vec<SubMsg> },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<String>)]
    ListApprovedServices {}, // Get list of approved services
}

#[cw_serde]
pub struct ValenceCallback {
    pub id: u64,
    pub result: SubMsgResult,
    pub payload: Binary,
}

impl From<Reply> for ValenceCallback {
    fn from(value: Reply) -> Self {
        ValenceCallback {
            id: value.id,
            result: value.result,
            payload: value.payload,
        }
    }
}

impl ValenceCallback {
    pub fn try_from_sub_msg_result(sub_msg_result: SubMsgResult) -> StdResult<Self> {
        let sub_result = match sub_msg_result.into_result() {
            Ok(field) => field,
            Err(err) => return Err(StdError::generic_err(err)),
        };

        for event in sub_result.events {
            if event.ty == "wasm" {
                for attr in event.attributes {
                    if attr.key == "valence_callback" {
                        let valence_callback: ValenceCallback = match from_json(attr.value) {
                            Ok(field) => field,
                            Err(err) => return Err(StdError::generic_err(err.to_string())),
                        };
                        return Ok(valence_callback);
                    }
                }
            }
        }
        Err(StdError::generic_err("valence callback not found"))
    }
}
