use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin, traits::MessageExt};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coins, from_json, to_json_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, Order, Response, StdResult, Storage, Uint64, WasmMsg,
};
use cw_ownable::{assert_owner, get_ownership, initialize_owner, is_owner};
use cw_storage_plus::Bound;
use cw_utils::Expiration;
use neutron_sdk::proto_types::osmosis::tokenfactory::v1beta1::{MsgBurn, MsgCreateDenom, MsgMint};
use valence_authorization_utils::{
    authorization::{
        Authorization, AuthorizationInfo, AuthorizationMode, AuthorizationState, PermissionType,
        Priority,
    },
    callback::{ExecutionResult, OperationInitiator, PolytoneCallbackMsg, ProcessorCallbackInfo},
    domain::{
        CosmwasmBridge, Domain, EvmBridge, ExecutionEnvironment, ExternalDomain,
        PolytoneConnectors, PolytoneNote, PolytoneProxyState,
    },
    msg::{
        ExecuteMsg, ExternalDomainInfo, InstantiateMsg, InternalAuthorizationMsg, Mint, OwnerMsg,
        PermissionedMsg, PermissionlessMsg, ProcessorMessage, QueryMsg,
    },
};
use valence_encoder_broker::msg::QueryMsg as EncoderBrokerQueryMsg;
use valence_encoder_utils::msg::{
    convert_into_encoder_messages, ProcessorMessageToDecode, ProcessorMessageToEncode,
};
use valence_gmp_utils::{
    hyperlane::{
        format_address_for_hyperlane, HandleMsg, InterchainSecurityModuleResponse,
        IsmSpecifierQueryMsg,
    },
    polytone::{Callback, CallbackMessage, CallbackRequest, PolytoneExecuteMsg},
};
use valence_processor_utils::msg::{AuthorizationMsg, ExecuteMsg as ProcessorExecuteMsg};

use crate::{
    authorization::Validate,
    domain::{add_external_domain, create_msg_for_processor, get_domain},
    error::{AuthorizationErrorReason, ContractError, MessageErrorReason, UnauthorizedReason},
    state::{
        AUTHORIZATIONS, CURRENT_EXECUTIONS, EXECUTION_ID, EXTERNAL_DOMAINS, FIRST_OWNERSHIP,
        PROCESSOR_CALLBACKS, PROCESSOR_ON_MAIN_DOMAIN, SUB_OWNERS,
    },
};

// pagination info for queries
const MAX_PAGE_LIMIT: u32 = 250;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Set up owners and initial subowners
    initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    for sub_owner in msg.sub_owners {
        SUB_OWNERS.save(
            deps.storage,
            deps.api.addr_validate(sub_owner.as_str())?,
            &Empty {},
        )?;
    }

    // Save processor on main domain
    PROCESSOR_ON_MAIN_DOMAIN.save(
        deps.storage,
        &deps.api.addr_validate(msg.processor.as_str())?,
    )?;

    EXECUTION_ID.save(deps.storage, &0)?;
    // When onwership is transferred for the first time this will be changed
    FIRST_OWNERSHIP.save(deps.storage, &true)?;

    Ok(Response::new().add_attribute("method", "instantiate_authorization"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwnership(action) => update_ownership(deps, env, info, action),
        ExecuteMsg::OwnerAction(owner_msg) => {
            assert_owner(deps.storage, &info.sender)?;
            match owner_msg {
                OwnerMsg::AddSubOwner { sub_owner } => add_sub_owner(deps, sub_owner),
                OwnerMsg::RemoveSubOwner { sub_owner } => remove_sub_owner(deps, sub_owner),
            }
        }
        ExecuteMsg::PermissionedAction(permissioned_msg) => {
            assert_owner_or_subowner(deps.storage, info.sender)?;
            match permissioned_msg {
                PermissionedMsg::AddExternalDomains { external_domains } => {
                    add_external_domains(deps, env, external_domains)
                }
                PermissionedMsg::CreateAuthorizations { authorizations } => {
                    create_authorizations(deps, env, authorizations)
                }
                PermissionedMsg::ModifyAuthorization {
                    label,
                    not_before,
                    expiration,
                    max_concurrent_executions,
                    priority,
                } => modify_authorization(
                    deps,
                    label,
                    not_before,
                    expiration,
                    max_concurrent_executions,
                    priority,
                ),
                PermissionedMsg::DisableAuthorization { label } => {
                    disable_authorization(deps, label)
                }
                PermissionedMsg::EnableAuthorization { label } => enable_authorization(deps, label),
                PermissionedMsg::MintAuthorizations { label, mints } => {
                    mint_authorizations(deps, env, label, mints)
                }
                PermissionedMsg::EvictMsgs {
                    domain,
                    queue_position,
                    priority,
                } => evict_messages(deps, domain, queue_position, priority),
                PermissionedMsg::InsertMsgs {
                    label,
                    queue_position,
                    priority,
                    messages,
                } => insert_messages(deps, env, label, queue_position, priority, messages),
                PermissionedMsg::PauseProcessor { domain } => pause_processor(deps, domain),
                PermissionedMsg::ResumeProcessor { domain } => resume_processor(deps, domain),
            }
        }
        ExecuteMsg::PermissionlessAction(permissionless_msg) => match permissionless_msg {
            PermissionlessMsg::SendMsgs {
                label,
                messages,
                ttl,
            } => send_msgs(deps, env, info, label, ttl, messages),
            PermissionlessMsg::RetryMsgs { execution_id } => retry_msgs(deps, env, execution_id),
            PermissionlessMsg::RetryBridgeCreation { domain_name } => {
                retry_bridge_creation(deps, env, domain_name)
            }
        },
        ExecuteMsg::InternalAuthorizationAction(internal_authorization_msg) => {
            match internal_authorization_msg {
                InternalAuthorizationMsg::ProcessorCallback {
                    execution_id,
                    execution_result,
                } => process_processor_callback(deps, env, info, execution_id, execution_result),
            }
        }
        ExecuteMsg::PolytoneCallback(callback_msg) => {
            process_polytone_callback(deps, env, info, callback_msg)
        }
        ExecuteMsg::HyperlaneCallback(handle_msg) => {
            process_hyperlane_callback(deps, env, info, handle_msg)
        }
    }
}

