/*
 * Primary entry point for the Skip Swap Valence contract.
 * Implements the core contract logic for swap execution, including:
 * - Executing basic swaps and optimized routes
 * - Validating and authorizing swap operations
 * - Contract configuration and management
 * - Query handlers for swap simulation and route parameters
 */

use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, 
    Response, StdResult, Uint128, WasmMsg, entry_point, StdError, Decimal,
};

use crate::authorization::{create_swap_message, create_swap_authorization, SwapMessage};
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, 
    RouteParametersResponse, SimulateSwapResponse,
    PendingSimulationRequestsResponse, SimulationRequestResponse,
};
use crate::state::{
    CONFIG, ROUTE_COUNT, SIMULATION_REQUESTS, SIMULATION_REQUEST_COUNT, SIMULATION_RESPONSES,
    RouteSimulationRequest,
};
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
        ExecuteMsg::RequestRouteSimulation {
            input_denom,
            input_amount,
            output_denom,
            max_slippage,
        } => execute_request_route_simulation(
            deps,
            env,
            info,
            input_denom,
            input_amount,
            output_denom,
            max_slippage,
        ),
        ExecuteMsg::SubmitRouteSimulation {
            request_id,
            route,
        } => execute_submit_route_simulation(deps, info, request_id, route),
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
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    input_denom: String,
    input_amount: Uint128,
    output_denom: String,
    max_slippage: Option<String>,
    output_address: Option<String>,
) -> Result<Response, ContractError> {
    // Load configuration
    let config = CONFIG.load(deps.storage)?;
    
    // Validate the input parameters
    let allowed_pair = config.allowed_asset_pairs.iter().any(|pair| 
        pair.input_asset == input_denom && pair.output_asset == output_denom
    );
    
    if !allowed_pair {
        return Err(ContractError::UnsupportedAssetPair {
            input_denom: input_denom.clone(),
            output_denom: output_denom.clone(),
        });
    }
    
    // Use provided slippage or default from config
    let slippage = max_slippage.as_ref().map_or_else(
        || config.max_slippage.to_string(),
        |s| s.clone()
    );
    
    // Determine output address (recipient of swap)
    let recipient = match &output_address {
        Some(addr) => deps.api.addr_validate(addr)?,
        None => info.sender.clone(),
    };
    
    // Check that we have received the correct funds
    let payment = info.funds.iter().find(|coin| coin.denom == input_denom);
    if payment.is_none() || payment.unwrap().amount != input_amount {
        return Err(ContractError::InvalidFunds {
            expected: format!("{} {}", input_amount, input_denom),
            received: info.funds.iter()
                .map(|c| format!("{} {}", c.amount, c.denom))
                .collect::<Vec<_>>()
                .join(", "),
        });
    }
    
    // Set timeout
    let timeout = env.block.time.plus_seconds(300).seconds(); // 5 minutes timeout
    
    // Create and execute a swap message to the Skip entry point
    let skip_msg = if max_slippage.is_some() || output_address.is_some() {
        // Use SwapWithParams if any optional parameters are provided
        WasmMsg::Execute {
            contract_addr: config.skip_entry_point.to_string(),
            msg: to_json_binary(&ExecuteMsg::SwapWithParams {
                input_denom: input_denom.clone(),
                input_amount,
                output_denom: output_denom.clone(),
                max_slippage: max_slippage.clone(),
                output_address: output_address.clone(),
            })?,
            funds: info.funds,
        }
    } else {
        // Use basic Swap for simplicity if no optional parameters
        WasmMsg::Execute {
            contract_addr: config.skip_entry_point.to_string(),
            msg: to_json_binary(&ExecuteMsg::Swap {
                input_denom: input_denom.clone(),
                input_amount,
                output_denom: output_denom.clone(),
            })?,
            funds: info.funds,
        }
    };
    
    // Increment route counter
    let route_id = ROUTE_COUNT.load(deps.storage)? + 1;
    ROUTE_COUNT.save(deps.storage, &route_id)?;
    
    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(skip_msg))
        .add_attribute("action", "swap")
        .add_attribute("route_id", route_id.to_string())
        .add_attribute("input_denom", input_denom)
        .add_attribute("input_amount", input_amount)
        .add_attribute("output_denom", output_denom)
        .add_attribute("slippage", slippage)
        .add_attribute("recipient", recipient)
        .add_attribute("timeout", timeout.to_string())
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
        QueryMsg::GetPendingSimulationRequests { start_after, limit } => 
            query_pending_simulation_requests(deps, start_after, limit),
        QueryMsg::GetSimulationRequest { request_id } => 
            query_simulation_request(deps, request_id),
        QueryMsg::GetSimulationResponse { request_id } => 
            query_simulation_response(deps, request_id),
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
    deps: Deps,
    input_denom: String,
    input_amount: Uint128,
    output_denom: String,
) -> StdResult<Binary> {
    // Load configuration
    let config = CONFIG.load(deps.storage)?;
    
    // Validate the input parameters
    let allowed_pair = config.allowed_asset_pairs.iter().any(|pair| 
        pair.input_asset == input_denom && pair.output_asset == output_denom
    );
    
    if !allowed_pair {
        return Err(StdError::generic_err(format!(
            "Unsupported asset pair: {} to {}",
            input_denom, output_denom
        )));
    }
    
    // Look for any existing fulfilled simulation requests that match these parameters
    let mut latest_route: Option<(u64, SkipRouteResponse)> = None;
    let mut latest_timestamp: u64 = 0;
    
    // Query simulation requests to find matching fulfilled ones
    let request_count = SIMULATION_REQUEST_COUNT.may_load(deps.storage)?.unwrap_or(0);
    
    for id in 1..=request_count {
        if let Ok(request) = SIMULATION_REQUESTS.load(deps.storage, id) {
            // Check if the request matches the parameters and is fulfilled
            if request.input_denom == input_denom && 
               request.output_denom == output_denom &&
               request.input_amount == input_amount &&
               request.fulfilled &&
               request.timestamp > latest_timestamp {
                // Check if there's a corresponding response
                if let Ok(route) = SIMULATION_RESPONSES.load(deps.storage, id) {
                    latest_timestamp = request.timestamp;
                    latest_route = Some((id, route));
                }
            }
        }
    }
    
    // If we found a recent route simulation, use that instead of the mock data
    if let Some((id, route)) = latest_route {
        // Create detailed route description
        let route_description = format!(
            "Swap {} {} for approximately {} {} via Skip Protocol with {}% max slippage (from simulation #{})",
            input_amount, 
            input_denom, 
            route.expected_output, 
            output_denom,
            route.slippage_tolerance_percent,
            id
        );
        
        let response = SimulateSwapResponse {
            expected_output: route.expected_output,
            route_description,
        };
        
        return to_json_binary(&response);
    }
    
    // If no simulation found, fall back to the mock implementation
    let expected_output = request_strategist_simulation(
        deps,
        &config.strategist_address, 
        &input_denom,
        input_amount,
        &output_denom,
        config.max_slippage,
    )?;
    
    // Create detailed route description
    let route_description = format!(
        "Swap {} {} for approximately {} {} via Skip Protocol with {}% max slippage (simulated)",
        input_amount, 
        input_denom, 
        expected_output, 
        output_denom,
        config.max_slippage
    );
    
    let response = SimulateSwapResponse {
        expected_output,
        route_description,
    };
    
    to_json_binary(&response)
}

