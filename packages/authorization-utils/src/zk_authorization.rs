use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Api, Binary};

use crate::{
    authorization::{
        AuthorizationMode, AuthorizationModeInfo, AuthorizationMsg, AuthorizationState,
    },
    domain::Domain,
};

#[cw_serde]
// What an owner or subowner can pass to the contract to create a ZK authorization
pub struct ZkAuthorizationInfo {
    // Unique ID for the authorization, will be used as denom of the TokenFactory token if needed
    pub label: String,
    pub mode: AuthorizationModeInfo,
    // ZK Specific:
    // The registry of the guest program that will be executed
    pub registry: u64,
    // The Verifying Key to be used for the Message
    pub vk: Binary,
    // Flag to indicate if we need to validate the last block execution of a specific ZK authorization
    pub validate_last_block_execution: bool,
}

impl ZkAuthorizationInfo {
    pub fn into_zk_authorization(self, api: &dyn Api) -> ZkAuthorization {
        ZkAuthorization {
            label: self.label,
            mode: self.mode.into_mode_validated(api),
            registry: self.registry,
            vk: self.vk,
            validate_last_block_execution: self.validate_last_block_execution,
            state: AuthorizationState::Enabled,
        }
    }
}

#[cw_serde]
pub struct ZkAuthorization {
    pub label: String,
    pub mode: AuthorizationMode,
    pub registry: u64,
    pub vk: Binary,
    pub validate_last_block_execution: bool,
    pub state: AuthorizationState,
}

#[cw_serde]
pub struct ZkMessage {
    pub registry: u64,
    pub block_number: u64,
    pub domain: Domain,
    // If this can only be executed on a specific authorization contract, we can optionally pass this here
    pub authorization_contract: Option<String>,
    pub message: AuthorizationMsg,
}
