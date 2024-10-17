use cosmwasm_schema::write_api;

use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};
use valence_splitter_service::msg::{ActionMsgs, QueryMsg, ServiceConfig, ServiceConfigUpdate};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<ServiceConfig>,
        execute: ExecuteMsg<ActionMsgs, ServiceConfigUpdate>,
        query: QueryMsg,
    }
}