/// Request swap simulation from the strategist contract
/// 
/// In a production environment, this would make a cross-contract query to the strategist
/// to get the simulated swap output based on current market conditions.
/// 
/// The expected flow is:
/// 1. Contract sends a SimulateSwap query to the strategist
/// 2. Strategist queries current market prices/routes from Skip API
/// 3. Strategist calculates expected output with slippage
/// 4. Strategist returns result to this contract
/// 
/// This allows the strategist to maintain price feeds and optimize routes
/// while keeping the contract itself simple.
fn request_strategist_simulation(
    _deps: Deps,
    _strategist_addr: &cosmwasm_std::Addr,
    input_denom: &str,
    input_amount: Uint128,
    output_denom: &str,
    slippage: Decimal,
) -> StdResult<Uint128> {
    // For testing, use a simplified simulation
    // In production, this would use deps.querier.query_wasm_smart to call the strategist
    
    // Example of how the query would be structured:
    // ```
    // #[derive(Serialize, Deserialize)]
    // enum StrategistQueryMsg {
    //     SimulateSwap {
    //         input_denom: String,
    //         input_amount: Uint128,
    //         output_denom: String,
    //         slippage: Decimal,
    //     }
    // }
    //
    // #[derive(Serialize, Deserialize)]
    // struct SimulationResult {
    //     expected_output: Uint128,
    //     route_description: Option<String>,
    //     swap_operations: Option<Vec<SwapOperation>>,
    // }
    //
    // let query_msg = to_json_binary(&StrategistQueryMsg::SimulateSwap {
    //     input_denom: input_denom.to_string(),
    //     input_amount,
    //     output_denom: output_denom.to_string(),
    //     slippage,
    // })?;
    // 
    // let simulation_result: SimulationResult = deps.querier.query_wasm_smart(
    //     strategist_addr.clone(),
    //     &query_msg
    // )?;
    // 
    // Ok(simulation_result.expected_output)
    // ```
    
    // Mock implementation for testing
    let exchange_rate = match (input_denom, output_denom) {
        ("uatom", "uusdc") => Decimal::from_ratio(30u128, 1u128),  // 1 ATOM = 30 USDC
        ("uusdc", "uatom") => Decimal::from_ratio(1u128, 30u128),  // 1 USDC = 0.033 ATOM
        ("uosmo", "uatom") => Decimal::from_ratio(2u128, 1u128),   // 1 OSMO = 2 ATOM
        ("uatom", "uosmo") => Decimal::from_ratio(1u128, 2u128),   // 1 ATOM = 0.5 OSMO
        ("ujuno", "uatom") => Decimal::from_ratio(1u128, 5u128),   // 1 JUNO = 0.2 ATOM
        ("uatom", "ujuno") => Decimal::from_ratio(5u128, 1u128),   // 1 ATOM = 5 JUNO
        _ => return Err(StdError::generic_err(format!(
            "Exchange rate not available for pair: {} to {}",
            input_denom, output_denom
        ))),
    };
    
    // Apply slippage to the exchange rate
    let slippage_factor = Decimal::one() - slippage;
    
    // Calculate expected output with slippage
    let raw_output = input_amount.mul_ceil(exchange_rate);
    let expected_output = raw_output.mul_floor(slippage_factor);
    
    // Ensure we're always getting a whole number of tokens
    Ok(expected_output)
}

