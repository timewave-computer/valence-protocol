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

impl ExternalDomain {
    pub fn get_connector_address(&self) -> Addr {
        match &self.execution_environment {
            ExecutionEnvironment::Cosmwasm(CosmwasmBridge::Polytone(polytone_info)) => {
                polytone_info.polytone_note.address.clone()
            }
            ExecutionEnvironment::Evm(EvmBridge::HyperlaneMailbox(address)) => address.clone(),
        }
    }

    pub fn get_callback_address(&self) -> Addr {
        match &self.execution_environment {
            ExecutionEnvironment::Cosmwasm(CosmwasmBridge::Polytone(polytone_info)) => {
                polytone_info.polytone_proxy.clone()
            }
            ExecutionEnvironment::Evm(EvmBridge::HyperlaneMailbox(address)) => address.clone(),
        }
    }

    pub fn get_polytone_proxy_state(&self) -> Option<PolytoneProxyState> {
        match &self.execution_environment {
            ExecutionEnvironment::Cosmwasm(CosmwasmBridge::Polytone(polytone_info)) => {
                Some(polytone_info.polytone_note.state.clone())
            }
            ExecutionEnvironment::Evm(EvmBridge::HyperlaneMailbox(_)) => None,
        }
    }

    pub fn set_polytone_proxy_state(
        &mut self,
        state: PolytoneProxyState,
    ) -> Result<(), &'static str> {
        match &mut self.execution_environment {
            ExecutionEnvironment::Cosmwasm(CosmwasmBridge::Polytone(polytone_info)) => {
                polytone_info.polytone_note.state = state;
                Ok(())
            }
            ExecutionEnvironment::Evm(EvmBridge::HyperlaneMailbox(_)) => {
                Err("EVM domain does not have a polytone proxy state")
            }
        }
    }
}

#[cw_serde]
pub enum ExecutionEnvironment {
    Cosmwasm(CosmwasmBridge),
    Evm(EvmBridge),
}

#[cw_serde]
pub enum CosmwasmBridge {
    Polytone(PolytoneConnectors),
}

#[cw_serde]
pub enum EvmBridge {
    HyperlaneMailbox(Addr),
}

#[cw_serde]
pub struct PolytoneConnectors {
    pub polytone_note: PolytoneNote,
    pub polytone_proxy: Addr,
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
