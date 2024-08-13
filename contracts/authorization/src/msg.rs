use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    // If not provided, the owner will be the sender
    pub owner: Option<Addr>,
    // Sub-owners can be added later if needed
    pub sub_owners: Option<Vec<Addr>>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    OwnerAction(OwnerMsg),
    SubOwnerAction(SubOwnerMsg),
    UserAction(UserMsg),
}

#[cw_serde]
pub enum OwnerMsg {
    AddSubOwner { sub_owner: Addr },
    RemoveSubOwner { sub_owner: Addr },
}

#[cw_serde]
pub enum SubOwnerMsg {}

#[cw_serde]
pub enum UserMsg {}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<Addr>)]
    SubOwners {},
}
