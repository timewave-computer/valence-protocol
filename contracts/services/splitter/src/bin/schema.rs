use cosmwasm_schema::write_api;

use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};
use valence_splitter_service::msg::{ActionMsgs, ServiceConfigUpdate, QueryMsg, ServiceConfig};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<ServiceConfig>,
        execute: ExecuteMsg<ActionMsgs, ServiceConfigUpdate>,
        query: QueryMsg,
    }
}
