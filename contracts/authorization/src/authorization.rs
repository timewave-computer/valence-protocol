use cosmwasm_std::{Binary, BlockInfo, MessageInfo, QuerierWrapper, Storage, Uint128};
use cw_utils::{must_pay, Expiration};
use serde_json::Value;
use valence_authorization_utils::{
    authorization::{
        Authorization, AuthorizationMode, AuthorizationState, ExecutionType, PermissionType,
        Priority,
    },
    domain::{Domain, ExecutionEnvironment},
    message::ParamRestriction,
};

use crate::{
    contract::build_tokenfactory_denom,
    error::{AuthorizationErrorReason, ContractError, MessageErrorReason, UnauthorizedReason},
    state::EXTERNAL_DOMAINS,
};

pub trait Validate {
    fn validate(&self, store: &dyn Storage) -> Result<(), ContractError>;
    fn ensure_enabled(&self) -> Result<(), ContractError>;
    fn ensure_active(&self, block: &BlockInfo) -> Result<(), ContractError>;
    fn ensure_not_expired(&self, block: &BlockInfo) -> Result<(), ContractError>;
    fn validate_permission(
        &self,
        querier: QuerierWrapper,
        contract_address: &str,
        info: &MessageInfo,
    ) -> Result<(), ContractError>;
    fn validate_messages(
        &self,
        store: &dyn Storage,
        messages: &[Binary],
    ) -> Result<(), ContractError>;
    fn validate_executable(
        &self,
        store: &dyn Storage,
        block: &BlockInfo,
        querier: QuerierWrapper,
        contract_address: &str,
        info: &MessageInfo,
        messages: &[Binary],
    ) -> Result<(), ContractError>;
}

impl Validate for Authorization {
    fn validate(&self, store: &dyn Storage) -> Result<(), ContractError> {
        // Label can't be empty or already exist
        if self.label.is_empty() {
            return Err(ContractError::Authorization(
                AuthorizationErrorReason::EmptyLabel {},
            ));
        }

        let mut actions_iter = self.action_batch.actions.iter();
        // An authorization must have actions
        let first_action = match actions_iter.next() {
            None => {
                return Err(ContractError::Authorization(
                    AuthorizationErrorReason::NoActions {},
                ))
            }
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
                return Err(ContractError::Authorization(
                    AuthorizationErrorReason::DifferentActionDomains {},
                ));
            }
        }

        // If an authorization is permissionless, it can't have high priority
        if self.mode.eq(&AuthorizationMode::Permissionless)
            && self.priority.clone().eq(&Priority::High)
        {
            return Err(ContractError::Authorization(
                AuthorizationErrorReason::PermissionlessWithHighPriority {},
            ));
        }

        // An authorization can't have actions with callback confirmations if needs to be executed atomically
        if self.action_batch.execution_type.eq(&ExecutionType::Atomic) {
            for each_action in self.action_batch.actions.iter() {
                if each_action.callback_confirmation.is_some() {
                    return Err(ContractError::Authorization(
                        AuthorizationErrorReason::AtomicWithCallbackConfirmation {},
                    ));
                }
            }
        }

