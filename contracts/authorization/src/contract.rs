use authorization_utils::{
    authorization::{
        Authorization, AuthorizationInfo, AuthorizationMode, AuthorizationState, PermissionType,
        Priority,
    },
    domain::ExternalDomain,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order,
    Response, StdResult, Storage, Uint128,
};
use cw_ownable::{assert_owner, get_ownership, initialize_owner, is_owner};
use cw_storage_plus::Bound;
use cw_utils::Expiration;
use neutron_sdk::bindings::msg::NeutronMsg;

use crate::{
    authorization::Validate,
    domain::add_domains,
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, Mint, OwnerMsg, QueryMsg, SubOwnerMsg},
    state::{AUTHORIZATIONS, EXTERNAL_DOMAINS, PROCESSOR_ON_MAIN_DOMAIN, SUB_OWNERS},
};

// pagination info for queries
const MAX_PAGE_LIMIT: u32 = 250;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Set up owners and initial subowners
    initialize_owner(
        deps.storage,
        deps.api,
        Some(
            deps.api
                .addr_validate(msg.owner.unwrap_or(info.sender).as_str())?
                .as_str(),
        ),
    )?;

    if let Some(sub_owners) = msg.sub_owners {
        for sub_owner in sub_owners {
            SUB_OWNERS.save(
                deps.storage,
                deps.api.addr_validate(sub_owner.as_str())?,
                &Empty {},
            )?;
        }
    }

    // Save processor on main domain
    PROCESSOR_ON_MAIN_DOMAIN.save(
        deps.storage,
        &deps.api.addr_validate(msg.processor.as_str())?,
    )?;

    // Save all external domains
    if let Some(external_domains) = msg.external_domains {
        add_domains(deps, external_domains)?;
    }

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
        ExecuteMsg::SubOwnerAction(sub_owner_msg) => {
            assert_owner_or_subowner(deps.storage, info.sender)?;
            match sub_owner_msg {
                SubOwnerMsg::AddExternalDomains { external_domains } => {
                    add_external_domains(deps, external_domains)
                }
                SubOwnerMsg::CreateAuthorizations { authorizations } => {
                    create_authorizations(deps, env, authorizations)
                }
                SubOwnerMsg::ModifyAuthorization {
                    label,
                    expiration,
                    max_concurrent_executions,
                    priority,
                } => modify_authorization(
                    deps,
                    label,
                    expiration,
                    max_concurrent_executions,
                    priority,
                ),
                SubOwnerMsg::DisableAuthorization { label } => disable_authorization(deps, label),
                SubOwnerMsg::EnableAuthorization { label } => enable_authorization(deps, label),
                SubOwnerMsg::MintAuthorizations { label, mints } => {
                    mint_authorizations(deps, env, label, mints)
                }
            }
        }
        ExecuteMsg::UserAction(_) => Ok(Response::default()),
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

fn add_sub_owner(deps: DepsMut, sub_owner: Addr) -> Result<Response<NeutronMsg>, ContractError> {
    SUB_OWNERS.save(
        deps.storage,
        deps.api.addr_validate(sub_owner.as_str())?,
        &Empty {},
    )?;

    Ok(Response::new()
        .add_attribute("action", "add_sub_owner")
        .add_attribute("sub_owner", sub_owner))
}

fn remove_sub_owner(deps: DepsMut, sub_owner: Addr) -> Result<Response<NeutronMsg>, ContractError> {
    SUB_OWNERS.remove(deps.storage, sub_owner.clone());

    Ok(Response::new()
        .add_attribute("action", "remove_sub_owner")
        .add_attribute("sub_owner", sub_owner))
}

fn add_external_domains(
    deps: DepsMut,
    external_domains: Vec<ExternalDomain>,
) -> Result<Response<NeutronMsg>, ContractError> {
    add_domains(deps, external_domains)?;

    Ok(Response::new().add_attribute("action", "add_external_domains"))
}

fn create_authorizations(
    deps: DepsMut,
    env: Env,
    authorizations: Vec<AuthorizationInfo>,
) -> Result<Response<NeutronMsg>, ContractError> {
    let mut tokenfactory_msgs = vec![];

    for each_authorization in authorizations {
        let authorization = Authorization::from(each_authorization);

        // Check that it doesn't exist yet
        if AUTHORIZATIONS.has(deps.storage, authorization.label.clone()) {
            return Err(ContractError::LabelAlreadyExists(
                authorization.label.clone(),
            ));
        }

        // Perform all validations on the authorization
        authorization.validate(deps.storage)?;

        // If Authorization is permissioned we need to create the tokenfactory denom and mint the corresponding amounts to the addresses that can
        // execute the authorization
        if let AuthorizationMode::Permissioned(permission_type) = &authorization.mode {
            // We will always create the denom if it's permissioned
            let create_denom_msg = NeutronMsg::submit_create_denom(authorization.label.clone());
            tokenfactory_msgs.push(create_denom_msg);

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
    expiration: Option<Expiration>,
    max_concurrent_executions: Option<u64>,
    priority: Option<Priority>,
) -> Result<Response<NeutronMsg>, ContractError> {
    let mut authorization = AUTHORIZATIONS
        .load(deps.storage, label.clone())
        .map_err(|_| ContractError::AuthorizationDoesNotExist(label.clone()))?;

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
        .map_err(|_| ContractError::AuthorizationDoesNotExist(label.clone()))?;

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
        .map_err(|_| ContractError::AuthorizationDoesNotExist(label.clone()))?;

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
        .map_err(|_| ContractError::AuthorizationDoesNotExist(label.clone()))?;

    let token_factory_msgs = match authorization.mode {
        AuthorizationMode::Permissioned(_) => Ok(mints.iter().map(|mint| {
            NeutronMsg::submit_mint_tokens(
                build_tokenfactory_denom(env.contract.address.as_str(), &label),
                mint.amount,
                mint.address.clone(),
            )
        })),
        AuthorizationMode::Permissionless => {
            Err(ContractError::CantMintForPermissionlessAuthorization {})
        }
    }?;

    Ok(Response::new()
        .add_attribute("action", "mint_authorizations")
        .add_messages(token_factory_msgs))
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
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

/// Returns the full denom of a tokenfactory token: factory/<contract_address>/<label>
pub fn build_tokenfactory_denom(contract_address: &str, label: &str) -> String {
    format!("factory/{}/{}", contract_address, label)
}
