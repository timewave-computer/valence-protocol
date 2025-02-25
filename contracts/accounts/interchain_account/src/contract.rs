#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order, Response, StdError,
    StdResult,
};
use cw2::set_contract_version;
use neutron_sdk::{bindings::query::NeutronQuery, sudo::msg::SudoMsg};

use valence_account_utils::{error::ContractError, ica::OpenAckVersion};

use crate::{
    msg::{ExecuteMsg, IcaInformation, IcaState, InstantiateMsg, QueryMsg},
    state::{APPROVED_LIBRARIES, ICA_STATE, REMOTE_DOMAIN_INFO},
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const INTERCHAIN_ACCOUNT_ID: &str = "valence-ica";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.admin))?;

    msg.approved_libraries.iter().try_for_each(|library| {
        APPROVED_LIBRARIES.save(deps.storage, deps.api.addr_validate(library)?, &Empty {})
    })?;

    REMOTE_DOMAIN_INFO.save(deps.storage, &msg.remote_domain_information)?;
    ICA_STATE.save(deps.storage, &IcaState::NotCreated)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ApproveLibrary { library } => execute::approve_library(deps, info, library),
        ExecuteMsg::RemoveLibrary { library } => execute::remove_library(deps, info, library),
        ExecuteMsg::ExecuteIcaMsg { msgs } => execute::execute_ica_msg(deps, env, info, msgs),
        ExecuteMsg::RegisterIca {} => execute::try_register_ica(deps, env),
        ExecuteMsg::UpdateOwnership(action) => execute::update_ownership(deps, env, info, action),
    }
}

mod execute {
    use cosmwasm_std::{ensure, DepsMut, Empty, Env, MessageInfo, Response, StdError};
    use neutron_sdk::{
        bindings::{query::NeutronQuery, types::ProtobufAny},
        query::min_ibc_fee::query_min_ibc_fee,
    };
    use valence_account_utils::{
        error::{ContractError, UnauthorizedReason},
        ica::{query_ica_registration_fee, register_ica_msg, submit_tx},
    };
    use valence_ibc_utils::neutron::{get_transfer_fee, min_ntrn_ibc_fee};

    use crate::{
        msg::IcaState,
        state::{APPROVED_LIBRARIES, ICA_STATE, REMOTE_DOMAIN_INFO},
    };

    use super::INTERCHAIN_ACCOUNT_ID;

    pub fn approve_library(
        deps: DepsMut<NeutronQuery>,
        info: MessageInfo,
        library: String,
    ) -> Result<Response, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        let library_addr = deps.api.addr_validate(&library)?;
        APPROVED_LIBRARIES.save(deps.storage, library_addr.clone(), &Empty {})?;

