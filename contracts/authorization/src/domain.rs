use cosmwasm_std::{Binary, DepsMut, Storage, WasmMsg};
use valence_authorization_utils::{
    authorization::{ActionsConfig, Authorization},
    domain::{CallbackProxy, Connector, Domain, ExternalDomain},
};

use crate::{
    error::{AuthorizationErrorReason, ContractError},
    state::{EXTERNAL_DOMAINS, PROCESSOR_ON_MAIN_DOMAIN},
};

/// Checks if external domain exists before adding it
pub fn add_domain(deps: DepsMut, domain: ExternalDomain) -> Result<(), ContractError> {
    if EXTERNAL_DOMAINS.has(deps.storage, domain.name.clone()) {
        return Err(ContractError::ExternalDomainAlreadyExists(domain.name));
    }

    match &domain.connector {
        Connector::PolytoneNote(addr) => deps.api.addr_validate(addr.as_str())?,
    };

    match &domain.callback_proxy {
        CallbackProxy::PolytoneProxy(addr) => deps.api.addr_validate(addr.as_str())?,
    };

    EXTERNAL_DOMAINS.save(deps.storage, domain.name.clone(), &domain)?;

    Ok(())
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

pub fn create_wasm_msg_for_processor_or_proxy(
    storage: &dyn Storage,
    execute_msg: Binary,
    domain: &Domain,
) -> Result<WasmMsg, ContractError> {
    // If the domain is the main domain we will use the processor on the main domain, otherwise we will use polytone
    match domain {
        Domain::Main => {
            let processor = PROCESSOR_ON_MAIN_DOMAIN.load(storage)?;
            Ok(WasmMsg::Execute {
                contract_addr: processor.to_string(),
                msg: execute_msg,
                funds: vec![],
            })
        }
        // TODO: Implement polytone messages + handle callbacks (will come with interchain testing)
        Domain::External(_) => todo!(),
    }
}
