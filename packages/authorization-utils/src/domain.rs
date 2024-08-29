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
    pub callback_proxy: Addr,
}

#[cw_serde]
pub enum ExecutionEnvironment {
    CosmWasm,
}

#[cw_serde]
pub enum Connector {
    PolytoneNote(Addr),
}
