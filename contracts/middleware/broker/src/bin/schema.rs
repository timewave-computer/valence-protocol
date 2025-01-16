use cosmwasm_schema::write_api;
use valence_middleware_broker::msg::{ExecuteMsg, InstantiateMsg};
use valence_middleware_utils::type_registry::types::RegistryQueryMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: RegistryQueryMsg,
    }
}
