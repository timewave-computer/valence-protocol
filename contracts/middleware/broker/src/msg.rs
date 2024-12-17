use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use valence_middleware_utils::type_registry::types::RegistryQueryMsg;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    SetRegistry { version: String, address: String },
}

#[cw_serde]
pub struct QueryMsg {
    pub registry_version: Option<String>,
    pub query: RegistryQueryMsg,
}

#[cw_serde]
pub struct TypeRegistry {
    // address of the instantiated registry
    pub registry_address: Addr,
    // semver
    pub version: String,
}
