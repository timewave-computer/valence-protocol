use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query, Expiration};
use valence_authorization_utils::{
    authorization::{Authorization, AuthorizationInfo, Priority},
    domain::{Domain, ExternalDomain},
};

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
        disabled_until: Option<Expiration>,
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
    // Method to remove any set of messages from any queue in any domain
    RemoveMsgs {
        // Which domain we are targetting
        domain: Domain,
        // position in the queue
        queue_position: u64,
        // what queue we are targetting
        priority: Priority,
    },
    // Method to add messages from an authorization to any queue
    AddMsgs {
        // The authorization label
        label: String,
        // Where and in which queue we are putting them
        queue_position: u64,
        priority: Priority,
        // Messages to add
        messages: Vec<Binary>,
    },
    // Pause a processor in any domain
    PauseProcessor {
        domain: Domain,
    },
    // Resume a processor in any domain
    ResumeProcessor {
        domain: Domain,
    },
}

#[cw_serde]
pub struct Mint {
    pub address: Addr,
    pub amount: Uint128,
}

#[cw_serde]
pub enum UserMsg {
    SendMsgs {
        label: String,
        messages: Vec<Binary>,
    },
}

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
