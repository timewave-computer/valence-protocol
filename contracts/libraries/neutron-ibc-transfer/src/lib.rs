pub mod contract;

pub mod msg {
    pub use valence_generic_ibc_transfer_library::msg::{
        Config, FunctionMsgs, IbcTransferAmount, LibraryConfig, LibraryConfigUpdate, QueryMsg,
        RemoteChainInfo,
    };
}

#[cfg(test)]
mod tests;
