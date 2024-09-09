use cosmwasm_std::{to_json_binary, Binary, CosmosMsg, DepsMut, Storage, Uint64, WasmMsg};
use valence_authorization_utils::{
    authorization::{ActionsConfig, Authorization},
    domain::{Connector, Domain},
    msg::ExternalDomainApi,
    polytone::{CallbackRequest, PolytoneExecuteMsg},
};

use crate::{
    error::{AuthorizationErrorReason, ContractError},
    state::{EXTERNAL_DOMAINS, PROCESSOR_ON_MAIN_DOMAIN},
};

/// Checks if external domain exists before adding it and creates the message to create the bridge account
pub fn add_domain(
    deps: DepsMut,
    callback_receiver: String,
    domain: &ExternalDomainApi,
) -> Result<CosmosMsg, ContractError> {
    let external_domain = domain.to_external_domain_validated(deps.api)?;

    if EXTERNAL_DOMAINS.has(deps.storage, external_domain.name.clone()) {
        return Err(ContractError::ExternalDomainAlreadyExists(
            external_domain.name,
        ));
    }

    EXTERNAL_DOMAINS.save(deps.storage, external_domain.name.clone(), &external_domain)?;

    // Create the message to create the bridge account
    let msg = match external_domain.connector {
        // We will send an empty message just for the sake of creating the proxy account.
        Connector::PolytoneNote {
            address,
            timeout_seconds,
            ..
        } => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: address.to_string(),
            msg: to_json_binary(&PolytoneExecuteMsg::Execute {
                msgs: vec![],
                callback: Some(CallbackRequest {
                    receiver: callback_receiver,
                    // When we add domain we will return a callback with the name of the domain to know that we are getting the callback when trying to create the proxy
                    msg: to_json_binary(&external_domain.name)?,
                }),
                timeout_seconds: Uint64::from(timeout_seconds),
            })?,
            funds: vec![],
        }),
    };

    Ok(msg)
}

pub fn get_domain(authorization: &Authorization) -> Result<Domain, ContractError> {
    match &authorization.actions_config {
        ActionsConfig::Atomic(config) => config
            .actions
            .first()
            .map(|action| action.domain.clone())
            .ok_or(ContractError::Authorization(
                AuthorizationErrorReason::NoActions {},
            )),
        ActionsConfig::NonAtomic(config) => config
            .actions
            .first()
            .map(|action| action.domain.clone())
            .ok_or(ContractError::Authorization(
                AuthorizationErrorReason::NoActions {},
            )),
    }
}

pub fn create_wasm_msg_for_main_processor_or_bridge(
    storage: &dyn Storage,
    execute_msg: Binary,
    domain: &Domain,
    callback_request: Option<CallbackRequest>,
) -> Result<WasmMsg, ContractError> {
    // If the domain is the main domain we will use the processor on the main domain, otherwise we will use polytone to send it to the processor on the external domain
    match domain {
        Domain::Main => {
            let processor = PROCESSOR_ON_MAIN_DOMAIN.load(storage)?;
            // Simple message for the main domain's processor
            Ok(WasmMsg::Execute {
                contract_addr: processor.to_string(),
                msg: execute_msg,
                funds: vec![],
            })
        }
        Domain::External(name) => {
            let external_domain = EXTERNAL_DOMAINS.load(storage, name.clone())?;
            match external_domain.connector {
                // If it has to go through polytone, we will create the message for polytone instead
                Connector::PolytoneNote {
                    address,
                    timeout_seconds,
                    ..
                } => Ok(WasmMsg::Execute {
                    contract_addr: address.to_string(),
                    msg: to_json_binary(&PolytoneExecuteMsg::Execute {
                        msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: external_domain.processor,
                            msg: execute_msg,
                            funds: vec![],
                        })],
                        callback: callback_request,
                        timeout_seconds: Uint64::from(timeout_seconds),
                    })?,
                    funds: vec![],
                }),
            }
        }
    }
}