fn update_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    if let cw_ownable::Action::TransferOwnership { new_owner, .. } = &action {
        if FIRST_OWNERSHIP.load(deps.storage)? {
            assert_owner(deps.storage, &info.sender)?;
            FIRST_OWNERSHIP.save(deps.storage, &false)?;
            let ownership = initialize_owner(deps.storage, deps.api, Some(new_owner))?;
            return Ok(Response::new().add_attributes(ownership.into_attributes()));
        }
    }

    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::new().add_attributes(ownership.into_attributes()))
}

fn add_sub_owner(deps: DepsMut, sub_owner: String) -> Result<Response, ContractError> {
    SUB_OWNERS.save(deps.storage, deps.api.addr_validate(&sub_owner)?, &Empty {})?;

    Ok(Response::new()
        .add_attribute("action", "add_sub_owner")
        .add_attribute("sub_owner", sub_owner))
}

fn remove_sub_owner(deps: DepsMut, sub_owner: String) -> Result<Response, ContractError> {
    SUB_OWNERS.remove(deps.storage, deps.api.addr_validate(&sub_owner)?);

    Ok(Response::new()
        .add_attribute("action", "remove_sub_owner")
        .add_attribute("sub_owner", sub_owner))
}

fn add_external_domains(
    mut deps: DepsMut,
    env: Env,
    external_domains: Vec<ExternalDomainInfo>,
) -> Result<Response, ContractError> {
    let mut messages = vec![];

    // Save all external domains
    for domain in external_domains {
        let validated_external_domain = add_external_domain(deps.branch(), domain)?;

        // Only create message for Cosmwasm Polytone case
        if let ExecutionEnvironment::Cosmwasm(cosmwasm_bridge) =
            validated_external_domain.execution_environment
        {
            let CosmwasmBridge::Polytone(polytone_connectors) = cosmwasm_bridge;
            // In polytone to create the proxy we can send an empty vector of messages
            let msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: polytone_connectors.polytone_note.address.to_string(),
                msg: to_json_binary(&PolytoneExecuteMsg::Execute {
                    msgs: vec![],
                    callback: Some(CallbackRequest {
                        receiver: env.contract.address.to_string(),
                        // When we add domain we will return a callback with the name of the domain
                        // to know that we are getting the callback when trying to create the proxy
                        msg: to_json_binary(&PolytoneCallbackMsg::CreateProxy(
                            validated_external_domain.name,
                        ))?,
                    }),
                    timeout_seconds: Uint64::from(
                        polytone_connectors.polytone_note.timeout_seconds,
                    ),
                })?,
                funds: vec![],
            });
            messages.push(msg);
        }
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "add_external_domains"))
}

fn create_authorizations(
    deps: DepsMut,
    env: Env,
    authorizations: Vec<AuthorizationInfo>,
) -> Result<Response, ContractError> {
    let mut tokenfactory_msgs = vec![];

    for each_authorization in authorizations {
        let authorization = each_authorization.into_authorization(&env.block, deps.api);

        // Check that it doesn't exist yet
        if AUTHORIZATIONS.has(deps.storage, authorization.label.clone()) {
            return Err(ContractError::Authorization(
                AuthorizationErrorReason::LabelAlreadyExists(authorization.label.clone()),
            ));
        }

        // Perform all validations on the authorization
        authorization.validate(deps.storage, deps.querier)?;

        // If Authorization is permissioned we need to create the tokenfactory token and mint the corresponding amounts to the addresses that can
        // execute the authorization
        if let AuthorizationMode::Permissioned(permission_type) = &authorization.mode {
            // We will always create the token if it's permissioned
            tokenfactory_msgs.push(create_denom_msg(
                env.contract.address.to_string(),
                authorization.label.clone(),
            ));

            // Full denom of the token that will be created
            let denom =
                build_tokenfactory_denom(env.contract.address.as_str(), &authorization.label);

            match permission_type {
                // If there is a call limit we will mint the amount of tokens specified in the call limit for each address and these will be burned after each correct execution
                PermissionType::WithCallLimit(call_limits) => {
                    for (addr, limit) in call_limits {
                        deps.api.addr_validate(addr.as_str())?;
                        let mint_msg = mint_msg(
                            env.contract.address.to_string(),
                            addr.to_string(),
                            limit.u128(),
                            denom.clone(),
                        );
                        tokenfactory_msgs.push(mint_msg);
                    }
                }
                // If it has no call limit we will mint 1 token for each address which will be used to verify if they can execute the authorization via a query
                PermissionType::WithoutCallLimit(addrs) => {
                    for addr in addrs {
                        deps.api.addr_validate(addr.as_str())?;
                        let mint_msg = mint_msg(
                            env.contract.address.to_string(),
                            addr.to_string(),
                            1,
                            denom.clone(),
                        );
                        tokenfactory_msgs.push(mint_msg);
                    }
                }
            }
        }

        // Save the authorization in the state
        AUTHORIZATIONS.save(deps.storage, authorization.label.clone(), &authorization)?;
    }

    Ok(Response::new()
        .add_attribute("action", "create_authorizations")
        .add_messages(tokenfactory_msgs))
}

fn modify_authorization(
    deps: DepsMut,
    label: String,
    not_before: Option<Expiration>,
    expiration: Option<Expiration>,
    max_concurrent_executions: Option<u64>,
    priority: Option<Priority>,
) -> Result<Response, ContractError> {
    let mut authorization = AUTHORIZATIONS
        .load(deps.storage, label.clone())
        .map_err(|_| {
            ContractError::Authorization(AuthorizationErrorReason::DoesNotExist(label.clone()))
        })?;

    if let Some(not_before) = not_before {
        authorization.not_before = not_before;
    }

    if let Some(expiration) = expiration {
        authorization.expiration = expiration;
    }

    if let Some(max_concurrent_executions) = max_concurrent_executions {
        authorization.max_concurrent_executions = max_concurrent_executions;
    }
    if let Some(priority) = priority {
        authorization.priority = priority;
    }

    authorization.validate(deps.storage, deps.querier)?;

    AUTHORIZATIONS.save(deps.storage, label, &authorization)?;

    Ok(Response::new().add_attribute("action", "modify_authorization"))
}

