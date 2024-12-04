use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    // Add library to approved list (only admin)
    ApproveLibrary { library: String },
    // Remove library from approved list (only admin)
    RemoveLibrary { library: String },
    // store a payload in storage
    PostBlob { key: String, value: Binary },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<String>)]
    ListApprovedLibraries {}, // Get list of approved libraries
    #[returns(Binary)]
    Blob { key: String }, // Get blob from storage
}
