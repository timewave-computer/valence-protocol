use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order,
    Response, StdResult, Storage, Uint128,
};
use cw_ownable::{assert_owner, get_ownership, initialize_owner, is_owner};
use cw_storage_plus::Bound;
use cw_utils::Expiration;
use neutron_sdk::bindings::msg::NeutronMsg;
use valence_authorization_utils::{
    authorization::{
        Authorization, AuthorizationInfo, AuthorizationMode, AuthorizationState, PermissionType,
        Priority,
    },
    callback::{CallbackInfo, ExecutionResult, PendingCallback},
    domain::{Connector, Domain, ExternalDomain},
    msg::{
        ExecuteMsg, InstantiateMsg, Mint, OwnerMsg, PermissionedMsg, PermissionlessMsg,
        ProcessorMessage, QueryMsg,
    },
};
use valence_processor_utils::msg::{AuthorizationMsg, ExecuteMsg as ProcessorExecuteMsg};

use crate::{
    authorization::Validate,
    domain::{add_domain, create_wasm_msg_for_processor_or_proxy, get_domain},
    error::{AuthorizationErrorReason, ContractError, UnauthorizedReason},
    state::{
        AUTHORIZATIONS, CONFIRMED_CALLBACKS, CURRENT_EXECUTIONS, EXECUTION_ID, EXTERNAL_DOMAINS,
        PENDING_CALLBACK, PROCESSOR_ON_MAIN_DOMAIN, SUB_OWNERS,
    },
};

// pagination info for queries
const MAX_PAGE_LIMIT: u32 = 250;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Set up owners and initial subowners
    initialize_owner(
        deps.storage,
        deps.api,
        Some(deps.api.addr_validate(&msg.owner)?.as_str()),
    )?;

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

    // Save all external domains
    for domain in msg.external_domains {
        add_domain(deps.branch(), domain)?;
    }

    EXECUTION_ID.save(deps.storage, &0)?;

    Ok(Response::new().add_attribute("method", "instantiate_authorization"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<NeutronMsg>, ContractError> {
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
                    add_external_domains(deps, external_domains)
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
                PermissionedMsg::RemoveMsgs {
                    domain,
                    queue_position,
                    priority,
                } => remove_messages(deps, domain, queue_position, priority),
                PermissionedMsg::AddMsgs {
                    label,
                    queue_position,
                    priority,
                    messages,
                } => add_messages(deps, label, queue_position, priority, messages),
                PermissionedMsg::PauseProcessor { domain } => pause_processor(deps, domain),
                PermissionedMsg::ResumeProcessor { domain } => resume_processor(deps, domain),
            }
        }
        ExecuteMsg::PermissionlessAction(permissionless_msg) => match permissionless_msg {
            PermissionlessMsg::SendMsgs { label, messages } => {
                send_msgs(deps, env, info, label, messages)
            }
            PermissionlessMsg::Callback {
                execution_id,
                execution_result,
            } => process_callback(deps, info, execution_id, execution_result),
        },
    }
}

fn update_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: cw_ownable::Action,
) -> Result<Response<NeutronMsg>, ContractError> {
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::new().add_attributes(ownership.into_attributes()))
}

fn add_sub_owner(deps: DepsMut, sub_owner: String) -> Result<Response<NeutronMsg>, ContractError> {
    SUB_OWNERS.save(deps.storage, deps.api.addr_validate(&sub_owner)?, &Empty {})?;

    Ok(Response::new()
        .add_attribute("action", "add_sub_owner")
        .add_attribute("sub_owner", sub_owner))
}

fn remove_sub_owner(
    deps: DepsMut,
    sub_owner: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    SUB_OWNERS.remove(deps.storage, deps.api.addr_validate(&sub_owner)?);

    Ok(Response::new()
        .add_attribute("action", "remove_sub_owner")
        .add_attribute("sub_owner", sub_owner))
}

fn add_external_domains(
    mut deps: DepsMut,
    external_domains: Vec<ExternalDomain>,
) -> Result<Response<NeutronMsg>, ContractError> {
    for domain in external_domains {
        add_domain(deps.branch(), domain)?;
    }

    Ok(Response::new().add_attribute("action", "add_external_domains"))
}