fn disable_authorization(deps: DepsMut, label: String) -> Result<Response, ContractError> {
    let mut authorization = AUTHORIZATIONS
        .load(deps.storage, label.clone())
        .map_err(|_| {
            ContractError::Authorization(AuthorizationErrorReason::DoesNotExist(label.clone()))
        })?;

    authorization.state = AuthorizationState::Disabled;

    AUTHORIZATIONS.save(deps.storage, label, &authorization)?;

    Ok(Response::new().add_attribute("action", "disable_authorization"))
}

fn enable_authorization(deps: DepsMut, label: String) -> Result<Response, ContractError> {
    let mut authorization = AUTHORIZATIONS
        .load(deps.storage, label.clone())
        .map_err(|_| {
            ContractError::Authorization(AuthorizationErrorReason::DoesNotExist(label.clone()))
        })?;

    authorization.state = AuthorizationState::Enabled;

    AUTHORIZATIONS.save(deps.storage, label, &authorization)?;

    Ok(Response::new().add_attribute("action", "enable_authorization"))
}

fn mint_authorizations(
    deps: DepsMut,
    env: Env,
    label: String,
    mints: Vec<Mint>,
) -> Result<Response, ContractError> {
    let authorization = AUTHORIZATIONS
        .load(deps.storage, label.clone())
        .map_err(|_| {
            ContractError::Authorization(AuthorizationErrorReason::DoesNotExist(label.clone()))
        })?;

    let tokenfactory_msgs = match authorization.mode {
        AuthorizationMode::Permissioned(_) => Ok(mints.iter().map(|mint| {
            mint_msg(
                env.contract.address.to_string(),
                mint.address.clone(),
                mint.amount.u128(),
                build_tokenfactory_denom(env.contract.address.as_str(), &label),
            )
        })),
        AuthorizationMode::Permissionless => Err(ContractError::Authorization(
            AuthorizationErrorReason::CantMintForPermissionless {},
        )),
    }?;

    Ok(Response::new()
        .add_attribute("action", "mint_authorizations")
        .add_messages(tokenfactory_msgs))
}

fn pause_processor(deps: DepsMut, domain: Domain) -> Result<Response, ContractError> {
    // The pause msg that is used for both main and external Cosmwasm domains
    let pause_msg = ProcessorExecuteMsg::AuthorizationModuleAction(AuthorizationMsg::Pause {});
    let execute_msg_binary = match domain.clone() {
        Domain::Main => to_json_binary(&pause_msg)?,
        Domain::External(external_domain_id) => {
            let external_domain = EXTERNAL_DOMAINS.load(deps.storage, external_domain_id)?;
            match external_domain.execution_environment {
                ExecutionEnvironment::Cosmwasm(_) => to_json_binary(&pause_msg)?,
                ExecutionEnvironment::Evm(encoder, _) => {
                    // We are going to query the encoder to get the corresponding message to pause the processor
                    deps.querier.query_wasm_smart(
                        encoder.broker_address,
                        &EncoderBrokerQueryMsg::Encode {
                            encoder_version: encoder.encoder_version,
                            message: ProcessorMessageToEncode::Pause {},
                        },
                    )?
                }
            }
        }
    };
    let message = create_msg_for_processor(deps.storage, execute_msg_binary, &domain, None)?;

    Ok(Response::new()
        .add_message(message)
        .add_attribute("action", "pause_processor"))
}

fn resume_processor(deps: DepsMut, domain: Domain) -> Result<Response, ContractError> {
    // The resume msg that is used for both main and external Cosmwasm domains
    let resume_msg = ProcessorExecuteMsg::AuthorizationModuleAction(AuthorizationMsg::Resume {});
    let execute_msg_binary = match domain.clone() {
        Domain::Main => to_json_binary(&resume_msg)?,
        Domain::External(external_domain_id) => {
            let external_domain = EXTERNAL_DOMAINS.load(deps.storage, external_domain_id)?;
            match external_domain.execution_environment {
                ExecutionEnvironment::Cosmwasm(_) => to_json_binary(&resume_msg)?,
                ExecutionEnvironment::Evm(encoder, _) => {
                    // We are going to query the encoder to get the corresponding message to resume the processor
                    deps.querier.query_wasm_smart(
                        encoder.broker_address,
                        &EncoderBrokerQueryMsg::Encode {
                            encoder_version: encoder.encoder_version,
                            message: ProcessorMessageToEncode::Resume {},
                        },
                    )?
                }
            }
        }
    };
    let message = create_msg_for_processor(deps.storage, execute_msg_binary, &domain, None)?;

    Ok(Response::new()
        .add_message(message)
        .add_attribute("action", "resume_processor"))
}

fn insert_messages(
    deps: DepsMut,
    env: Env,
    label: String,
    queue_position: u64,
    priority: Priority,
    messages: Vec<ProcessorMessage>,
) -> Result<Response, ContractError> {
    let authorization = AUTHORIZATIONS
        .load(deps.storage, label.clone())
        .map_err(|_| {
            ContractError::Authorization(AuthorizationErrorReason::DoesNotExist(label.clone()))
        })?;

    // Validate that the messages match with the label
    authorization.validate_messages(&messages)?;

    let current_executions = CURRENT_EXECUTIONS
        .load(deps.storage, label.clone())
        .unwrap_or_default();
    CURRENT_EXECUTIONS.save(
        deps.storage,
        label.clone(),
        &current_executions.checked_add(1).expect("Overflow"),
    )?;

    let domain = get_domain(&authorization)?;
    let id = get_and_increase_execution_id(deps.storage)?;
    let expiration_time = authorization
        .subroutine
        .get_expiration_time()
        .and_then(|time| time.checked_add(env.block.time.seconds()));

    let insert_msgs =
        ProcessorExecuteMsg::AuthorizationModuleAction(AuthorizationMsg::InsertMsgs {
            id,
            queue_position,
            msgs: messages.clone(),
            subroutine: authorization.subroutine.clone(),
            priority: priority.clone(),
            expiration_time,
        });

    let execute_msg_binary = match domain.clone() {
        Domain::Main => to_json_binary(&insert_msgs)?,
        Domain::External(external_domain_id) => {
            let external_domain = EXTERNAL_DOMAINS.load(deps.storage, external_domain_id)?;
            match external_domain.execution_environment {
                ExecutionEnvironment::Cosmwasm(_) => to_json_binary(&insert_msgs)?,
                ExecutionEnvironment::Evm(encoder, _) => {
                    // Transform processor messages with encoder messages that have information of the library they need to encode to, according
                    // to the authorization
                    let encoder_messages =
                        convert_into_encoder_messages(messages.clone(), &authorization)?;
                    deps.querier.query_wasm_smart(
                        encoder.broker_address,
                        &EncoderBrokerQueryMsg::Encode {
                            encoder_version: encoder.encoder_version,
                            message: ProcessorMessageToEncode::InsertMsgs {
                                execution_id: id,
                                queue_position,
                                priority,
                                subroutine: authorization.subroutine,
                                expiration_time,
                                messages: encoder_messages,
                            },
                        },
                    )?
                }
            }
        }
    };

    // Callback request used for polytone
    let callback_request = CallbackRequest {
        receiver: env.contract.address.to_string(),
        // We will use the ID to know which callback we are getting
        msg: to_json_binary(&PolytoneCallbackMsg::ExecutionID(id))?,
    };

    let msg = create_msg_for_processor(
        deps.storage,
        execute_msg_binary,
        &domain,
        Some(callback_request),
    )?;

    store_inprocess_callback(
        deps.storage,
        env.block.time.seconds(),
        id,
        OperationInitiator::Owner,
        domain,
        label,
        None,
        messages,
    )?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "add_messages")
        .add_attribute("authorization_label", authorization.label))
}

