use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, 
    Response, StdResult, Uint128, WasmMsg, entry_point,
};

use crate::authorization::{create_swap_message, create_swap_authorization, SwapMessage};
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, 
    RouteParametersResponse, SimulateSwapResponse,
};
use crate::state::{CONFIG, ROUTE_COUNT};
use crate::types::{Config, SkipRouteResponse};
use crate::validation::{validate_optimized_route, create_skip_swap_authorization};
use valence_authorization_utils::authorization::AuthorizationInfo;

// Define our own wrapper for authorization messages
#[derive(serde::Serialize, serde::Deserialize)]
struct PermissionedMsg {
    pub create_authorizations: CreateAuthorizationsMsg,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CreateAuthorizationsMsg {
    pub authorizations: Vec<AuthorizationInfo>,
}

#[derive(serde::Serialize, serde::Deserialize)]
enum AuthExecuteMsg {
    PermissionedAction(PermissionedMsg),
    SendMsgs { 
        label: String, 
        messages: Vec<SwapMessage>,
        ttl: Option<u64>,
    },
}

/// Initialize the contract with configuration
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Set the owner to the message sender
    let mut config = msg.config;
    config.owner = info.sender.clone();
    
    // Store the configuration
    CONFIG.save(deps.storage, &config)?;
    
    // Initialize route counter
    ROUTE_COUNT.save(deps.storage, &0u64)?;
    
    // If the authorization contract is configured, create the authorization
    let mut messages = vec![];
    if let Some(auth_contract) = &config.authorization_contract {
        if config.use_authorization_contract {
            // Extract the asset pairs from the config
            let asset_pairs: Vec<(String, String)> = config.allowed_asset_pairs
                .iter()
                .map(|pair| (pair.input_asset.clone(), pair.output_asset.clone()))
                .collect();
            
            // Create the authorization
            let auth_info = create_swap_authorization(
                &config.strategist_address,
                asset_pairs,
                config.allowed_venues.clone(),
                config.max_slippage,
            );
            
            // Create a message to send to the authorization contract
            let create_auth_msg = WasmMsg::Execute {
                contract_addr: auth_contract.to_string(),
                msg: to_json_binary(&AuthExecuteMsg::PermissionedAction(
                    PermissionedMsg {
                        create_authorizations: CreateAuthorizationsMsg { 
                            authorizations: vec![auth_info] 
                        }
                    }
                ))?,
                funds: vec![],
            };
            
            messages.push(CosmosMsg::Wasm(create_auth_msg));
        }
    }
    
    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

/// Execute the contract
#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Swap {
            input_denom,
            input_amount,
            output_denom,
        } => execute_swap(deps, env, info, input_denom, input_amount, output_denom, None, None),
        ExecuteMsg::SwapWithParams {
            input_denom,
            input_amount,
            output_denom,
            max_slippage,
            output_address,
        } => execute_swap(
            deps,
            env,
            info,
            input_denom,
            input_amount,
            output_denom,
            max_slippage,
            output_address,
        ),
        ExecuteMsg::ExecuteOptimizedRoute {
            input_denom,
            input_amount,
            output_denom,
            min_output_amount,
            route,
            timeout_timestamp,
            swap_venue,
        } => execute_optimized_route(
            deps,
            env,
            info,
            input_denom,
            input_amount,
            output_denom,
            min_output_amount,
            route,
            timeout_timestamp,
            swap_venue,
        ),
        ExecuteMsg::UpdateConfig { config } => execute_update_config(deps, info, config),
        ExecuteMsg::CreateSkipSwapAuthorization {} => execute_create_skip_swap_authorization(deps, info),
    }
}

