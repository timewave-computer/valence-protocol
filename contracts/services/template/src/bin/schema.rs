use cosmwasm_schema::write_api;

use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};
use valence_template_service::msg::{ActionsMsgs, QueryMsg, ServiceConfig, ServiceConfigUpdate};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<ServiceConfig>,
        execute: ExecuteMsg<ActionsMsgs,ServiceConfigUpdate>,
        query: QueryMsg,
    }
}