fn evict_messages(
    deps: DepsMut,
    domain: Domain,
    queue_position: u64,
    priority: Priority,
) -> Result<Response, ContractError> {
    let evict_msg = ProcessorExecuteMsg::AuthorizationModuleAction(AuthorizationMsg::EvictMsgs {
        queue_position,
        priority: priority.clone(),
    });

    let execute_msg_binary = match domain.clone() {
        Domain::Main => to_json_binary(&evict_msg)?,
        Domain::External(external_domain_id) => {
            let external_domain = EXTERNAL_DOMAINS.load(deps.storage, external_domain_id)?;
            match external_domain.execution_environment {
                ExecutionEnvironment::Cosmwasm(_) => to_json_binary(&evict_msg)?,
                ExecutionEnvironment::Evm(encoder, _) => deps.querier.query_wasm_smart(
                    encoder.broker_address,
                    &EncoderBrokerQueryMsg::Encode {
                        encoder_version: encoder.encoder_version,
                        message: ProcessorMessageToEncode::EvictMsgs {
                            queue_position,
                            priority,
                        },
                    },
                )?,
            }
        }
    };
    let msg = create_msg_for_processor(deps.storage, execute_msg_binary, &domain, None)?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "remove_messages"))
}

fn send_msgs(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    label: String,
    ttl: Option<Expiration>,
    messages: Vec<ProcessorMessage>,
) -> Result<Response, ContractError> {
    let authorization = AUTHORIZATIONS
        .load(deps.storage, label.clone())
        .map_err(|_| {
            ContractError::Authorization(AuthorizationErrorReason::DoesNotExist(label.clone()))
        })?;

    authorization.validate_executable(
        &env.block,
        deps.querier,
        env.contract.address.as_str(),
        &info,
        &messages,
    )?;

    // We need to check that we haven't reached the max concurrent executions and if not, increase it by 1
    let current_executions = CURRENT_EXECUTIONS
        .load(deps.storage, label.clone())
        .unwrap_or_default();
    if current_executions >= authorization.max_concurrent_executions {
        return Err(ContractError::Authorization(
            AuthorizationErrorReason::MaxConcurrentExecutionsReached {},
        ));
    }
    CURRENT_EXECUTIONS.save(
        deps.storage,
        label.clone(),
        &current_executions.checked_add(1).expect("Overflow"),
    )?;

    // Get the domain to know which processor to use
    let domain = get_domain(&authorization)?;
    // Get the ID we are going to use for the execution (used to process callbacks)
    let id = get_and_increase_execution_id(deps.storage)?;
    // Calculate batch expiration time if Subroutine has one
    let expiration_time = authorization
        .subroutine
        .get_expiration_time()
        .and_then(|time| time.checked_add(env.block.time.seconds()));
    // Message for the processor
    let send_msgs = ProcessorExecuteMsg::AuthorizationModuleAction(AuthorizationMsg::EnqueueMsgs {
        id,
        msgs: messages.clone(),
        subroutine: authorization.subroutine.clone(),
        priority: authorization.priority.clone(),
        expiration_time,
    });

    let execute_msg_binary = match domain.clone() {
        Domain::Main => to_json_binary(&send_msgs)?,
        Domain::External(external_domain_id) => {
            let external_domain = EXTERNAL_DOMAINS.load(deps.storage, external_domain_id)?;
            match external_domain.execution_environment {
                ExecutionEnvironment::Cosmwasm(_) => to_json_binary(&send_msgs)?,
                ExecutionEnvironment::Evm(encoder, _) => {
                    // Transform processor messages with encoder messages that have information of the library they need to encode to, according
                    // to the authorization
                    let encoder_messages =
                        convert_into_encoder_messages(messages.clone(), &authorization)?;
                    deps.querier.query_wasm_smart(
                        encoder.broker_address,
                        &EncoderBrokerQueryMsg::Encode {
                            encoder_version: encoder.encoder_version,
                            message: ProcessorMessageToEncode::SendMsgs {
                                execution_id: id,
                                priority: authorization.priority,
                                subroutine: authorization.subroutine,
                                expiration_time,
                                messages: encoder_messages,
                            },
                        },
                    )?
                }
            }
        }
    };

    let callback_request = CallbackRequest {
        receiver: env.contract.address.to_string(),
        // We will use the ID to know which callback we are getting
        msg: to_json_binary(&PolytoneCallbackMsg::ExecutionID(id))?,
    };

    // We need to know if this will be sent to the processor on the main domain or to an external domain
    let msg = create_msg_for_processor(
        deps.storage,
        execute_msg_binary,
        &domain,
        Some(callback_request),
    )?;

    store_inprocess_callback(
        deps.storage,
        env.block.time.seconds(),
        id,
        OperationInitiator::User(info.sender),
        domain,
        label,
        ttl,
        messages,
    )?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "send_msgs")
        .add_attribute("authorization_label", authorization.label))
}