/// Creates the authorization for skip swap in the Valence authorization contract
fn execute_create_skip_swap_authorization(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Load config and verify sender is the owner
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized { 
            msg: "Only the contract owner can create authorizations".to_string() 
        });
    }
    
    // Check if we have an authorization contract configured
    let auth_contract = match &config.authorization_contract {
        Some(addr) => addr.clone(),
        None => return Err(ContractError::Unauthorized { 
            msg: "No authorization contract configured".to_string() 
        }),
    };
    
    // Create the authorization
    create_skip_swap_authorization(&config)?;
    
    // Extract the asset pairs from the config
    let asset_pairs: Vec<(String, String)> = config.allowed_asset_pairs
        .iter()
        .map(|pair| (pair.input_asset.clone(), pair.output_asset.clone()))
        .collect();
    
    // Create the authorization
    let auth_info = create_swap_authorization(
        &config.strategist_address,
        asset_pairs,
        config.allowed_venues.clone(),
        config.max_slippage,
    );
    
    // Create a message to send to the authorization contract
    let create_auth_msg = WasmMsg::Execute {
        contract_addr: auth_contract.to_string(),
        msg: to_json_binary(&AuthExecuteMsg::PermissionedAction(
            PermissionedMsg {
                create_authorizations: CreateAuthorizationsMsg { 
                    authorizations: vec![auth_info] 
                }
            }
        ))?,
        funds: vec![],
    };
    
    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(create_auth_msg))
        .add_attribute("action", "create_skip_swap_authorization"))
}

/// Execute a swap with default parameters
fn execute_swap(
    _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    input_denom: String,
    input_amount: Uint128,
    output_denom: String,
    _max_slippage: Option<String>,
    _output_address: Option<String>,
) -> Result<Response, ContractError> {
    // This is a placeholder that would typically:
    // 1. Load config
    // 2. Validate the input parameters
    // 3. Create and execute a swap message to the Skip entry point
    
    // For now, we'll just return a success response
    Ok(Response::new()
        .add_attribute("action", "swap")
        .add_attribute("input_denom", input_denom)
        .add_attribute("input_amount", input_amount)
        .add_attribute("output_denom", output_denom)
        .add_attribute("sender", info.sender))
}

/// Execute an optimized route
fn execute_optimized_route(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    input_denom: String,
    input_amount: Uint128,
    output_denom: String,
    min_output_amount: Uint128,
    route: SkipRouteResponse,
    timeout_timestamp: Option<u64>,
    swap_venue: Option<String>,
) -> Result<Response, ContractError> {
    // Load contract configuration
    let config = CONFIG.load(deps.storage)?;
    
    // Validate the route and parameters
    validate_optimized_route(
        deps.as_ref(), 
        &config, 
        &info.sender, 
        &input_denom, 
        &output_denom, 
        &route
    )?;
    
    // Check that the expected output is at least the minimum
    if route.expected_output < min_output_amount {
        return Err(ContractError::InvalidOutputAmount {
            min_output_amount: min_output_amount.to_string(),
            expected_output: route.expected_output.to_string(),
        });
    }
    
    // Set timeout timestamp if not provided
    let timeout = timeout_timestamp.unwrap_or_else(|| {
        deps.api.debug("No timeout provided, using default");
        env.block.time.plus_seconds(300).seconds() // 5 minutes timeout
    });
    
    // Determine whether to use the authorization contract or direct execution
    let messages = if config.use_authorization_contract && config.authorization_contract.is_some() {
        // Use the authorization contract
        let auth_contract = config.authorization_contract.as_ref().unwrap().clone();
        
        // Create the swap message using the processor message format
        let default_venue = route.operations.first()
            .and_then(|op| op.swap_venue.clone())
            .unwrap_or_else(|| "default".to_string());
        
        let venue = swap_venue.unwrap_or(default_venue);
        
        let processor_msg = create_swap_message(
            &config.skip_entry_point,
            &input_denom,
            input_amount.u128(),
            &output_denom,
            min_output_amount.u128(),
            route.slippage_tolerance_percent,
            &venue,
        );
        
        // Create the authorization execution message
        let auth_msg = AuthExecuteMsg::SendMsgs {
            label: config.swap_authorization_label.clone(),
            messages: vec![processor_msg],
            ttl: None, // No expiration for now
        };
        
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: auth_contract.to_string(),
            msg: to_json_binary(&auth_msg)?,
            funds: vec![], // No funds needed for this message
        })]
    } else {
        // Direct execution
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.skip_entry_point.to_string(),
            msg: to_json_binary(&ExecuteMsg::ExecuteOptimizedRoute {
                input_denom: input_denom.clone(),
                input_amount,
                output_denom: output_denom.clone(),
                min_output_amount,
                route: route.clone(),
                timeout_timestamp: Some(timeout),
                swap_venue,
            })?,
            funds: vec![],
        })]
    };
    
    // Increment route counter
    let route_id = ROUTE_COUNT.load(deps.storage)? + 1;
    ROUTE_COUNT.save(deps.storage, &route_id)?;
    
    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "execute_optimized_route")
        .add_attribute("route_id", route_id.to_string())
        .add_attribute("input_denom", input_denom)
        .add_attribute("input_amount", input_amount)
        .add_attribute("output_denom", output_denom)
        .add_attribute("min_output_amount", min_output_amount)
        .add_attribute("sender", info.sender))
}

