use authorization_utils::{authorization::{
    AuthorizationInfo, AuthorizationMode, ExecutionType, Priority,
}, domain::Domain};
use cosmwasm_std::Storage;

use crate::{
    error::ContractError,
    state::{AUTHORIZATIONS, EXTERNAL_DOMAINS},
};

/// Will perform all the necessary checks to validate an authorization
pub fn validate_authorization(
    store: &dyn Storage,
    authorization: &AuthorizationInfo,
) -> Result<(), ContractError> {
    // Label can't be empty or already exist
    if authorization.label.is_empty() {
        return Err(ContractError::EmptyLabel {});
    }
    if AUTHORIZATIONS.has(store, authorization.label.clone()) {
        return Err(ContractError::LabelAlreadyExists(
            authorization.label.clone(),
        ));
    }

    // An authorization must have actions
    let first_action = match authorization.action_batch.actions.first() {
        None => return Err(ContractError::NoActions {}),
        Some(action) => action,
    };

    // The domain of the actions must be registered
    match &first_action.domain {
        Domain::Main => (),
        Domain::External(domain_name) => {
            if !EXTERNAL_DOMAINS.has(store, domain_name.clone()) {
                return Err(ContractError::DomainIsNotRegistered(domain_name.clone()));
            }
        }
    }

    // All actions in an authorization must be executed in the same domain (for v1)
    for each_action in authorization.action_batch.actions.iter() {
        if !each_action.domain.eq(&first_action.domain) {
            return Err(ContractError::DifferentActionDomains {});
        }
    }

    // If an authorization is permissionless, it can't have high priority
    if authorization.mode.eq(&AuthorizationMode::Permissionless)
        && authorization
            .priority
            .clone()
            .unwrap_or_default()
            .eq(&Priority::High)
    {
        return Err(ContractError::PermissionlessAuthorizationWithHighPriority {});
    }

    // An authorization can't have actions with callback confirmations if needs to be executed atomically
    if authorization
        .action_batch
        .execution_type
        .eq(&ExecutionType::Atomic)
    {
        for each_action in authorization.action_batch.actions.iter() {
            if each_action.callback_confirmation.is_some() {
                return Err(ContractError::AtomicAuthorizationWithCallbackConfirmation {});
            }
        }
    }

    Ok(())
}