fn retry_msgs(deps: DepsMut, env: Env, execution_id: u64) -> Result<Response, ContractError> {
    let mut callback_info = PROCESSOR_CALLBACKS
        .load(deps.storage, execution_id)
        .map_err(|_| ContractError::ExecutionIDNotFound { execution_id })?;

    // Only messages that are in Timeout(retriable) state can be retried
    if callback_info.execution_result != ExecutionResult::Timeout(true) {
        return Err(ContractError::Message(MessageErrorReason::NotRetriable {}));
    }

    let mut messages = vec![];
    match callback_info.ttl {
        Some(ttl) if !ttl.is_expired(&env.block) => {
            // They can be retried
            // Check if we already passed the maximum amount of concurrent executions and update it if we didn't
            let current_executions =
                CURRENT_EXECUTIONS.load(deps.storage, callback_info.label.clone())?;
            let authorization = AUTHORIZATIONS.load(deps.storage, callback_info.label.clone())?;
            if current_executions >= authorization.max_concurrent_executions {
                return Err(ContractError::Authorization(
                    AuthorizationErrorReason::MaxConcurrentExecutionsReached {},
                ));
            }
            CURRENT_EXECUTIONS.save(
                deps.storage,
                callback_info.label.clone(),
                &current_executions.checked_add(1).expect("Overflow"),
            )?;
            // Calculate batch expiration time if Subroutine has one
            let expiration_time = authorization
                .subroutine
                .get_expiration_time()
                .and_then(|time| time.checked_add(env.block.time.seconds()));
            let execute_msg_binary = to_json_binary(
                &ProcessorExecuteMsg::AuthorizationModuleAction(AuthorizationMsg::EnqueueMsgs {
                    id: execution_id,
                    msgs: callback_info.messages.clone(),
                    subroutine: authorization.subroutine,
                    priority: authorization.priority,
                    expiration_time,
                }),
            )?;
            // Update the state
            callback_info.execution_result = ExecutionResult::InProcess;
            // Update the last_updated_at timestamp
            callback_info.last_updated_at = env.block.time.seconds();
            // Create the callback again
            let callback_request = CallbackRequest {
                receiver: env.contract.address.to_string(),
                // We will use the ID to know which callback we are getting
                msg: to_json_binary(&PolytoneCallbackMsg::ExecutionID(execution_id))?,
            };
            messages.push(create_msg_for_processor(
                deps.storage,
                execute_msg_binary,
                &callback_info.domain,
                Some(callback_request),
            )?);
        }
        _ => {
            // TTL has expired, check if we need to send a token back
            if let (
                OperationInitiator::User(initiator_addr),
                AuthorizationMode::Permissioned(PermissionType::WithCallLimit(_)),
            ) = (
                &callback_info.initiator,
                &AUTHORIZATIONS
                    .load(deps.storage, callback_info.label.clone())?
                    .mode,
            ) {
                // Update the state to not retriable anymore
                callback_info.execution_result = ExecutionResult::Timeout(false);
                // Update the last_updated_at timestamp
                callback_info.last_updated_at = env.block.time.seconds();

                let denom =
                    build_tokenfactory_denom(env.contract.address.as_str(), &callback_info.label);
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: initiator_addr.to_string(),
                    amount: coins(1, denom),
                }));
            }
        }
    };

    // Save the callback info that was modified when processing the retry
    PROCESSOR_CALLBACKS.save(deps.storage, execution_id, &callback_info)?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "retry_msgs"))
}

fn retry_bridge_creation(
    deps: DepsMut,
    env: Env,
    domain_name: String,
) -> Result<Response, ContractError> {
    let mut external_domain = EXTERNAL_DOMAINS.load(deps.storage, domain_name.clone())?;

    let msg = match external_domain.execution_environment {
        ExecutionEnvironment::Cosmwasm(cosmwasm_bridge) => match cosmwasm_bridge {
            CosmwasmBridge::Polytone(polytone_connectors) => {
                let polytone_note = &polytone_connectors.polytone_note;
                if polytone_note.state.ne(&PolytoneProxyState::TimedOut) {
                    return Err(ContractError::Unauthorized(
                        UnauthorizedReason::BridgeCreationNotTimedOut {},
                    ));
                }

                // Update the state
                let new_polytone_note = PolytoneNote {
                    address: polytone_note.address.clone(),
                    timeout_seconds: polytone_note.timeout_seconds,
                    state: PolytoneProxyState::PendingResponse,
                };
                let new_polytone_connectors = PolytoneConnectors {
                    polytone_note: new_polytone_note,
                    polytone_proxy: polytone_connectors.polytone_proxy.clone(),
                };
                external_domain.execution_environment = ExecutionEnvironment::Cosmwasm(
                    CosmwasmBridge::Polytone(new_polytone_connectors),
                );
                EXTERNAL_DOMAINS.save(deps.storage, domain_name.clone(), &external_domain)?;

                WasmMsg::Execute {
                    contract_addr: polytone_note.address.to_string(),
                    msg: to_json_binary(&PolytoneExecuteMsg::Execute {
                        msgs: vec![],
                        callback: Some(CallbackRequest {
                            receiver: env.contract.address.to_string(),
                            // When we add domain we will return a callback with the name of the domain to know that we are getting the callback when trying to create the proxy
                            msg: to_json_binary(&PolytoneCallbackMsg::CreateProxy(domain_name))?,
                        }),
                        timeout_seconds: Uint64::from(polytone_note.timeout_seconds),
                    })?,
                    funds: vec![],
                }
            }
        },
        ExecutionEnvironment::Evm(_, evm_bridge) => match evm_bridge {
            EvmBridge::Hyperlane(_) => {
                return Err(ContractError::BridgeCreationNotRequired {});
            }
        },
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "retry_bridge_creation"))
}

