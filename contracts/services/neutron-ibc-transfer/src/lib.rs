pub mod contract;

pub mod msg {
    pub use valence_generic_ibc_transfer_service::msg::{
        ActionMsgs, Config, IbcTransferAmount, OptionalServiceConfig, QueryMsg, RemoteChainInfo,
        ServiceConfig,
    };
}

#[cfg(test)]
mod tests;
