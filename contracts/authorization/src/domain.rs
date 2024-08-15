use authorization_utils::domain::{CallbackProxy, Connector, ExternalDomain};
use cosmwasm_std::DepsMut;

use crate::{error::ContractError, state::EXTERNAL_DOMAINS};

/// Checks if external domain exists before adding it
pub fn add_domains(deps: DepsMut, domains: Vec<ExternalDomain>) -> Result<(), ContractError> {
    for domain in domains {
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
    }
    Ok(())
}
