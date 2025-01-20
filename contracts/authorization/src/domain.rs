use cosmwasm_std::{to_json_binary, Binary, CosmosMsg, DepsMut, Storage, Uint64, WasmMsg};
use valence_authorization_utils::{
    authorization::{Authorization, Subroutine},
    callback::PolytoneCallbackMsg,
    domain::{CosmwasmBridge, Domain, ExecutionEnvironment, PolytoneNote},
    msg::ExternalDomainInfo,
};
use valence_bridging_utils::polytone::{CallbackRequest, PolytoneExecuteMsg};

use crate::{
    error::{AuthorizationErrorReason, ContractError},
    state::{EXTERNAL_DOMAINS, PROCESSOR_ON_MAIN_DOMAIN},
};

/// Checks if external domain exists before adding it and creates the message to create the bridge account
pub fn add_domain(
    deps: DepsMut,
    callback_receiver: String,
    domain: ExternalDomainInfo,
) -> Result<Option<CosmosMsg>, ContractError> {
    let external_domain = domain.to_external_domain_validated(deps.api)?;

    if EXTERNAL_DOMAINS.has(deps.storage, external_domain.name.clone()) {
        return Err(ContractError::ExternalDomainAlreadyExists(
            external_domain.name,
        ));
    }

    EXTERNAL_DOMAINS.save(deps.storage, external_domain.name.clone(), &external_domain)?;

    // Create the message to create the bridge account if it's required
    let msg = match external_domain.execution_environment {
        ExecutionEnvironment::Cosmwasm(cosmwasm_bridge) => match cosmwasm_bridge {
            CosmwasmBridge::Polytone(polytone_connectors) => {
                // In polytone to create the proxy we can send an empty vector of messages
                Some(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: polytone_connectors.polytone_note.address.to_string(),
                    msg: to_json_binary(&PolytoneExecuteMsg::Execute {
                        msgs: vec![],
                        callback: Some(CallbackRequest {
                            receiver: callback_receiver,
                            // When we add domain we will return a callback with the name of the domain to know that we are getting the callback when trying to create the proxy
                            msg: to_json_binary(&PolytoneCallbackMsg::CreateProxy(
                                external_domain.name,
                            ))?,
                        }),
                        timeout_seconds: Uint64::from(
                            polytone_connectors.polytone_note.timeout_seconds,
                        ),
                    })?,
                    funds: vec![],
                }))
            }
        },
        ExecutionEnvironment::Evm(_) => None,
    };

    Ok(msg)
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
                ExecutionEnvironment::Evm(_) => todo!(),
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