/// Update the contract configuration
fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    mut config: Config,
) -> Result<Response, ContractError> {
    // Only the current owner can update the config
    let current_config = CONFIG.load(deps.storage)?;
    if info.sender != current_config.owner {
        return Err(ContractError::Unauthorized { 
            msg: "Only the contract owner can update the configuration".to_string() 
        });
    }
    
    // Prevent owner from being changed
    config.owner = current_config.owner;
    
    // Save the new configuration
    CONFIG.save(deps.storage, &config)?;
    
    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("owner", config.owner)
        .add_attribute("strategist", config.strategist_address))
}

/// Query the contract
#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => query_config(deps),
        QueryMsg::GetRouteParameters { token } => query_route_parameters(deps, token),
        QueryMsg::SimulateSwap {
            input_denom,
            input_amount,
            output_denom,
        } => query_simulate_swap(deps, input_denom, input_amount, output_denom),
    }
}

/// Query the contract configuration
fn query_config(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    
    let response = ConfigResponse {
        owner: config.owner.to_string(),
        skip_entry_point: config.skip_entry_point.to_string(),
        strategist_address: config.strategist_address.to_string(),
        allowed_asset_pairs: config.allowed_asset_pairs,
        allowed_venues: config.allowed_venues,
        max_slippage: config.max_slippage.to_string(),
        authorization_contract: config.authorization_contract.map(|addr| addr.to_string()),
        use_authorization_contract: config.use_authorization_contract,
        swap_authorization_label: config.swap_authorization_label,
    };
    
    to_json_binary(&response)
}

/// Query route parameters for a specific token
fn query_route_parameters(deps: Deps, token: String) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    
    // Filter asset pairs that have the token as input
    let relevant_pairs = config
        .allowed_asset_pairs
        .iter()
        .filter(|pair| pair.input_asset == token)
        .cloned()
        .collect::<Vec<_>>();
    
    // Convert token destinations to a vec of tuples for easier serialization
    let destinations = config
        .token_destinations
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect::<Vec<_>>();
    
    let response = RouteParametersResponse {
        allowed_asset_pairs: relevant_pairs,
        allowed_venues: config.allowed_venues,
        max_slippage: config.max_slippage.to_string(),
        token_destinations: destinations,
    };
    
    to_json_binary(&response)
}

/// Simulate a swap and return expected output
fn query_simulate_swap(
    _deps: Deps,
    input_denom: String,
    input_amount: Uint128,
    output_denom: String,
) -> StdResult<Binary> {
    // This would typically query the Skip API for a route
    // For now, we'll return a dummy response
    
    let response = SimulateSwapResponse {
        expected_output: input_amount, // Just a placeholder
        route_description: format!(
            "Swap {} {} for {} via Skip Protocol",
            input_amount, input_denom, output_denom
        ),
    };
    
    to_json_binary(&response)
}
