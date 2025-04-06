use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128};
use cw_ownable::cw_ownable_query;
use valence_ibc_utils::types::PacketForwardMiddlewareConfig;
use valence_library_utils::LibraryAccountType;
use valence_library_utils::{error::LibraryError, msg::LibraryConfigValidation};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
pub enum FunctionMsgs {
    Transfer {},
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
/// Enum representing the different query messages that can be sent.
pub enum QueryMsg {}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    // Address of the input account (Valence interchain account)
    pub input_addr: LibraryAccountType,
    // Amount that is going to be transferred
    pub amount: Uint128,
    // Denom that is going to be transferred
    pub denom: String,
    // Receiver on the other chain
    pub receiver: String,
    // Memo to be passed in the IBC transfer message.
    pub memo: String,
    // Remote chain info
    pub remote_chain_info: RemoteChainInfo,
    // Denom map for the Packet-Forwarding Middleware, to perform a multi-hop transfer.
    pub denom_to_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
}

#[cw_serde]
pub struct RemoteChainInfo {
    // Channel ID to be used
    pub channel_id: String,
    // Timeout for the IBC transfer in seconds. If not specified, a default 600 seconds will be used will be used
    pub ibc_transfer_timeout: Option<u64>,
}

impl RemoteChainInfo {
    pub fn new(channel_id: String, ibc_transfer_timeout: Option<u64>) -> Self {
        Self {
            channel_id,
            ibc_transfer_timeout,
        }
    }
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        amount: Uint128,
        denom: String,
        receiver: String,
        memo: String,
        remote_chain_info: RemoteChainInfo,
        denom_to_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            amount,
            denom,
            receiver,
            memo,
            remote_chain_info,
            denom_to_pfm_map,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<Addr, LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        if self.amount.is_zero() {
            return Err(LibraryError::ConfigurationError(
                "Invalid ICA IBC transfer config: amount cannot be zero.".to_string(),
            ));
        }

        if self.denom.is_empty() {
            return Err(LibraryError::ConfigurationError(
                "Invalid ICA IBC transfer config: denom cannot be empty.".to_string(),
            ));
        }

        if self.remote_chain_info.channel_id.is_empty() {
            return Err(LibraryError::ConfigurationError(
                "Invalid ICA IBC transfer config: channel_id cannot be empty.".to_string(),
            ));
        }

        if self.receiver.is_empty() {
            return Err(LibraryError::ConfigurationError(
                "Invalid ICA IBC transfer config: receiver cannot be empty.".to_string(),
            ));
        }

        if let Some(timeout) = self.remote_chain_info.ibc_transfer_timeout {
            if timeout == 0 {
                return Err(LibraryError::ConfigurationError(
                    "Invalid ICA IBC transfer config: timeout cannot be zero.".to_string(),
                ));
            }
        }

        Ok(input_addr)
    }
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let input_addr = self.do_validate(deps.api)?;

        Ok(Config {
            input_addr,
            amount: self.amount,
            denom: self.denom.clone(),
            receiver: self.receiver.clone(),
            memo: self.memo.clone(),
            remote_chain_info: self.remote_chain_info.clone(),
            denom_to_pfm_map: self.denom_to_pfm_map.clone(),
        })
    }
}

impl LibraryConfigUpdate {
    pub fn update_config(self, deps: DepsMut) -> Result<(), LibraryError> {
        let mut config: Config = valence_library_base::load_config(deps.storage)?;

        // First update input_addr (if needed)
        if let Some(input_addr) = self.input_addr {
            config.input_addr = input_addr.to_addr(deps.api)?;
        }

        // Next update the amount (if needed)
        if let Some(amount) = self.amount {
            if amount.is_zero() {
                return Err(LibraryError::ConfigurationError(
                    "Invalid ICA IBC transfer config: amount cannot be zero.".to_string(),
                ));
            }
            config.amount = amount;
        }

        // Next update the denom (if needed)
        if let Some(denom) = self.denom {
            if denom.is_empty() {
                return Err(LibraryError::ConfigurationError(
                    "Invalid ICA IBC transfer config: denom cannot be empty.".to_string(),
                ));
            }
            config.denom = denom;
        }

        // Next update the receiver (if needed)
        if let Some(receiver) = self.receiver {
            if receiver.is_empty() {
                return Err(LibraryError::ConfigurationError(
                    "Invalid ICA IBC transfer config: receiver cannot be empty.".to_string(),
                ));
            }
            config.receiver = receiver;
        }

        // Next update the remote_chain_info (if needed)
        if let Some(remote_chain_info) = self.remote_chain_info {
            if remote_chain_info.channel_id.is_empty() {
                return Err(LibraryError::ConfigurationError(
                    "Invalid ICA IBC transfer config: channel_id cannot be empty.".to_string(),
                ));
            }

            if let Some(timeout) = remote_chain_info.ibc_transfer_timeout {
                if timeout == 0 {
                    return Err(LibraryError::ConfigurationError(
                        "Invalid ICA IBC transfer config: timeout cannot be zero.".to_string(),
                    ));
                }
            }

            config.remote_chain_info = remote_chain_info;
        }

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}

#[cw_serde]
pub struct Config {
    pub input_addr: Addr,
    pub amount: Uint128,
    pub denom: String,
    pub receiver: String,
    pub memo: String,
    pub remote_chain_info: RemoteChainInfo,
    pub denom_to_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
}

impl Config {
    pub fn new(
        input_addr: Addr,
        amount: Uint128,
        denom: String,
        receiver: String,
        memo: String,
        remote_chain_info: RemoteChainInfo,
        denom_to_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
    ) -> Self {
        Config {
            input_addr,
            amount,
            denom,
            receiver,
            memo,
            remote_chain_info,
            denom_to_pfm_map,
        }
    }
}
