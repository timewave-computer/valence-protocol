pub mod contract;

pub mod msg {
    pub use valence_generic_ibc_transfer_library::msg::{
        ActionMsgs, Config, IbcTransferAmount, LibraryConfig, LibraryConfigUpdate, QueryMsg,
        RemoteChainInfo,
    };
}

#[cfg(test)]
mod tests;
