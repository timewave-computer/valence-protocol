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
    use cosmos_sdk_proto::{
        cosmos::base::v1beta1::Coin, ibc::applications::transfer::v1::MsgTransfer, prost::Name,
        traits::MessageExt,
    };
    use cosmwasm_std::{AnyMsg, Binary, DepsMut, Env, MessageInfo, Response};
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
                let proto_msg = MsgTransfer {
                    source_port: "transfer".to_string(),
                    source_channel: cfg.remote_chain_info.channel_id,
                    token: Some(Coin {
                        denom: cfg.denom,
                        amount: cfg.amount.to_string(),
                    }),
                    sender: remote_address,
                    receiver: cfg.receiver,
                    timeout_height: None,
                    timeout_timestamp: env
                        .block
                        .time
                        .plus_seconds(
                            cfg.remote_chain_info
                                .ibc_transfer_timeout
                                .unwrap_or(DEFAULT_IBC_TIMEOUT),
                        )
                        .nanos(),
                };

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
                    .add_attribute("method", "cctp_transfer"))
            }
        }
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
