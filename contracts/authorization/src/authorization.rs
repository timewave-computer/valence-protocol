use cosmwasm_std::{Binary, BlockInfo, MessageInfo, QuerierWrapper, Storage, Uint128};
use cw_utils::{must_pay, Expiration};
use serde_json::Value;
use valence_authorization_utils::{
    authorization::{
        Authorization, AuthorizationMode, AuthorizationState, PermissionType, Priority, Subroutine,
    },
    authorization_message::{MessageType, ParamRestriction},
    domain::{Domain, ExecutionEnvironment, ExternalDomain},
    function::Function,
    msg::ProcessorMessage,
};
use valence_encoder_broker::msg::QueryMsg as EncoderBrokerQueryMsg;

use crate::{
    contract::build_tokenfactory_denom,
    error::{AuthorizationErrorReason, ContractError, MessageErrorReason, UnauthorizedReason},
    state::EXTERNAL_DOMAINS,
};

pub trait Validate {
    fn validate(&self, store: &dyn Storage, querier: QuerierWrapper) -> Result<(), ContractError>;
    fn validate_subroutine(
        &self,
        store: &dyn Storage,
        querier: QuerierWrapper,
    ) -> Result<(), ContractError>;
    fn validate_functions<T: Function>(
        &self,
        store: &dyn Storage,
        functions: &[T],
        querier: QuerierWrapper,
    ) -> Result<(), ContractError>;
    fn validate_domain<T: Function>(&self, func: &T, domain: &Domain) -> Result<(), ContractError>;
    fn validate_message_type<T: Function>(
        &self,
        func: &T,
        external_domain: &Option<ExternalDomain>,
    ) -> Result<(), ContractError>;
    fn validate_evm_library<T: Function>(
        &self,
        func: &T,
        querier: &QuerierWrapper,
    ) -> Result<(), ContractError>;
    fn validate_param_restrictions<T: Function>(&self, func: &T) -> Result<(), ContractError>;
    fn ensure_enabled(&self) -> Result<(), ContractError>;
    fn ensure_active(&self, block: &BlockInfo) -> Result<(), ContractError>;
    fn ensure_not_expired(&self, block: &BlockInfo) -> Result<(), ContractError>;
    fn validate_messages(&self, messages: &[ProcessorMessage]) -> Result<(), ContractError>;
    fn validate_messages_generic<T: Function>(
        messages: &[ProcessorMessage],
        functions: &[T],
    ) -> Result<(), ContractError>;
    fn validate_executable(
        &self,
        block: &BlockInfo,
        querier: QuerierWrapper,
        contract_address: &str,
        info: &MessageInfo,
        messages: &[ProcessorMessage],
    ) -> Result<(), ContractError>;
}

impl Validate for Authorization {
    fn validate(&self, store: &dyn Storage, querier: QuerierWrapper) -> Result<(), ContractError> {
        // Label can't be empty
        if self.label.is_empty() {
            return Err(ContractError::Authorization(
                AuthorizationErrorReason::EmptyLabel {},
            ));
        }

        // Validate subroutine
        self.validate_subroutine(store, querier)?;

        // If an authorization is permissionless, it can't have high priority
        if self.mode.eq(&AuthorizationMode::Permissionless)
            && self.priority.clone().eq(&Priority::High)
        {
            return Err(ContractError::Authorization(
                AuthorizationErrorReason::PermissionlessWithHighPriority {},
            ));
        }

        Ok(())
    }

    fn validate_subroutine(
        &self,
        store: &dyn Storage,
        querier: QuerierWrapper,
    ) -> Result<(), ContractError> {
        match &self.subroutine {
            Subroutine::Atomic(config) => {
                self.validate_functions(store, &config.functions, querier)?
            }
            Subroutine::NonAtomic(config) => {
                self.validate_functions(store, &config.functions, querier)?
            }
        }
        Ok(())
    }

