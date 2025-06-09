// Purpose: Entry point for account factory contract modules
pub mod contract;
pub mod msg;
pub mod state;

#[cfg(not(feature = "library"))]
pub use contract::{execute, instantiate, query}; 