/// Request a route simulation to be fulfilled by the strategist
fn execute_request_route_simulation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    input_denom: String,
    input_amount: Uint128,
    output_denom: String,
    max_slippage: Option<String>,
) -> Result<Response, ContractError> {
    // Load configuration
    let config = CONFIG.load(deps.storage)?;
    
    // Validate the input parameters
    let allowed_pair = config.allowed_asset_pairs.iter().any(|pair| 
        pair.input_asset == input_denom && pair.output_asset == output_denom
    );
    
    if !allowed_pair {
        return Err(ContractError::UnsupportedAssetPair {
            input_denom: input_denom.clone(),
            output_denom: output_denom.clone(),
        });
    }
    
    // Use provided slippage or default from config
    let slippage = max_slippage
        .unwrap_or_else(|| config.max_slippage.to_string());
    
    // Get the next request ID
    let request_id = SIMULATION_REQUEST_COUNT
        .may_load(deps.storage)?
        .unwrap_or(0) + 1;
    
    // Create the simulation request
    let request = RouteSimulationRequest {
        request_id,
        requester: info.sender.clone(),
        input_denom: input_denom.clone(),
        input_amount,
        output_denom: output_denom.clone(),
        max_slippage: slippage.clone(),
        timestamp: env.block.time.seconds(),
        fulfilled: false,
    };
    
    // Save the request to storage
    SIMULATION_REQUESTS.save(deps.storage, request_id, &request)?;
    SIMULATION_REQUEST_COUNT.save(deps.storage, &request_id)?;
    
    Ok(Response::new()
        .add_attribute("action", "request_route_simulation")
        .add_attribute("request_id", request_id.to_string())
        .add_attribute("input_denom", input_denom)
        .add_attribute("input_amount", input_amount)
        .add_attribute("output_denom", output_denom)
        .add_attribute("max_slippage", slippage)
        .add_attribute("requester", info.sender))
}