    /// Validates all functions in an authorization
    /// Checks:
    /// - Functions exist
    /// - Domain consistency and registration
    /// - Message type compatibility
    /// - Library validity for Evm calls
    /// - Parameter restrictions
    fn validate_functions<T: Function>(
        &self,
        store: &dyn Storage,
        functions: &[T],
        querier: QuerierWrapper,
    ) -> Result<(), ContractError> {
        // An authorization must have at least one function
        let first = functions.first().ok_or(ContractError::Authorization(
            AuthorizationErrorReason::NoFunctions {},
        ))?;

        // Get domains to perform validations
        let domain = first.domain();
        let external_domain = match domain {
            Domain::Main => None,
            Domain::External(name) => Some(
                EXTERNAL_DOMAINS
                    .load(store, name.clone())
                    .map_err(|_| ContractError::DomainIsNotRegistered(name.clone()))?,
            ),
        };

        for func in functions {
            self.validate_domain(func, domain)?;
            self.validate_message_type(func, &external_domain)?;
            self.validate_evm_library(func, &querier)?;
            self.validate_param_restrictions(func)?;
        }
        Ok(())
    }

    /// Ensures function belongs to a certain domain
    fn validate_domain<T: Function>(&self, func: &T, domain: &Domain) -> Result<(), ContractError> {
        // All messages in an authorization must be executed in the same domain (for v1)
        if !func.domain().eq(domain) {
            return Err(ContractError::Authorization(
                AuthorizationErrorReason::DifferentFunctionDomains {},
            ));
        }
        Ok(())
    }

