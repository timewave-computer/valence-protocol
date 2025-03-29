/*
 * skip-swap-valence: CosmWasm smart contract that interfaces with Skip Protocol 
 * for cross-chain and DEX aggregator swaps. This contract serves as the main
 * execution interface for Skip swap operations in the Valence ecosystem.
 */

pub mod authorization;
pub mod contract;
pub mod error;
pub mod msg;
pub mod state;
pub mod types;
pub mod validation;

pub use authorization::*;
pub use contract::*;
pub use error::*;
pub use msg::*;
pub use state::*;
pub use types::*;
pub use validation::*;
pub use valence_library_base;
pub use valence_library_utils;

// Export main components for external use
pub mod prelude {
    pub use crate::authorization::*;
    pub use crate::contract::*;
    pub use crate::error::*;
    pub use crate::msg::*;
    pub use crate::state::*;
    pub use crate::types::*;
    pub use crate::validation::*;
} 