        Ok(())
    }

    fn ensure_enabled(&self) -> Result<(), ContractError> {
        if self.state.ne(&AuthorizationState::Enabled) {
            return Err(ContractError::Unauthorized(
                UnauthorizedReason::NotEnabled {},
            ));
        }
        Ok(())
    }

    fn ensure_active(&self, block: &BlockInfo) -> Result<(), ContractError> {
        match self.not_before {
            Expiration::Never {} => Ok(()),
            expiration => {
                if !expiration.is_expired(block) {
                    Err(ContractError::Unauthorized(
                        UnauthorizedReason::NotActiveYet {},
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }

    fn ensure_not_expired(&self, block: &BlockInfo) -> Result<(), ContractError> {
        if self.expiration.is_expired(block) {
            return Err(ContractError::Unauthorized(UnauthorizedReason::Expired {}));
        }

        Ok(())
    }

    fn validate_permission(
        &self,
        querier: QuerierWrapper,
        contract_address: &str,
        info: &MessageInfo,
    ) -> Result<(), ContractError> {
        let token_factory_denom = build_tokenfactory_denom(contract_address, &self.label);
        match self.mode {
            // If the authorization is permissionless, it's always valid
            AuthorizationMode::Permissionless => (),
            // If the authorization is permissioned without call limit, we check that the sender has the token corresponding to that authorization in his wallet
            AuthorizationMode::Permissioned(PermissionType::WithoutCallLimit(_)) => {
                let balance = querier.query_balance(info.sender.clone(), token_factory_denom)?;
                if balance.amount.is_zero() {
                    return Err(ContractError::Unauthorized(
                        UnauthorizedReason::NotAllowed {},
                    ));
                }
            }
            // If the authorization is permissioned with call limit, the sender must pay one token to execute the authorization, which will be burned
            // if it executes (or partially executes) and will be refunded if it doesn't.
            AuthorizationMode::Permissioned(PermissionType::WithCallLimit(_)) => {
                let funds = must_pay(info, &token_factory_denom)
                    .map_err(|_| ContractError::Unauthorized(UnauthorizedReason::NotAllowed {}))?;

                if funds.ne(&Uint128::one()) {
                    return Err(ContractError::Unauthorized(
                        UnauthorizedReason::RequiresOneToken {},
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_messages(
        &self,
        store: &dyn Storage,
        messages: &[Binary],
    ) -> Result<(), ContractError> {
        if messages.len() != self.action_batch.actions.len() {
            return Err(ContractError::Message(MessageErrorReason::InvalidAmount {}));
        }

        for (each_message, each_action) in messages.iter().zip(self.action_batch.actions.iter()) {
            let execution_environment = match &each_action.domain {
                Domain::Main => ExecutionEnvironment::CosmWasm,
                Domain::External(name) => {
                    let domain = EXTERNAL_DOMAINS.load(store, name.clone())?;
                    domain.execution_environment
                }
            };

            match &execution_environment {
                ExecutionEnvironment::CosmWasm => {
                    // Extract the json from each message
                    let json: Value =
                        serde_json::from_slice(each_message.as_slice()).map_err(|e| {
                            ContractError::InvalidJson {
                                error: e.to_string(),
                            }
                        })?;

                    // Check that json only has one top level key
                    if json.as_object().map(|obj| obj.len()).unwrap_or(0) != 1 {
                        return Err(ContractError::Message(
                            MessageErrorReason::InvalidStructure {},
                        ));
                    }

                    // Check that top level key matches the message name
                    if json
                        .get(each_action.message_details.message.name.as_str())
                        .is_none()
                    {
                        return Err(ContractError::Message(MessageErrorReason::DoesNotMatch {}));
                    }

                    // Check that all the Parameter restrictions are met
                    if let Some(param_restrictions) =
                        &each_action.message_details.message.params_restrictions
                    {
                        for each_restriction in param_restrictions {
                            check_restriction(&json, each_restriction)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_executable(
        &self,
        store: &dyn Storage,
        block: &BlockInfo,
        querier: QuerierWrapper,
        contract_address: &str,
        info: &MessageInfo,
        messages: &[Binary],
    ) -> Result<(), ContractError> {
        self.ensure_enabled()?;
        self.ensure_active(block)?;
        self.ensure_not_expired(block)?;
        self.validate_permission(querier, contract_address, info)?;
        self.validate_messages(store, messages)?;

        Ok(())
    }
}

fn check_restriction(
    json: &Value,
    param_restriction: &ParamRestriction,
) -> Result<(), ContractError> {
    // Looks up a value by a JSON Pointer and returns a mutable reference to
    // that value.
    //
    // JSON Pointer defines a string syntax for identifying a specific value
    // within a JSON.
    //
    // A Pointer is a Unicode string with the reference tokens separated by `/`.
    // The addressed value is returned and if there is no such value `None` is
    // returned.
    // Example:
    // let data = json!({
    //     "x": {
    //         "y": ["z", "zz"]
    //     }
    // });
    //
    // assert_eq!(data.pointer("/x/y/1").unwrap(), &json!("zz"));
    // assert_eq!(data.pointer("/a/b/c"), None);
    let pointer = |keys: &[String]| -> String { keys.join("/") };

    match param_restriction {
        ParamRestriction::MustBeIncluded(keys) => {
            json.pointer(&pointer(keys)).ok_or(ContractError::Message(
                MessageErrorReason::InvalidMessageParams {},
            ))?;
        }
        ParamRestriction::CannotBeIncluded(keys) => {
            if json.pointer(&pointer(keys)).is_some() {
                return Err(ContractError::Message(
                    MessageErrorReason::InvalidMessageParams {},
                ));
            }
        }
        ParamRestriction::MustBeValue(keys, value) => {
            let final_value = json.pointer(&pointer(keys)).ok_or(ContractError::Message(
                MessageErrorReason::InvalidMessageParams {},
            ))?;
            // Deserialize the expected value for a more robust comparison
            let expected: Value = serde_json::from_slice(value)
                .map_err(|_| ContractError::Message(MessageErrorReason::InvalidMessageParams {}))?;
            if *final_value != expected {
                return Err(ContractError::Message(
                    MessageErrorReason::InvalidMessageParams {},
                ));
            }
        }
    }
    Ok(())
}