    /// Validates message type compatibility with execution environment:
    /// - Cosmwasm: Only CosmwasmExecuteMsg/MigrateMsg allowed
    /// - Evm: Only EvmRawCall/EvmCall allowed
    /// - Main domain: Only CosmwasmExecuteMsg/MigrateMsg allowed
    fn validate_message_type<T: Function>(
        &self,
        func: &T,
        external_domain: &Option<ExternalDomain>,
    ) -> Result<(), ContractError> {
        match external_domain {
            Some(domain) => match domain.execution_environment {
                ExecutionEnvironment::Cosmwasm(_) => {
                    if !matches!(
                        func.message_details().message_type,
                        MessageType::CosmwasmExecuteMsg | MessageType::CosmwasmMigrateMsg
                    ) {
                        return Err(ContractError::Authorization(
                            AuthorizationErrorReason::InvalidMessageType {},
                        ));
                    }
                }
                ExecutionEnvironment::Evm(_, _) => {
                    if !matches!(
                        func.message_details().message_type,
                        MessageType::EvmRawCall | MessageType::EvmCall(_, _)
                    ) {
                        return Err(ContractError::Authorization(
                            AuthorizationErrorReason::InvalidMessageType {},
                        ));
                    }
                }
            },
            None => {
                // We are on Main Domain so only Cosmwasm messages are allowed
                match func.message_details().message_type {
                    MessageType::CosmwasmExecuteMsg | MessageType::CosmwasmMigrateMsg => (),
                    _ => {
                        return Err(ContractError::Authorization(
                            AuthorizationErrorReason::InvalidMessageType {},
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// For EvmCall messages:
    /// - Verifies library exists in encoder by querying the encoder broker
    fn validate_evm_library<T: Function>(
        &self,
        func: &T,
        querier: &QuerierWrapper,
    ) -> Result<(), ContractError> {
        // If it's a EvmCall, the library must be a valid library on the encoder
        if let MessageType::EvmCall(encoder, library_name) = &func.message_details().message_type {
            let exists: bool = querier.query_wasm_smart(
                encoder.broker_address.clone(),
                &EncoderBrokerQueryMsg::IsValidLibrary {
                    encoder_version: encoder.encoder_version.clone(),
                    library: library_name.clone(),
                },
            )?;
            if !exists {
                return Err(ContractError::Authorization(
                    AuthorizationErrorReason::InvalidLibraryName {},
                ));
            }
        }
        Ok(())
    }

    /// Validates parameter restrictions:
    /// - EvmRawCall: Only MustBeBytes restrictions allowed (maximum 1)
    /// - Other messages: Cannot have MustBeBytes restrictions
    fn validate_param_restrictions<T: Function>(&self, func: &T) -> Result<(), ContractError> {
        let details = func.message_details();
        let restrictions = match &details.message.params_restrictions {
            Some(r) => r,
            None => return Ok(()),
        };

        match details.message_type {
            MessageType::EvmRawCall => {
                if restrictions.len() > 1
                    || (restrictions.len() == 1
                        && !matches!(restrictions[0], ParamRestriction::MustBeBytes(_)))
                {
                    return Err(ContractError::Authorization(
                        AuthorizationErrorReason::InvalidParamRestrictions {},
                    ));
                }
            }
            _ => {
                if restrictions
                    .iter()
                    .any(|r| matches!(r, ParamRestriction::MustBeBytes(_)))
                {
                    return Err(ContractError::Authorization(
                        AuthorizationErrorReason::InvalidParamRestrictions {},
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

    fn validate_messages(&self, messages: &[ProcessorMessage]) -> Result<(), ContractError> {
        match &self.subroutine {
            Subroutine::Atomic(config) => {
                Self::validate_messages_generic(messages, &config.functions)?
            }
            Subroutine::NonAtomic(config) => {
                Self::validate_messages_generic(messages, &config.functions)?
            }
        }
        Ok(())
    }

    fn validate_messages_generic<T: Function>(
        messages: &[ProcessorMessage],
        functions: &[T],
    ) -> Result<(), ContractError> {
        if messages.len() != functions.len() {
            return Err(ContractError::Message(MessageErrorReason::InvalidAmount {}));
        }

        for (each_message, each_function) in messages.iter().zip(functions.iter()) {
            // Check that the message type matches the function type
            if !each_message.eq(&each_function.message_details().message_type) {
                return Err(ContractError::Message(MessageErrorReason::InvalidType {}));
            }

            // Extract the message from the ProcessorMessage
            let msg = each_message.get_msg();

            // If the message received is a json we're going to validate it as a json
            // If it's raw bytes we are going to validate the raw bytes
            match each_message {
                ProcessorMessage::CosmwasmExecuteMsg { .. }
                | ProcessorMessage::CosmwasmMigrateMsg { .. }
                | ProcessorMessage::EvmCall { .. } => {
                    // Extract the json from each message
                    let json: Value = serde_json::from_slice(msg.as_slice()).map_err(|e| {
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
                        .get(each_function.message_details().message.name.as_str())
                        .is_none()
                    {
                        return Err(ContractError::Message(MessageErrorReason::DoesNotMatch {}));
                    }

                    // Check that all the Parameter restrictions are met
                    if let Some(param_restrictions) =
                        &each_function.message_details().message.params_restrictions
                    {
                        for each_restriction in param_restrictions {
                            check_json_restriction(&json, each_restriction)?;
                        }
                    }
                }
                ProcessorMessage::EvmRawCall { .. } => {
                    // Check that all the Parameter restrictions are met
                    if let Some(param_restrictions) =
                        &each_function.message_details().message.params_restrictions
                    {
                        for each_restriction in param_restrictions {
                            check_bytes_restriction(msg, each_restriction)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_executable(
        &self,
        block: &BlockInfo,
        querier: QuerierWrapper,
        contract_address: &str,
        info: &MessageInfo,
        messages: &[ProcessorMessage],
    ) -> Result<(), ContractError> {
        self.ensure_enabled()?;
        self.ensure_active(block)?;
        self.ensure_not_expired(block)?;
        validate_permission(
            &self.label,
            &self.mode,
            querier,
            contract_address,
            info,
        )?;
        self.validate_messages(messages)?;

        Ok(())
    }
}

fn check_json_restriction(
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
    let pointer = |keys: &[String]| -> String { format!("/{}", keys.join("/")) };

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
        // We should never get here because we validate authorizations to not have these restrictions for json messages
        ParamRestriction::MustBeBytes(_) => {
            return Err(ContractError::Message(MessageErrorReason::InvalidType {}))
        }
    }
    Ok(())
}

fn check_bytes_restriction(
    msg: &Binary,
    param_restriction: &ParamRestriction,
) -> Result<(), ContractError> {
    match param_restriction {
        ParamRestriction::MustBeBytes(expected) => {
            if msg != expected {
                return Err(ContractError::Message(
                    MessageErrorReason::InvalidMessageParams {},
                ));
            }
        }
        // We should never get here because we validate authorizations to not have these restrictions for raw bytes messages
        _ => return Err(ContractError::Message(MessageErrorReason::InvalidType {})),
    }
    Ok(())
}

pub fn validate_permission(
    label: &str,
    authorization_mode: &AuthorizationMode,
    querier: QuerierWrapper,
    contract_address: &str,
    info: &MessageInfo,
) -> Result<(), ContractError> {
    let token_factory_denom = build_tokenfactory_denom(contract_address, label);
    match authorization_mode {
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
