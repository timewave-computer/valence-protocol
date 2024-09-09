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
    pub connector: Connector,
    pub processor: String,
    pub callback_proxy: CallbackProxy,
}

impl ExternalDomain {
    pub fn get_connector_address(&self) -> Addr {
        match &self.connector {
            Connector::PolytoneNote { address, .. } => address.clone(),
        }
    }

    pub fn get_callback_proxy_address(&self) -> Addr {
        match &self.callback_proxy {
            CallbackProxy::PolytoneProxy(address) => address.clone(),
        }
    }

    pub fn get_connector_state(&self) -> PolytoneProxyState {
        match &self.connector {
            Connector::PolytoneNote { state, .. } => state.clone(),
        }
    }

    pub fn set_connector_state(&mut self, state: PolytoneProxyState) {
        match &mut self.connector {
            Connector::PolytoneNote {
                state: current_state,
                ..
            } => {
                *current_state = state;
            }
        }
    }
}

#[cw_serde]
pub enum ExecutionEnvironment {
    CosmWasm,
}

#[cw_serde]
pub enum Connector {
    PolytoneNote {
        address: Addr,
        timeout_seconds: u64,
        state: PolytoneProxyState,
    },
}

#[cw_serde]
pub enum PolytoneProxyState {
    // IBC transaction was timedout
    TimedOut,
    // Waiting for IBC acknowledgement
    PendingResponse,
    // IBC transaction was successfull and thus the proxy contract was created
    Created,
}

#[cw_serde]
pub enum CallbackProxy {
    PolytoneProxy(Addr),
}
