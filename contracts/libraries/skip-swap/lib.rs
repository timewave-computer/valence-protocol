/// This module provides a unified entry point for Skip Swap functionality.
/// It re-exports the main components from both the Valence Skip Swap contract
/// and the strategist implementation.

// Re-export the contract implementation for direct use
pub use skip_swap_valence as contract;

// Re-export the strategist implementation for orchestration
pub use skip_swap_valence_strategist as strategist;

/// Provides a convenient way to create a new Skip Swap strategist with default configuration
pub fn create_strategist(
    chain_client: strategist::chain::ChainClient,
    skip_api_client: impl strategist::skip::SkipApiClient,
    config: strategist::orchestrator::OrchestratorConfig,
) -> strategist::orchestrator::Orchestrator<impl strategist::skip::SkipApiClient> {
    strategist::orchestrator::Orchestrator::new(chain_client, skip_api_client, config)
}

/// Utility function to check if a route is valid according to current configuration
pub fn validate_route(
    route: &contract::types::SkipRouteResponse,
    allowed_venues: &[String],
    max_slippage: &str,
) -> Result<bool, String> {
    // Check if all operations use allowed venues
    let valid_venues = route.operations.iter().all(|op| {
        op.swap_venue.as_ref().map_or(false, |venue| allowed_venues.contains(venue))
    });
    
    if !valid_venues {
        return Err("Route contains operations with disallowed venues".to_string());
    }
    
    // Check if slippage is within allowed limit
    if let Some(route_slippage) = &route.slippage_tolerance_percent {
        let max = max_slippage.parse::<f64>().map_err(|_| "Invalid max slippage format".to_string())?;
        let route_slip = route_slippage.parse::<f64>().map_err(|_| "Invalid route slippage format".to_string())?;
        
        if route_slip > max {
            return Err(format!("Route slippage {}% exceeds maximum allowed {}%", route_slip, max));
        }
    }
    
    Ok(true)
}
