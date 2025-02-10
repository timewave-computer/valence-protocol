use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub enum Domain {
    Main,
    External(String),
}

#[cw_serde]
pub struct ExternalDomain {
    pub name: String,
    pub execution_environment: ExecutionEnvironment,
    pub processor: String,
}

#[cw_serde]
pub enum ExecutionEnvironment {
    Cosmwasm(CosmwasmBridge),
    Evm(Encoder, EvmBridge),
}

impl ExecutionEnvironment {
    pub fn get_connector_address(&self) -> Addr {
        match self {
            ExecutionEnvironment::Cosmwasm(CosmwasmBridge::Polytone(polytone_info)) => {
                polytone_info.polytone_note.address.clone()
            }
            ExecutionEnvironment::Evm(_, EvmBridge::Hyperlane(hyperlane_info)) => {
                hyperlane_info.mailbox.clone()
            }
        }
    }

    pub fn get_callback_address(&self) -> Addr {
        match self {
            ExecutionEnvironment::Cosmwasm(CosmwasmBridge::Polytone(polytone_info)) => {
                polytone_info.polytone_proxy.clone()
            }
            ExecutionEnvironment::Evm(_, EvmBridge::Hyperlane(hyperlane_info)) => {
                hyperlane_info.mailbox.clone()
            }
        }
    }
}

#[cw_serde]
pub enum CosmwasmBridge {
    Polytone(PolytoneConnectors),
}

#[cw_serde]
pub enum EvmBridge {
    Hyperlane(HyperlaneConnector),
}

#[cw_serde]
pub struct PolytoneConnectors {
    pub polytone_note: PolytoneNote,
    pub polytone_proxy: Addr,
}

impl PolytoneConnectors {
    pub fn get_polytone_proxy_state(&self) -> PolytoneProxyState {
        self.polytone_note.state.clone()
    }

    pub fn set_polytone_proxy_state(&mut self, state: PolytoneProxyState) {
        self.polytone_note.state = state;
    }
}

#[cw_serde]
pub struct PolytoneNote {
    pub address: Addr,
    pub timeout_seconds: u64,
    pub state: PolytoneProxyState,
}

#[cw_serde]
pub enum PolytoneProxyState {
    // IBC transaction was timedout
    TimedOut,
    // Waiting for IBC acknowledgement
    PendingResponse,
    // IBC transaction was successfull and thus the proxy contract was created
    Created,
    // Unexpected error occured during creation
    UnexpectedError(String),
}

#[cw_serde]
pub struct Encoder {
    pub broker_address: Addr,
    pub encoder_version: String,
}

#[cw_serde]
pub struct HyperlaneConnector {
    pub mailbox: Addr,
    pub domain_id: u32,
}
