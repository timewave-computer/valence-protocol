use authorization_utils::{
    authorization::{Authorization, AuthorizationInfo, Priority},
    domain::ExternalDomain,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query, Expiration};

#[cw_serde]
pub struct InstantiateMsg {
    // If not provided, the owner will be the sender
    pub owner: Option<Addr>,
    // Sub-owners can be added later if needed
    pub sub_owners: Vec<Addr>,
    // Processor on Main domain
    pub processor: Addr,
    // External domains
    pub external_domains: Vec<ExternalDomain>,
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
pub enum SubOwnerMsg {
    AddExternalDomains {
        external_domains: Vec<ExternalDomain>,
    },
    CreateAuthorizations {
        authorizations: Vec<AuthorizationInfo>,
    },
    ModifyAuthorization {
        label: String,
        expiration: Option<Expiration>,
        max_concurrent_executions: Option<u64>,
        priority: Option<Priority>,
    },
    DisableAuthorization {
        label: String,
    },
    EnableAuthorization {
        label: String,
    },
    // Mint authorizations is only used for permissioned authorizations
    MintAuthorizations {
        label: String,
        mints: Vec<Mint>,
    },
}

#[cw_serde]
pub struct Mint {
    pub address: Addr,
    pub amount: Uint128,
}

#[cw_serde]
pub enum UserMsg {}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<Addr>)]
    SubOwners {},
    #[returns(Addr)]
    Processor {},
    #[returns(Vec<ExternalDomain>)]
    ExternalDomains {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(Vec<Authorization>)]
    Authorizations {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}