/// Submit a route simulation result (strategist only)
fn execute_submit_route_simulation(
    deps: DepsMut,
    info: MessageInfo,
    request_id: u64,
    route: SkipRouteResponse,
) -> Result<Response, ContractError> {
    // Load configuration
    let config = CONFIG.load(deps.storage)?;
    
    // Only the strategist can submit route simulations
    if info.sender != config.strategist_address {
        return Err(ContractError::Unauthorized { 
            msg: "Only the strategist can submit route simulations".to_string() 
        });
    }
    
    // Load the simulation request
    let mut request = SIMULATION_REQUESTS
        .may_load(deps.storage, request_id)?
        .ok_or_else(|| ContractError::NotFound {
            msg: format!("Simulation request with ID {} not found", request_id)
        })?;
    
    // Check if the request is already fulfilled
    if request.fulfilled {
        return Err(ContractError::InvalidRequest {
            msg: format!("Simulation request {} is already fulfilled", request_id)
        });
    }
    
    // Validate that the route matches the request
    if route.source_asset_denom != request.input_denom || 
       route.dest_asset_denom != request.output_denom ||
       route.amount != request.input_amount {
        return Err(ContractError::InvalidRoute {
            msg: "Route parameters do not match the simulation request".to_string()
        });
    }
    
    // Mark the request as fulfilled
    request.fulfilled = true;
    SIMULATION_REQUESTS.save(deps.storage, request_id, &request)?;
    
    // Save the route response
    SIMULATION_RESPONSES.save(deps.storage, request_id, &route)?;
    
    Ok(Response::new()
        .add_attribute("action", "submit_route_simulation")
        .add_attribute("request_id", request_id.to_string())
        .add_attribute("expected_output", route.expected_output)
        .add_attribute("slippage_tolerance", route.slippage_tolerance_percent.to_string()))
}

/// Query pending simulation requests
fn query_pending_simulation_requests(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let limit = limit.unwrap_or(10) as usize;
    let start = start_after.unwrap_or(0) + 1; // Start after the given ID
    
    let request_count = SIMULATION_REQUEST_COUNT.may_load(deps.storage)?.unwrap_or(0);
    let mut requests = Vec::new();
    
    for id in start..=request_count {
        if requests.len() >= limit {
            break;
        }
        
        if let Ok(request) = SIMULATION_REQUESTS.load(deps.storage, id) {
            // Only include unfulfilled requests
            if !request.fulfilled {
                requests.push(SimulationRequestResponse {
                    request_id: request.request_id,
                    requester: request.requester.to_string(),
                    input_denom: request.input_denom,
                    input_amount: request.input_amount,
                    output_denom: request.output_denom,
                    max_slippage: request.max_slippage,
                    timestamp: request.timestamp,
                    fulfilled: false,
                });
            }
        }
    }
    
    to_json_binary(&PendingSimulationRequestsResponse { requests })
}

/// Query a specific simulation request
fn query_simulation_request(
    deps: Deps,
    request_id: u64,
) -> StdResult<Binary> {
    let request = SIMULATION_REQUESTS.load(deps.storage, request_id)?;
    
    let response = SimulationRequestResponse {
        request_id: request.request_id,
        requester: request.requester.to_string(),
        input_denom: request.input_denom,
        input_amount: request.input_amount,
        output_denom: request.output_denom,
        max_slippage: request.max_slippage,
        timestamp: request.timestamp,
        fulfilled: request.fulfilled,
    };
    
    to_json_binary(&response)
}

/// Query a simulation response
fn query_simulation_response(
    deps: Deps,
    request_id: u64,
) -> StdResult<Binary> {
    let route = SIMULATION_RESPONSES.load(deps.storage, request_id)?;
    to_json_binary(&route)
}