fn process_processor_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    execution_id: u64,
    execution_result: ExecutionResult,
) -> Result<Response, ContractError> {
    let mut callback = PROCESSOR_CALLBACKS.load(deps.storage, execution_id)?;

    // Check that the sender is the one that should send the callback
    if info.sender != callback.processor_callback_address {
        return Err(ContractError::Unauthorized(
            UnauthorizedReason::UnauthorizedProcessorCallbackSender {},
        ));
    }

    // Update the result
    callback.execution_result = execution_result;
    // Update the last_updated_at timestamp
    callback.last_updated_at = env.block.time.seconds();
    PROCESSOR_CALLBACKS.save(deps.storage, execution_id, &callback)?;

    // Reduce the current executions for the label
    CURRENT_EXECUTIONS.update(
        deps.storage,
        callback.label.clone(),
        |current| -> Result<u64, ContractError> {
            let count = current.unwrap_or_default();
            if count == 0 {
                Err(ContractError::CurrentExecutionsIsZero {})
            } else {
                Ok(count - 1)
            }
        },
    )?;

    // Check if a token was sent to perform this operation and that it wasn't started by the owner
    let authorization = AUTHORIZATIONS.load(deps.storage, callback.label.clone())?;
    let mut messages = vec![];
    if let OperationInitiator::User(initiator_addr) = &callback.initiator {
        if let AuthorizationMode::Permissioned(PermissionType::WithCallLimit(_)) =
            authorization.mode
        {
            let denom =
                build_tokenfactory_denom(env.contract.address.as_str(), &authorization.label);

            let msg = match callback.execution_result {
                ExecutionResult::Success
                | ExecutionResult::PartiallyExecuted(_, _)
                | ExecutionResult::RemovedByOwner => {
                    // If the operation was executed, partially executed or removed by the owner the token will be burned
                    burn_msg(env.contract.address.to_string(), 1, denom)
                }
                _ => {
                    // Otherwise, the tokens will be sent back
                    CosmosMsg::Bank(BankMsg::Send {
                        to_address: initiator_addr.to_string(),
                        amount: coins(1, denom),
                    })
                }
            };

            messages.push(msg);
        }
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "process_processor_callback")
        .add_attribute("execution_id", execution_id.to_string()))
}

fn process_polytone_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    callback_msg: CallbackMessage,
) -> Result<Response, ContractError> {
    // We will only process callbacks from messages initiated by the authorization contract
    if callback_msg.initiator != env.contract.address {
        return Err(ContractError::Unauthorized(
            UnauthorizedReason::InvalidPolytoneCallbackInitiator {},
        ));
    }

    // Parse the initiator_msg into our new PolytoneCallbackMsg enum
    let Ok(polytone_callback_msg) = from_json::<PolytoneCallbackMsg>(callback_msg.initiator_msg)
    else {
        return Err(ContractError::Message(
            MessageErrorReason::InvalidPolytoneCallback {},
        ));
    };

    let mut messages = vec![];

    match polytone_callback_msg {
        PolytoneCallbackMsg::ExecutionID(execution_id) => {
            // Make sure that the right address sent the polytone callback
            let mut callback_info = PROCESSOR_CALLBACKS.load(deps.storage, execution_id)?;

            // Get the polytone address
            let Some(ref polytone_address) = callback_info.bridge_callback_address else {
                return Err(ContractError::Unauthorized(
                    UnauthorizedReason::UnauthorizedCallbackSender {},
                ));
            };

            // Only the polytone address can send the callback
            if info.sender != polytone_address {
                return Err(ContractError::Unauthorized(
                    UnauthorizedReason::UnauthorizedCallbackSender {},
                ));
            }

            match callback_msg.result {
                Callback::Execute(result) => {
                    match result {
                        Ok(_) => (),
                        Err(error) => {
                            if callback_info.execution_result == ExecutionResult::InProcess {
                                let is_expired = callback_info
                                    .ttl
                                    .map(|ttl| ttl.is_expired(&env.block))
                                    .unwrap_or(true);
                                callback_info.execution_result = if error == "timeout" {
                                    if is_expired {
                                        // We check if we need to send the token back, if action was initiatiated by a user and a token was sent
                                        if let (
                                            OperationInitiator::User(initiator_addr),
                                            AuthorizationMode::Permissioned(
                                                PermissionType::WithCallLimit(_),
                                            ),
                                        ) = (
                                            &callback_info.initiator,
                                            &AUTHORIZATIONS
                                                .load(deps.storage, callback_info.label.clone())?
                                                .mode,
                                        ) {
                                            let denom = build_tokenfactory_denom(
                                                env.contract.address.as_str(),
                                                &callback_info.label,
                                            );
                                            messages.push(CosmosMsg::Bank(BankMsg::Send {
                                                to_address: initiator_addr.to_string(),
                                                amount: coins(1, denom),
                                            }));
                                        }
                                    }
                                    // If it's expired it's not retriable anymore
                                    ExecutionResult::Timeout(!is_expired)
                                } else {
                                    ExecutionResult::UnexpectedError(error)
                                };

                                // Update the last_updated_at timestamp
                                callback_info.last_updated_at = env.block.time.seconds();
                                // Save the callback update
                                PROCESSOR_CALLBACKS.save(
                                    deps.storage,
                                    execution_id,
                                    &callback_info,
                                )?;

                                // Update the current executions for the label
                                CURRENT_EXECUTIONS.update(
                                    deps.storage,
                                    callback_info.label,
                                    |current| -> Result<u64, ContractError> {
                                        let count = current.unwrap_or_default();
                                        if count == 0 {
                                            Err(ContractError::CurrentExecutionsIsZero {})
                                        } else {
                                            Ok(count - 1)
                                        }
                                    },
                                )?;
                            }
                        }
                    }
                }
                // We might have run out of gas so we need to log the error for this and it won't be retriable
                Callback::FatalError(error) => {
                    callback_info.execution_result = ExecutionResult::UnexpectedError(error);

                    // Update the last_updated_at timestamp
                    callback_info.last_updated_at = env.block.time.seconds();
                    // Save the callback update
                    PROCESSOR_CALLBACKS.save(deps.storage, execution_id, &callback_info)?;

                    // Update the current executions for the label
                    CURRENT_EXECUTIONS.update(
                        deps.storage,
                        callback_info.label,
                        |current| -> Result<u64, ContractError> {
                            let count = current.unwrap_or_default();
                            if count == 0 {
                                Err(ContractError::CurrentExecutionsIsZero {})
                            } else {
                                Ok(count - 1)
                            }
                        },
                    )?;
                }
                // This should never happen because we are not sending queries
                Callback::Query(_) => {
                    return Err(ContractError::Message(
                        MessageErrorReason::InvalidPolytoneCallback {},
                    ))
                }
            }
        }
        PolytoneCallbackMsg::CreateProxy(domain_name) => {
            // Get the domain name we are getting the polytone callback for
            let mut external_domain = EXTERNAL_DOMAINS.load(deps.storage, domain_name.clone())?;
            // Only Polytone Note is allowed to send this callback
            if info.sender
                != external_domain
                    .execution_environment
                    .get_connector_address()
            {
                return Err(ContractError::Unauthorized(
                    UnauthorizedReason::UnauthorizedCallbackSender {},
                ));
            }

            // Ensure we are working with a CosmWasm Polytone environment
            let polytone_connectors = match &mut external_domain.execution_environment {
                ExecutionEnvironment::Cosmwasm(CosmwasmBridge::Polytone(polytone_info)) => {
                    polytone_info
                }
                _ => {
                    // Unreachable
                    return Err(ContractError::Message(
                        MessageErrorReason::InvalidPolytoneCallback {},
                    ));
                }
            };

            match callback_msg.result {
                Callback::Execute(result) => {
                    // If the result is a timeout, we will update the state of the connector to timeout otherwise we will update to Created
                    if result == Err("timeout".to_string())
                        && polytone_connectors.get_polytone_proxy_state()
                            == PolytoneProxyState::PendingResponse
                    {
                        polytone_connectors.set_polytone_proxy_state(PolytoneProxyState::TimedOut)
                    } else {
                        polytone_connectors.set_polytone_proxy_state(PolytoneProxyState::Created)
                    }
                }
                Callback::FatalError(error) => {
                    // We should never run out of gas for executing an empty array of messages, but in the case we do we'll log it
                    polytone_connectors
                        .set_polytone_proxy_state(PolytoneProxyState::UnexpectedError(error))
                }
                // Should never happen because we don't do queries
                Callback::Query(_) => {
                    return Err(ContractError::Message(
                        MessageErrorReason::InvalidPolytoneCallback {},
                    ))
                }
            }
            EXTERNAL_DOMAINS.save(deps.storage, domain_name, &external_domain)?;
        }
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "process_polytone_callback"))
}

