use cosmwasm_schema::write_api;
use valence_middleware_utils::type_registry::types::{
    RegistryExecuteMsg, RegistryInstantiateMsg, RegistryQueryMsg,
};

fn main() {
    write_api! {
        instantiate: RegistryInstantiateMsg,
        execute: RegistryExecuteMsg,
        query: RegistryQueryMsg,
    }
}
