use cosmwasm_std::{Binary, Deps, MessageInfo, StdError, Storage, Uint128};
use cw_utils::must_pay;
use serde_json::Value;
use valence_authorization_utils::{
    authorization::{Authorization, AuthorizationMode, ExecutionType, PermissionType, Priority},
    domain::{Domain, ExecutionEnvironment},
    message::ParamRestriction,
};

use crate::{contract::build_tokenfactory_denom, error::ContractError, state::EXTERNAL_DOMAINS};

pub trait Validate {
    fn validate(&self, store: &dyn Storage) -> Result<(), ContractError>;
    fn validate_permission(
        &self,
        deps: Deps,
        contract_address: &str,
        info: &MessageInfo,
    ) -> Result<(), ContractError>;
    fn validate_messages(&self, deps: Deps, messages: &[Binary]) -> Result<(), ContractError>;
}

impl Validate for Authorization {
    fn validate(&self, store: &dyn Storage) -> Result<(), ContractError> {
        // Label can't be empty or already exist
        if self.label.is_empty() {
            return Err(ContractError::EmptyLabel {});
        }

        let mut actions_iter = self.action_batch.actions.iter();
        // An authorization must have actions
        let first_action = match actions_iter.next() {
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
        for each_action in actions_iter {
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

    fn validate_permission(
        &self,
        deps: Deps,
        contract_address: &str,
        info: &MessageInfo,
    ) -> Result<(), ContractError> {
        let token_factory_denom = build_tokenfactory_denom(contract_address, &self.label);
        match self.mode {
            // If the authorization is permissionless, it's always valid
            AuthorizationMode::Permissionless => (),
            // If the authorization is permissioned without call limit, we check that the sender has the token corresponding to that authorization in his wallet
            AuthorizationMode::Permissioned(PermissionType::WithoutCallLimit(_)) => {
                let balance = deps
                    .querier
                    .query_balance(info.sender.clone(), token_factory_denom)?;
                if balance.amount.is_zero() {
                    return Err(ContractError::Unauthorized {});
                }
            }
            AuthorizationMode::Permissioned(PermissionType::WithCallLimit(_)) => {
                let funds = must_pay(info, &token_factory_denom)
                    .map_err(|_| ContractError::AuthorizationRequiresOneToken {})?;

                if funds.ne(&Uint128::one()) {
                    return Err(ContractError::AuthorizationRequiresOneToken {});
                }
            }
        }
        Ok(())
    }

    fn validate_messages(&self, deps: Deps, messages: &[Binary]) -> Result<(), ContractError> {
        if messages.len() != self.action_batch.actions.len() {
            return Err(ContractError::MessagesDoNotMatchActions {});
        }

        for (each_message, each_action) in messages.iter().zip(self.action_batch.actions.iter()) {
            let execution_environment = match &each_action.domain {
                Domain::Main => ExecutionEnvironment::CosmWasm,
                Domain::External(name) => {
                    let domain = EXTERNAL_DOMAINS.load(deps.storage, name.clone())?;
                    domain.execution_environment
                }
            };

            match &execution_environment {
                ExecutionEnvironment::CosmWasm => {
                    // Extract the json from each message
                    let json: Value = serde_json::from_slice(each_message.as_slice())
                        .map_err(|e| ContractError::Std(StdError::generic_err(e.to_string())))?;

                    // Check if the message matches the action
                    if json
                        .get(each_action.message_details.message.name.as_str())
                        .is_none()
                    {
                        return Err(ContractError::InvalidMessage {});
                    }

                    // Check that all the Parameter restrictions are met
                    if let Some(param_restrictions) =
                        &each_action.message_details.message.params_restrictions
                    {
                        for each_restriction in param_restrictions {
                            check_restriction(&json, each_restriction)?;
                        }
                    }

                    // TODO: Create the Processor/Polytone Message
                }
            }
        }

        // TODO: Return the list of messages to be sent to the processor/connector

        Ok(())
    }
}

fn check_restriction(
    json: &Value,
    param_restriction: &ParamRestriction,
) -> Result<(), ContractError> {
    match param_restriction {
        ParamRestriction::MustBeIncluded(keys) => {
            let mut current_value = json;

            for key in keys {
                current_value = current_value
                    .get(key)
                    .ok_or(ContractError::InvalidMessageParams {})?;
            }
        }
        ParamRestriction::CannotBeIncluded(keys) => {
            let mut current_value = json;

            for key in keys.iter().take(keys.len() - 1) {
                current_value = match current_value.get(key) {
                    Some(value) => value,
                    None => return Ok(()), // If part of the path doesn't exist, it's valid
                };
            }

            // Check the final key in the path
            if let Some(final_key) = keys.last() {
                if current_value.get(final_key).is_some() {
                    return Err(ContractError::InvalidMessageParams {});
                }
            }
        }
        ParamRestriction::MustBeValue(keys, value) => {
            let mut current_value = json;

            for key in keys {
                current_value = current_value
                    .get(key)
                    .ok_or(ContractError::InvalidMessageParams {})?;
            }

            // Deserialize the expected value for a more robust comparison
            let expected: Value = serde_json::from_slice(value)
                .map_err(|_| ContractError::InvalidMessageParams {})?;

            if *current_value != expected {
                return Err(ContractError::InvalidMessageParams {});
            }
        }
    }
    Ok(())
}
