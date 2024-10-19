pub mod contract;

pub mod msg {
    pub use valence_generic_ibc_transfer_service::msg::{
        ActionMsgs, Config, IbcTransferAmount, QueryMsg, RemoteChainInfo, ServiceConfig,
        ServiceConfigUpdate,
    };
}

#[cfg(test)]
mod tests;
