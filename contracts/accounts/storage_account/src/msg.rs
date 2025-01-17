use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use valence_middleware_utils::type_registry::types::ValenceType;

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    // Add library to approved list (only admin)
    ApproveLibrary { library: String },
    // Remove library from approved list (only admin)
    RemoveLibrary { library: String },
    // stores the given `ValenceType` variant under storage key `key`
    StoreValenceType { key: String, variant: ValenceType },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Get list of approved libraries
    #[returns(Vec<String>)]
    ListApprovedLibraries {},
    // Get Valence type variant from storage
    #[returns(ValenceType)]
    QueryValenceType { key: String },
}
