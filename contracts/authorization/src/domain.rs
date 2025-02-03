use cosmwasm_std::{Binary, CosmosMsg, DepsMut, StdResult, Storage, Uint64, WasmMsg};
use valence_authorization_utils::{
    authorization::{Authorization, Subroutine},
    domain::{CosmwasmBridge, Domain, EvmBridge, ExecutionEnvironment, ExternalDomain},
    msg::ExternalDomainInfo,
};
use valence_bridging_utils::{
    hyperlane::create_msg_for_hyperlane,
    polytone::{create_msg_for_polytone, CallbackRequest},
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
) -> StdResult<CosmosMsg> {
    match domain {
        Domain::Main => create_msg_for_main_domain(storage, execute_msg),
        Domain::External(external_domain) => {
            let external_domain = EXTERNAL_DOMAINS.load(storage, external_domain.clone())?;
            match external_domain.execution_environment {
                ExecutionEnvironment::Cosmwasm(cosmwasm_bridge) => match cosmwasm_bridge {
                    CosmwasmBridge::Polytone(polytone_connectors) => create_msg_for_polytone(
                        polytone_connectors.polytone_note.address.to_string(),
                        Uint64::from(polytone_connectors.polytone_note.timeout_seconds),
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
) -> StdResult<CosmosMsg> {
    let processor = PROCESSOR_ON_MAIN_DOMAIN.load(storage)?;
    // Simple message for the main domain's processor
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: processor.to_string(),
        msg: execute_msg,
        funds: vec![],
    }))
}