fn create_authorizations(
    deps: DepsMut,
    env: Env,
    authorizations: Vec<AuthorizationInfo>,
) -> Result<Response<NeutronMsg>, ContractError> {
    let mut tokenfactory_msgs = vec![];

    for each_authorization in authorizations {
        let authorization = each_authorization.into_authorization(&env.block);

        // Check that it doesn't exist yet
        if AUTHORIZATIONS.has(deps.storage, authorization.label.clone()) {
            return Err(ContractError::Authorization(
                AuthorizationErrorReason::LabelAlreadyExists(authorization.label.clone()),
            ));
        }

        // Perform all validations on the authorization
        authorization.validate(deps.storage)?;

        // If Authorization is permissioned we need to create the tokenfactory token and mint the corresponding amounts to the addresses that can
        // execute the authorization
        if let AuthorizationMode::Permissioned(permission_type) = &authorization.mode {
            // We will always create the token if it's permissioned
            let create_token_msg = NeutronMsg::submit_create_denom(authorization.label.clone());
            tokenfactory_msgs.push(create_token_msg);

            // Full denom of the token that will be created
            let denom =
                build_tokenfactory_denom(env.contract.address.as_str(), &authorization.label);

            match permission_type {
                // If there is a call limit we will mint the amount of tokens specified in the call limit for each address and these will be burned after each correct execution
                PermissionType::WithCallLimit(call_limits) => {
                    for (addr, limit) in call_limits {
                        deps.api.addr_validate(addr.as_str())?;
                        let mint_msg = NeutronMsg::submit_mint_tokens(&denom, *limit, addr);
                        tokenfactory_msgs.push(mint_msg);
                    }
                }
                // If it has no call limit we will mint 1 token for each address which will be used to verify if they can execute the authorization via a query
                PermissionType::WithoutCallLimit(addrs) => {
                    for addr in addrs {
                        deps.api.addr_validate(addr.as_str())?;
                        let mint_msg = NeutronMsg::submit_mint_tokens(&denom, Uint128::one(), addr);
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
) -> Result<Response<NeutronMsg>, ContractError> {
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

    authorization.validate(deps.storage)?;

    AUTHORIZATIONS.save(deps.storage, label, &authorization)?;

    Ok(Response::new().add_attribute("action", "modify_authorization"))
}

fn disable_authorization(
    deps: DepsMut,
    label: String,
) -> Result<Response<NeutronMsg>, ContractError> {
    let mut authorization = AUTHORIZATIONS
        .load(deps.storage, label.clone())
        .map_err(|_| {
            ContractError::Authorization(AuthorizationErrorReason::DoesNotExist(label.clone()))
        })?;

    authorization.state = AuthorizationState::Disabled;

    AUTHORIZATIONS.save(deps.storage, label, &authorization)?;

    Ok(Response::new().add_attribute("action", "disable_authorization"))
}

fn enable_authorization(
    deps: DepsMut,
    label: String,
) -> Result<Response<NeutronMsg>, ContractError> {
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
) -> Result<Response<NeutronMsg>, ContractError> {
    let authorization = AUTHORIZATIONS
        .load(deps.storage, label.clone())
        .map_err(|_| {
            ContractError::Authorization(AuthorizationErrorReason::DoesNotExist(label.clone()))
        })?;

    let token_factory_msgs = match authorization.mode {
        AuthorizationMode::Permissioned(_) => Ok(mints.iter().map(|mint| {
            NeutronMsg::submit_mint_tokens(
                build_tokenfactory_denom(env.contract.address.as_str(), &label),
                mint.amount,
                mint.address.clone(),
            )
        })),
        AuthorizationMode::Permissionless => Err(ContractError::Authorization(
            AuthorizationErrorReason::CantMintForPermissionless {},
        )),
    }?;

    Ok(Response::new()
        .add_attribute("action", "mint_authorizations")
        .add_messages(token_factory_msgs))
}

fn pause_processor(deps: DepsMut, domain: Domain) -> Result<Response<NeutronMsg>, ContractError> {
    let execute_msg_binary = to_json_binary(&ProcessorExecuteMsg::AuthorizationModuleAction(
        AuthorizationMsg::Pause {},
    ))?;
    let wasm_msg =
        create_wasm_msg_for_processor_or_proxy(deps.storage, execute_msg_binary, &domain)?;

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "pause_processor"))
}

fn resume_processor(deps: DepsMut, domain: Domain) -> Result<Response<NeutronMsg>, ContractError> {
    let execute_msg_binary = to_json_binary(&ProcessorExecuteMsg::AuthorizationModuleAction(
        AuthorizationMsg::Resume {},
    ))?;
    let wasm_msg =
        create_wasm_msg_for_processor_or_proxy(deps.storage, execute_msg_binary, &domain)?;

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "resume_processor"))
}

fn add_messages(
    deps: DepsMut,
    label: String,
    queue_position: u64,
    priority: Priority,
    messages: Vec<ProcessorMessage>,
) -> Result<Response<NeutronMsg>, ContractError> {
    let authorization = AUTHORIZATIONS
        .load(deps.storage, label.clone())
        .map_err(|_| {
            ContractError::Authorization(AuthorizationErrorReason::DoesNotExist(label.clone()))
        })?;

    // We dont need to perform any validation because this is sent by the owner
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
    let execute_msg_binary = to_json_binary(&ProcessorExecuteMsg::AuthorizationModuleAction(
        AuthorizationMsg::AddMsgs {
            id,
            queue_position,
            msgs: messages.clone(),
            action_batch: authorization.action_batch,
            priority,
        },
    ))?;
    let wasm_msg =
        create_wasm_msg_for_processor_or_proxy(deps.storage, execute_msg_binary, &domain)?;

    store_pending_callback(deps.storage, id, domain, label, messages)?;

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "add_messages")
        .add_attribute("authorization_label", authorization.label))
}

