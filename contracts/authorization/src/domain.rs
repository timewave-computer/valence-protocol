use cosmwasm_std::{
    to_json_binary, Addr, Binary, CosmosMsg, DepsMut, HexBinary, Storage, Uint64, WasmMsg,
};
use valence_authorization_utils::{
    authorization::{Authorization, Subroutine},
    domain::{
        CosmwasmBridge, Domain, EvmBridge, ExecutionEnvironment, ExternalDomain, PolytoneNote,
    },
    msg::ExternalDomainInfo,
};
use valence_bridging_utils::{
    hyperlane::{DispatchMsg, HyperlaneExecuteMsg},
    polytone::{CallbackRequest, PolytoneExecuteMsg},
};

use crate::{
    error::{AuthorizationErrorReason, ContractError},
    state::{EXTERNAL_DOMAINS, PROCESSOR_ON_MAIN_DOMAIN},
};

/// Saves a validated external domain if it doesn't already exist and returns it
pub fn add_external_domain(
    deps: DepsMut,
    domain: ExternalDomainInfo,
) -> Result<ExternalDomain, ContractError> {
    let external_domain = domain.to_external_domain_validated(deps.api)?;

    if EXTERNAL_DOMAINS.has(deps.storage, external_domain.name.clone()) {
        return Err(ContractError::ExternalDomainAlreadyExists(
            external_domain.name,
        ));
    }

    EXTERNAL_DOMAINS.save(deps.storage, external_domain.name.clone(), &external_domain)?;

    Ok(external_domain)
}

pub fn get_domain(authorization: &Authorization) -> Result<Domain, ContractError> {
    match &authorization.subroutine {
        Subroutine::Atomic(config) => config
            .functions
            .first()
            .map(|function| function.domain.clone())
            .ok_or(ContractError::Authorization(
                AuthorizationErrorReason::NoFunctions {},
            )),
        Subroutine::NonAtomic(config) => config
            .functions
            .first()
            .map(|function| function.domain.clone())
            .ok_or(ContractError::Authorization(
                AuthorizationErrorReason::NoFunctions {},
            )),
    }
}

pub fn create_msg_for_processor(
    storage: &dyn Storage,
    execute_msg: Binary,
    domain: &Domain,
    callback_request: Option<CallbackRequest>,
) -> Result<CosmosMsg, ContractError> {
    match domain {
        Domain::Main => create_msg_for_main_domain(storage, execute_msg),
        Domain::External(external_domain) => {
            let external_domain = EXTERNAL_DOMAINS.load(storage, external_domain.clone())?;
            match external_domain.execution_environment {
                ExecutionEnvironment::Cosmwasm(cosmwasm_bridge) => match cosmwasm_bridge {
                    CosmwasmBridge::Polytone(polytone_connectors) => create_msg_for_polytone(
                        polytone_connectors.polytone_note,
                        external_domain.processor,
                        execute_msg,
                        callback_request,
                    ),
                },
                ExecutionEnvironment::Evm(_, evm_bridge) => match evm_bridge {
                    EvmBridge::Hyperlane(hyperlane_connector) => create_msg_for_hyperlane(
                        hyperlane_connector.mailbox,
                        hyperlane_connector.domain_id,
                        external_domain.processor,
                        execute_msg,
                    ),
                },
            }
        }
    }
}

pub fn create_msg_for_main_domain(
    storage: &dyn Storage,
    execute_msg: Binary,
) -> Result<CosmosMsg, ContractError> {
    let processor = PROCESSOR_ON_MAIN_DOMAIN.load(storage)?;
    // Simple message for the main domain's processor
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: processor.to_string(),
        msg: execute_msg,
        funds: vec![],
    }))
}

pub fn create_msg_for_polytone(
    polytone_note: PolytoneNote,
    processor: String,
    execute_msg: Binary,
    callback_request: Option<CallbackRequest>,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: polytone_note.address.to_string(),
        msg: to_json_binary(&PolytoneExecuteMsg::Execute {
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: processor,
                msg: execute_msg,
                funds: vec![],
            })],
            callback: callback_request,
            timeout_seconds: Uint64::from(polytone_note.timeout_seconds),
        })?,
        funds: vec![],
    }))
}

pub fn create_msg_for_hyperlane(
    mailbox: Addr,
    domain_id: u32,
    processor: String,
    execute_msg: Binary,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: mailbox.to_string(),
        msg: to_json_binary(&HyperlaneExecuteMsg::Dispatch(DispatchMsg {
            dest_domain: domain_id,
            recipient_addr: format_address_for_hyperlane(processor)?,
            msg_body: HexBinary::from(execute_msg.to_vec()),
            hook: None,
            metadata: None,
        }))?,
        funds: vec![],
    }))
}

/// Formats an address for Hyperlane by removing the "0x" prefix and padding it to 32 bytes
pub fn format_address_for_hyperlane(address: String) -> Result<HexBinary, ContractError> {
    // Remove "0x" prefix if present
    let address_hex = address.trim_start_matches("0x").to_string().to_lowercase();
    // Pad to 32 bytes (64 hex characters) because mailboxes expect 32 bytes addresses with leading zeros
    let padded_address = format!("{:0>64}", address_hex);
    // Convert to HexBinary which is what Hyperlane expects
    Ok(HexBinary::from_hex(&padded_address)?)
}
