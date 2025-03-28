use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, 
    Response, StdResult, Uint128, WasmMsg, entry_point,
};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, 
    RouteParametersResponse, SimulateSwapResponse,
};
use crate::state::{CONFIG, ROUTE_COUNT};
use crate::types::{Config, SkipRouteResponse};
use crate::validation::validate_optimized_route;

/// Initialize the contract with configuration
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Store the configuration
    CONFIG.save(deps.storage, &msg.config)?;
    
    // Initialize route counter
    ROUTE_COUNT.save(deps.storage, &0u64)?;
    
    Ok(Response::new()
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
    }
}

/// Execute a swap with default parameters
fn execute_swap(
    deps: DepsMut,
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

/// Execute an optimized route from the strategist
fn execute_optimized_route(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    input_denom: String,
    input_amount: Uint128,
    output_denom: String,
    min_output_amount: Uint128,
    route: SkipRouteResponse,
    timeout_timestamp: Option<u64>,
    swap_venue: Option<String>,
) -> Result<Response, ContractError> {
    // Load config
    let config = CONFIG.load(deps.storage)?;
    
    // Validate the optimized route
    validate_optimized_route(
        &config,
        &info.sender,
        &input_denom,
        &output_denom,
        &route,
    )?;
    
    // Check that the expected output is at least the minimum
    if route.expected_output < min_output_amount {
        return Err(ContractError::InvalidOutputAmount {
            min_output_amount: min_output_amount.to_string(),
            expected_output: route.expected_output.to_string(),
        });
    }
    
    // Create a message to the Skip entry point
    // This is a placeholder for actual implementation
    let skip_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.skip_entry_point.to_string(),
        msg: to_binary(&ExecuteMsg::ExecuteOptimizedRoute {
            input_denom: input_denom.clone(),
            input_amount,
            output_denom: output_denom.clone(),
            min_output_amount,
            route: route.clone(),
            timeout_timestamp,
            swap_venue,
        })?,
        funds: vec![],
    });
    
    // Increment route counter
    let route_id = ROUTE_COUNT.load(deps.storage)? + 1;
    ROUTE_COUNT.save(deps.storage, &route_id)?;
    
    Ok(Response::new()
        .add_message(skip_msg)
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
    config: Config,
) -> Result<Response, ContractError> {
    // Only the current owner can update the config
    let current_config = CONFIG.load(deps.storage)?;
    if info.sender != current_config.strategist_address {
        return Err(ContractError::Unauthorized { 
            msg: "Only the current strategist can update the configuration".to_string() 
        });
    }
    
    // Save the new configuration
    CONFIG.save(deps.storage, &config)?;
    
    Ok(Response::new()
        .add_attribute("action", "update_config")
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
        skip_entry_point: config.skip_entry_point.to_string(),
        strategist_address: config.strategist_address.to_string(),
        allowed_asset_pairs: config.allowed_asset_pairs,
        allowed_venues: config.allowed_venues,
        max_slippage: config.max_slippage.to_string(),
    };
    
    to_binary(&response)
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
    
    to_binary(&response)
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
    
    to_binary(&response)
} 