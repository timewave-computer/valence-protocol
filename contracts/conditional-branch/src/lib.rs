#[allow(dead_code)]
mod contract;
mod error;
pub mod msg;

#[cfg(feature = "icq_queries")]
mod icq;

#[cfg(feature = "icq_queries")]
mod state;

#[cfg(test)]
mod tests;
