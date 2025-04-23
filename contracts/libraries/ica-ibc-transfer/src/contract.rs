#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_library_utils::{
    error::LibraryError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_IBC_TIMEOUT: u64 = 600; // 10 minutes

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<LibraryConfig>,
) -> Result<Response, LibraryError> {
    valence_library_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<FunctionMsgs, LibraryConfigUpdate>,
) -> Result<Response, LibraryError> {
    valence_library_base::execute(
        deps,
        env,
        info,
        msg,
        functions::process_function,
        execute::update_config,
    )
}

mod functions {
    use std::collections::BTreeMap;

    use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin, prost::Name, traits::MessageExt};
    use cosmwasm_std::{
        to_json_string, AnyMsg, Binary, DepsMut, Env, MessageInfo, Response, StdResult,
    };
    use ibc_proto::ibc::apps::transfer::v1::MsgTransfer;
    use valence_ibc_utils::types::{
        ForwardMetadata, PacketForwardMiddlewareConfig, PacketMetadata,
    };
    use valence_library_utils::{
        error::LibraryError,
        ica::{execute_on_behalf_of, get_remote_ica_address},
    };

    use crate::msg::{Config, FunctionMsgs};

    use super::DEFAULT_IBC_TIMEOUT;

    pub fn process_function(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        msg: FunctionMsgs,
        cfg: Config,
    ) -> Result<Response, LibraryError> {
        let remote_address = get_remote_ica_address(deps.as_ref(), cfg.input_addr.as_str())?;

        match msg {
            FunctionMsgs::Transfer {} => {
                // Create the proto message
                let proto_msg = create_msg_transfer(
                    &env,
                    cfg.remote_chain_info.channel_id,
                    cfg.receiver.clone(),
                    cfg.denom.clone(),
                    cfg.amount,
                    cfg.memo.clone(),
                    cfg.remote_chain_info.ibc_transfer_timeout,
                    &cfg.denom_to_pfm_map,
                    remote_address,
                )?;

                // Create the Any
                let any_msg = AnyMsg {
                    type_url: MsgTransfer::type_url(),
                    value: Binary::from(proto_msg.to_bytes().map_err(|e| {
                        LibraryError::ExecutionError(format!("Failed to encode MsgTransfer: {}", e))
                    })?),
                };

                let input_account_msgs = execute_on_behalf_of(vec![any_msg], &cfg.input_addr)?;

                Ok(Response::new()
                    .add_message(input_account_msgs)
                    .add_attribute("method", "ica_ibc_transfer"))
            }
            FunctionMsgs::EurekaTransfer { eureka_fee } => {
                let eureka_config = match cfg.eureka_config {
                    Some(config) => config,
                    None => {
                        return Err(LibraryError::ExecutionError(
                            "No Eureka config provided.".to_string(),
                        ))
                    }
                };

                let eureka_memo = valence_ibc_utils::generic::build_eureka_memo(
                    &env,
                    cfg.receiver.clone(),
                    eureka_fee,
                    eureka_config.clone(),
                )?;

                // Create the proto message
                let proto_msg = create_msg_transfer(
                    &env,
                    cfg.remote_chain_info.channel_id,
                    eureka_config.callback_contract,
                    cfg.denom.clone(),
                    cfg.amount,
                    eureka_memo,
                    cfg.remote_chain_info.ibc_transfer_timeout,
                    &BTreeMap::default(),
                    remote_address,
                )?;

                // Create the Any
                let any_msg = AnyMsg {
                    type_url: MsgTransfer::type_url(),
                    value: Binary::from(proto_msg.to_bytes().map_err(|e| {
                        LibraryError::ExecutionError(format!("Failed to encode MsgTransfer: {}", e))
                    })?),
                };

                let input_account_msgs = execute_on_behalf_of(vec![any_msg], &cfg.input_addr)?;

                Ok(Response::new()
                    .add_message(input_account_msgs)
                    .add_attribute("method", "ica_eureka_ibc_transfer"))
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_msg_transfer(
        env: &Env,
        source_channel: String,
        receiver: String,
        denom: String,
        amount: impl ToString,
        memo: String,
        timeout_seconds: Option<u64>,
        denom_to_pfm_map: &BTreeMap<String, PacketForwardMiddlewareConfig>,
        sender: String,
    ) -> StdResult<MsgTransfer> {
        let timeout_timestamp = env
            .block
            .time
            .plus_seconds(timeout_seconds.unwrap_or(DEFAULT_IBC_TIMEOUT))
            .nanos();

        let msg = match denom_to_pfm_map.get(&denom) {
            // No PFM config for the denom sent
            None => MsgTransfer {
                source_port: "transfer".to_string(),
                source_channel,
                token: Some(Coin {
                    denom,
                    amount: amount.to_string(),
                }),
                sender,
                receiver,
                timeout_height: None,
                timeout_timestamp,
                memo,
            },
            // PFM Config found
            Some(pfm_config) => {
                // Create the packet metadata for forwarding
                let pfm_memo = to_json_string(&PacketMetadata {
                    forward: Some(ForwardMetadata {
                        receiver: receiver.clone(),
                        port: "transfer".to_string(),
                        // hop chain to final receiver chain channel
                        channel: pfm_config.hop_to_destination_chain_channel_id.to_string(),
                    }),
                })?;

                MsgTransfer {
                    source_port: "transfer".to_string(),
                    source_channel: pfm_config.local_to_hop_chain_channel_id.to_string(),
                    token: Some(Coin {
                        denom,
                        amount: amount.to_string(),
                    }),
                    sender,
                    receiver: pfm_config
                        .hop_chain_receiver_address
                        .clone()
                        .unwrap_or("pfm".to_string()),
                    timeout_height: None,
                    timeout_timestamp,
                    memo: pfm_memo,
                }
            }
        };

        Ok(msg)
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use valence_library_utils::error::LibraryError;

    use crate::msg::LibraryConfigUpdate;

    pub fn update_config(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        new_config: LibraryConfigUpdate,
    ) -> Result<(), LibraryError> {
        new_config.update_config(deps)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => {
            to_json_binary(&valence_library_base::get_ownership(deps.storage)?)
        }
        QueryMsg::GetProcessor {} => {
            to_json_binary(&valence_library_base::get_processor(deps.storage)?)
        }
        QueryMsg::GetLibraryConfig {} => {
            let config: Config = valence_library_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
        QueryMsg::GetRawLibraryConfig {} => {
            let raw_config: LibraryConfig =
                valence_library_utils::raw_config::query_raw_library_config(deps.storage)?;
            to_json_binary(&raw_config)
        }
    }
}