fn remove_messages(
    deps: DepsMut,
    domain: Domain,
    queue_position: u64,
    priority: Priority,
) -> Result<Response<NeutronMsg>, ContractError> {
    let execute_msg_binary = to_json_binary(&ProcessorExecuteMsg::AuthorizationModuleAction(
        AuthorizationMsg::RemoveMsgs {
            queue_position,
            priority,
        },
    ))?;
    let wasm_msg =
        create_wasm_msg_for_processor_or_proxy(deps.storage, execute_msg_binary, &domain)?;

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "remove_messages"))
}

fn send_msgs(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    label: String,
    messages: Vec<ProcessorMessage>,
) -> Result<Response<NeutronMsg>, ContractError> {
    let authorization = AUTHORIZATIONS
        .load(deps.storage, label.clone())
        .map_err(|_| {
            ContractError::Authorization(AuthorizationErrorReason::DoesNotExist(label.clone()))
        })?;

    authorization.validate_executable(
        deps.storage,
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
    // Message for the processor
    let execute_msg_binary = to_json_binary(&ProcessorExecuteMsg::AuthorizationModuleAction(
        AuthorizationMsg::EnqueueMsgs {
            id,
            msgs: messages.clone(),
            action_batch: authorization.action_batch,
            priority: authorization.priority,
        },
    ))?;
    // We need to know if this will be sent to the processor on the main domain or to an external domain
    let wasm_msg =
        create_wasm_msg_for_processor_or_proxy(deps.storage, execute_msg_binary, &domain)?;

    store_pending_callback(deps.storage, id, domain, label, messages)?;

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "send_msgs")
        .add_attribute("authorization_label", authorization.label))
}

fn process_callback(
    deps: DepsMut,
    info: MessageInfo,
    execution_id: u64,
    execution_result: ExecutionResult,
) -> Result<Response<NeutronMsg>, ContractError> {
    let pending_callback = PENDING_CALLBACK.load(deps.storage, execution_id)?;

    // Check that the sender is the one that should send the callback
    if info.sender != pending_callback.address {
        return Err(ContractError::Unauthorized(
            UnauthorizedReason::UnauthorizedCallbackSender {},
        ));
    }
    // We'll remove the pending callback
    PENDING_CALLBACK.remove(deps.storage, execution_id);

    // Store the confirmed callback in our confirmed callback history
    // with all the information we want
    let confirmed_callback = CallbackInfo {
        execution_id,
        address: pending_callback.address,
        domain: pending_callback.domain,
        label: pending_callback.label,
        messages: pending_callback.messages,
        execution_result,
    };
    CONFIRMED_CALLBACKS.save(deps.storage, execution_id, &confirmed_callback)?;

    Ok(Response::new()
        .add_attribute("action", "process_callback")
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
        QueryMsg::Authorizations { start_after, limit } => {
            to_json_binary(&get_authorizations(deps, start_after, limit))
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
    EXECUTION_ID.save(storage, &id.checked_add(1).expect("Overflow"))?;
    Ok(id)
}

/// Store the pending callback
pub fn store_pending_callback(
    storage: &mut dyn Storage,
    id: u64,
    domain: Domain,
    label: String,
    messages: Vec<ProcessorMessage>,
) -> StdResult<()> {
    let address = match &domain {
        Domain::Main => PROCESSOR_ON_MAIN_DOMAIN.load(storage)?,
        Domain::External(domain_name) => {
            let external_domain = EXTERNAL_DOMAINS.load(storage, domain_name.clone())?;
            match external_domain.connector {
                Connector::PolytoneNote(address) => address,
            }
        }
    };

    let pending_callback = PendingCallback {
        address,
        domain,
        label,
        messages,
    };

    PENDING_CALLBACK.save(storage, id, &pending_callback)
}
