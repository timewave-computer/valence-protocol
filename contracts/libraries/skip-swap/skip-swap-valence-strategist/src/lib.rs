/*
 * skip-swap-valence-strategist: Client library for orchestrating and optimizing
 * Skip Protocol swap routes. This strategic layer monitors accounts, queries optimal
 * routes from Skip API, and manages the execution of swaps through the skip-swap-valence
 * contract.
 */

pub mod chain;
pub mod config;
pub mod msg;
pub mod orchestrator;
pub mod strategist;
pub mod types;
pub mod skip;

pub use orchestrator::{Orchestrator, OrchestratorConfig};
pub use chain::ChainClient;
pub use skip::{
    // Synchronous API
    SkipApiClient, SkipApi, SkipRouteResponse, 
    create_execute_optimized_route_msg,
    
    // Asynchronous API
    SkipAsync, SkipApiClientAsync, MockSkipApiAsync, 
    SkipRouteResponseAsync, SkipApiError
};
pub use types::{RouteParameters, AssetPair};
pub use config::{StrategistConfig, load_config};
pub use strategist::{Strategist, StrategistError}; 