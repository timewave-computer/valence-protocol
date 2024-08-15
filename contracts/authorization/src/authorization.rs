use authorization_utils::{
    authorization::{Authorization, AuthorizationMode, ExecutionType, Priority},
    domain::Domain,
};
use cosmwasm_std::Storage;

use crate::{error::ContractError, state::EXTERNAL_DOMAINS};

pub trait Validate {
    fn validate(&self, store: &dyn Storage) -> Result<(), ContractError>;
}

impl Validate for Authorization {
    fn validate(&self, store: &dyn Storage) -> Result<(), ContractError> {
        // Label can't be empty or already exist
        if self.label.is_empty() {
            return Err(ContractError::EmptyLabel {});
        }

        // An authorization must have actions
        let first_action = match self.action_batch.actions.first() {
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
        for each_action in self.action_batch.actions.iter() {
            if !each_action.domain.eq(&first_action.domain) {
                return Err(ContractError::DifferentActionDomains {});
            }
        }

        // If an authorization is permissionless, it can't have high priority
        if self.mode.eq(&AuthorizationMode::Permissionless)
            && self.priority.clone().eq(&Priority::High)
        {
            return Err(ContractError::PermissionlessAuthorizationWithHighPriority {});
        }

        // An authorization can't have actions with callback confirmations if needs to be executed atomically
        if self.action_batch.execution_type.eq(&ExecutionType::Atomic) {
            for each_action in self.action_batch.actions.iter() {
                if each_action.callback_confirmation.is_some() {
                    return Err(ContractError::AtomicAuthorizationWithCallbackConfirmation {});
                }
            }
        }

        Ok(())
    }
}