// HandleMsg is sent by the mailbox, and it contains the callback from the processor in the `body` field of the Msg
fn process_hyperlane_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    handle_msg: HandleMsg,
) -> Result<Response, ContractError> {
    // We need to check that the callback comes from a registered processor address in the external domains and that the domain ID matches
    // and obtain the encoder to decode the callback
    let encoder = EXTERNAL_DOMAINS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(Result::ok)
        .find_map(|(_, external_domain)| {
            // First check the execution environment
            match &external_domain.execution_environment {
                // It must be an EVM connected with Hyperlane execution environment
                ExecutionEnvironment::Evm(encoder, EvmBridge::Hyperlane(connector)) => {
                    // We must do the following checks:
                    // 1) The sender is the mailbox address registered for this domain
                    // 2) The domain ID must match the origin of the message
                    // 3) The sender on the External Domain must be the processor address that we registered for this domain (formatted accordingly to Hyperlane)

                    if connector.mailbox != info.sender {
                        return None;
                    }

                    // Since there is a possible remote edge case for different domain IDs to have the same processor address we must check both
                    if connector.domain_id != handle_msg.origin {
                        return None;
                    }

                    match format_address_for_hyperlane(external_domain.processor.clone()) {
                        Ok(formatted_address) => {
                            if formatted_address == handle_msg.sender {
                                Some(encoder.clone())
                            } else {
                                None
                            }
                        }
                        Err(_) => None,
                    }
                }
                // Return None for non-EVM/Hyperlane execution environments
                _ => None,
            }
        })
        .ok_or(ContractError::Unauthorized(
            UnauthorizedReason::UnauthorizedCallbackSender {},
        ))?;

    // Now that we know for sure its an authorized and valid callback, we need to decode it and update the status
    let wrapped_callback: Binary = deps.querier.query_wasm_smart(
        encoder.broker_address,
        &EncoderBrokerQueryMsg::Decode {
            encoder_version: encoder.encoder_version,
            message: ProcessorMessageToDecode::HyperlaneCallback {
                callback: handle_msg.body,
            },
        },
    )?;

    // Deserialize the callback to obtain the execution ID and the result
    let InternalAuthorizationMsg::ProcessorCallback {
        execution_id,
        execution_result,
    } = from_json(&wrapped_callback)?;

    // Update the information
    let mut callback_info = PROCESSOR_CALLBACKS.load(deps.storage, execution_id)?;
    callback_info.execution_result = execution_result;
    callback_info.last_updated_at = env.block.time.seconds();
    PROCESSOR_CALLBACKS.save(deps.storage, execution_id, &callback_info)?;

    // Reduce the current executions for the label
    CURRENT_EXECUTIONS.update(
        deps.storage,
        callback_info.label.clone(),
        |current| -> Result<u64, ContractError> {
            let count = current.unwrap_or_default();
            if count == 0 {
                Err(ContractError::CurrentExecutionsIsZero {})
            } else {
                Ok(count - 1)
            }
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "process_hyperlane_callback")
        .add_attribute("execution_id", execution_id.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&get_ownership(deps.storage)?),
        QueryMsg::SubOwners {} => to_json_binary(&get_sub_owners(deps)?),
        QueryMsg::Processor {} => to_json_binary(&get_processor(deps)?),
        QueryMsg::ExternalDomains { start_after, limit } => {
            to_json_binary(&get_external_domains(deps, start_after, limit))
        }
        QueryMsg::ExternalDomain { name } => to_json_binary(&get_external_domain(deps, name)?),
        QueryMsg::Authorizations { start_after, limit } => {
            to_json_binary(&get_authorizations(deps, start_after, limit))
        }
        QueryMsg::ProcessorCallbacks { start_after, limit } => {
            to_json_binary(&get_processor_callbacks(deps, start_after, limit))
        }
        QueryMsg::ProcessorCallback { execution_id } => {
            to_json_binary(&get_processor_callback(deps, execution_id)?)
        }
        QueryMsg::IsmSpecifier(IsmSpecifierQueryMsg::InterchainSecurityModule()) => {
            Ok(to_json_binary(&InterchainSecurityModuleResponse {
                ism: None,
            })?)
        }
    }
}