        Ok(Response::new()
            .add_attribute("method", "approve_library")
            .add_attribute("library", library_addr))
    }

    pub fn remove_library(
        deps: DepsMut<NeutronQuery>,
        info: MessageInfo,
        library: String,
    ) -> Result<Response, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        let library_addr = deps.api.addr_validate(&library)?;
        APPROVED_LIBRARIES.remove(deps.storage, library_addr.clone());

        Ok(Response::new()
            .add_attribute("method", "remove_library")
            .add_attribute("library", library_addr))
    }

    pub fn try_register_ica(
        deps: DepsMut<NeutronQuery>,
        env: Env,
    ) -> Result<Response, ContractError> {
        // First we verify that we are in the correct state to allow ICA creation
        let state = ICA_STATE.load(deps.storage)?;
        if state.ne(&IcaState::NotCreated) || state.ne(&IcaState::Closed) {
            return Err(ContractError::InvalidIcaState {
                current_state: state.to_string(),
            });
        }

        let remote_domain_info = REMOTE_DOMAIN_INFO.load(deps.storage)?;
        let ica_registration_fee = query_ica_registration_fee(deps.querier)?;

        // Check that we have enough to cover the registration fee
        ensure!(
            ica_registration_fee.iter().all(|coin| {
                deps.querier
                    .query_balance(&env.contract.address, &coin.denom)
                    .unwrap_or_default()
                    .amount
                    >= coin.amount
            }),
            ContractError::NotEnoughBalanceForIcaRegistration
        );

        let register_ica_msg = register_ica_msg(
            env.contract.address.into_string(),
            remote_domain_info.connection_id,
            INTERCHAIN_ACCOUNT_ID.to_string(),
            ica_registration_fee,
        );

        // Update the state to InProgress
        ICA_STATE.save(deps.storage, &IcaState::InProgress)?;

        Ok(Response::new()
            .add_message(register_ica_msg)
            .add_attribute("method", "register_ica"))
    }

    pub fn execute_ica_msg(
        deps: DepsMut<NeutronQuery>,
        env: Env,
        info: MessageInfo,
        msgs: Vec<ProtobufAny>,
    ) -> Result<Response, ContractError> {
        // If not admin, check if it's an approved library
        ensure!(
            cw_ownable::is_owner(deps.storage, &info.sender)?
                || APPROVED_LIBRARIES.has(deps.storage, info.sender.clone()),
            ContractError::Unauthorized(UnauthorizedReason::NotAdminOrApprovedLibrary)
        );

        // Get the Remote Chain Information
        let remote_domain_info = REMOTE_DOMAIN_INFO.load(deps.storage)?;

        // Get the IBC fee
        let ibc_fee = min_ntrn_ibc_fee(
            query_min_ibc_fee(deps.as_ref())
                .map_err(|err| StdError::generic_err(err.to_string()))?
                .min_fee,
        );

        // Get the proto fee
        let transfer_fee = get_transfer_fee(ibc_fee);

        // Create the SubmitTx msg
        let submit_tx = submit_tx(
            env.contract.address.into_string(),
            remote_domain_info.connection_id,
            INTERCHAIN_ACCOUNT_ID.to_string(),
            msgs,
            "".to_string(),
            remote_domain_info.ica_timeout.u64(),
            transfer_fee,
        );

        // Send the message
        Ok(Response::new()
            .add_message(submit_tx)
            .add_attribute("method", "execute_ica_msg")
            .add_attribute("sender", info.sender))
    }

    pub fn update_ownership(
        deps: DepsMut<NeutronQuery>,
        env: Env,
        info: MessageInfo,
        action: cw_ownable::Action,
    ) -> Result<Response, ContractError> {
        let result = cw_ownable::update_ownership(
            deps.into_empty(),
            &env.block,
            &info.sender,
            action.clone(),
        )?;
        Ok(Response::default()
            .add_attribute("method", "update_ownership")
            .add_attribute("action", format!("{:?}", action))
            .add_attribute("result", format!("{:?}", result)))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<NeutronQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::ListApprovedLibraries {} => {
            let libraries = APPROVED_LIBRARIES
                .keys(deps.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?;
            to_json_binary(&libraries)
        }
        QueryMsg::IcaState {} => {
            let state = ICA_STATE.load(deps.storage)?;
            to_json_binary(&state)
        }
        QueryMsg::RemoteDomainInfo {} => {
            let remote_domain_info = REMOTE_DOMAIN_INFO.load(deps.storage)?;
            to_json_binary(&remote_domain_info)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> StdResult<Response> {
    match msg {
        SudoMsg::Response { request, data } => {
            // If the channel closed, we need to update the state to Closed to allow recreation
            if request.sequence.is_none() || request.source_channel.is_none() {
                ICA_STATE.save(deps.storage, &IcaState::Closed)?;
            }

            Ok(Response::new()
                .add_attribute("method", "sudo_response")
                .add_attribute("data", data.to_string()))
        }

        SudoMsg::Error { request, details } => {
            // If the channel closed, we need to update the state to Closed to allow recreation
            if request.sequence.is_none() || request.source_channel.is_none() {
                ICA_STATE.save(deps.storage, &IcaState::Closed)?;
            }

            Ok(Response::new()
                .add_attribute("method", "sudo_error")
                .add_attribute("details", details))
        }

        // If it times out means the channel is closed
        SudoMsg::Timeout { .. } => {
            ICA_STATE.save(deps.storage, &IcaState::Closed)?;

            Ok(Response::new().add_attribute("method", "sudo_timeout"))
        }

        // For handling successful registering of ICA
        SudoMsg::OpenAck {
            port_id,
            counterparty_version,
            ..
        } => {
            // We need to parse the json we get from the counterparty version to extract the necessary information
            let parsed_version: OpenAckVersion =
                serde_json::from_str(counterparty_version.as_str())
                    .map_err(|_| StdError::generic_err("Failed to parse counterparty version"))?;

            ICA_STATE.save(
                deps.storage,
                &IcaState::Created(IcaInformation {
                    address: parsed_version.address,
                    port_id,
                    controller_connection_id: parsed_version.controller_connection_id,
                }),
            )?;

            Ok(Response::new().add_attribute("method", "sudo_open_ack"))
        }

        _ => Ok(Response::default()),
    }
}