fn get_sub_owners(deps: Deps) -> StdResult<Vec<Addr>> {
    let sub_owners = SUB_OWNERS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (k, _) = item?;
            Ok(k)
        })
        .collect::<StdResult<Vec<Addr>>>()?;
    Ok(sub_owners)
}

fn get_processor(deps: Deps) -> StdResult<Addr> {
    PROCESSOR_ON_MAIN_DOMAIN.load(deps.storage)
}

fn get_external_domains(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Vec<ExternalDomain> {
    let limit = limit.unwrap_or(MAX_PAGE_LIMIT).min(MAX_PAGE_LIMIT);
    let start = start_after.map(Bound::exclusive);

    EXTERNAL_DOMAINS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .filter_map(Result::ok)
        .map(|(_, ed)| ed)
        .collect()
}

fn get_external_domain(deps: Deps, name: String) -> StdResult<ExternalDomain> {
    EXTERNAL_DOMAINS.load(deps.storage, name)
}

fn get_authorizations(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Vec<Authorization> {
    let limit = limit.unwrap_or(MAX_PAGE_LIMIT).min(MAX_PAGE_LIMIT);
    let start = start_after.map(Bound::exclusive);

    AUTHORIZATIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .filter_map(Result::ok)
        .map(|(_, auth)| auth)
        .collect()
}

fn get_processor_callbacks(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> Vec<ProcessorCallbackInfo> {
    let limit = limit.unwrap_or(MAX_PAGE_LIMIT).min(MAX_PAGE_LIMIT);
    let start = start_after.map(Bound::exclusive);

    PROCESSOR_CALLBACKS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .filter_map(Result::ok)
        .map(|(_, cb)| cb)
        .collect()
}

fn get_processor_callback(deps: Deps, execution_id: u64) -> StdResult<ProcessorCallbackInfo> {
    PROCESSOR_CALLBACKS.load(deps.storage, execution_id)
}

// Helpers

/// Asserts that the caller is the owner or a subowner
fn assert_owner_or_subowner(store: &dyn Storage, address: Addr) -> Result<(), ContractError> {
    if !is_owner(store, &address)? && !SUB_OWNERS.has(store, address) {
        return Err(ContractError::Unauthorized(
            UnauthorizedReason::NotAllowed {},
        ));
    }
    Ok(())
}

/// Returns the full denom of a tokenfactory token: factory/<contract_address>/<label>
pub fn build_tokenfactory_denom(contract_address: &str, label: &str) -> String {
    format!("factory/{}/{}", contract_address, label)
}

/// Unique ID for an execution on any processor
pub fn get_and_increase_execution_id(storage: &mut dyn Storage) -> StdResult<u64> {
    let id = EXECUTION_ID.load(storage)?;
    EXECUTION_ID.save(storage, &id.wrapping_add(1))?;
    Ok(id)
}

/// Store the pending callback
#[allow(clippy::too_many_arguments)]
pub fn store_inprocess_callback(
    storage: &mut dyn Storage,
    current_timestamp: u64,
    id: u64,
    initiator: OperationInitiator,
    domain: Domain,
    label: String,
    ttl: Option<Expiration>,
    messages: Vec<ProcessorMessage>,
) -> StdResult<()> {
    let (processor_callback_address, bridge_callback_address) = match &domain {
        Domain::Main => (PROCESSOR_ON_MAIN_DOMAIN.load(storage)?, None),
        Domain::External(domain_name) => {
            let external_domain = EXTERNAL_DOMAINS.load(storage, domain_name.clone())?;
            // The address that will send the callback for that specific processor and the address that can send a timeout
            (
                external_domain.execution_environment.get_callback_address(),
                Some(
                    external_domain
                        .execution_environment
                        .get_connector_address(),
                ),
            )
        }
    };

    let callback = ProcessorCallbackInfo {
        execution_id: id,
        created_at: current_timestamp,
        last_updated_at: current_timestamp,
        initiator,
        bridge_callback_address,
        processor_callback_address,
        domain,
        label,
        messages,
        ttl,
        execution_result: ExecutionResult::InProcess,
    };

    PROCESSOR_CALLBACKS.save(storage, id, &callback)?;

    Ok(())
}

fn create_denom_msg(sender: String, subdenom: String) -> CosmosMsg {
    let msg_create_denom = MsgCreateDenom { sender, subdenom };
    // TODO: Change to AnyMsg instead of Stargate when we can test with CW 2.0 (They are the same, just a rename)
    #[allow(deprecated)]
    CosmosMsg::Stargate {
        type_url: "/osmosis.tokenfactory.v1beta1.MsgCreateDenom".to_string(),
        value: Binary::from(msg_create_denom.to_bytes().unwrap()),
    }
}

fn mint_msg(sender: String, recipient: String, amount: u128, denom: String) -> CosmosMsg {
    let msg_mint = MsgMint {
        sender,
        amount: Some(Coin {
            denom,
            amount: amount.to_string(),
        }),
        mint_to_address: recipient,
    };
    // TODO: Change to AnyMsg instead of Stargate when we can test with CW 2.0 (They are the same, just a rename)
    #[allow(deprecated)]
    CosmosMsg::Stargate {
        type_url: "/osmosis.tokenfactory.v1beta1.MsgMint".to_string(),
        value: Binary::from(msg_mint.to_bytes().unwrap()),
    }
}

fn burn_msg(sender: String, amount: u128, denom: String) -> CosmosMsg {
    let msg_burn = MsgBurn {
        sender,
        amount: Some(Coin {
            denom,
            amount: amount.to_string(),
        }),
        burn_from_address: "".to_string(),
    };
    // TODO: Change to AnyMsg instead of Stargate when we can test with CW 2.0 (They are the same, just a rename)
    #[allow(deprecated)]
    CosmosMsg::Stargate {
        type_url: "/osmosis.tokenfactory.v1beta1.MsgBurn".to_string(),
        value: Binary::from(msg_burn.to_bytes().unwrap()),
    }
}
