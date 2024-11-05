use cosmwasm_schema::write_api;

use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};
use valence_template_service::msg::{FunctionMsgs, QueryMsg, ServiceConfig, ServiceConfigUpdate};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<ServiceConfig>,
        execute: ExecuteMsg<FunctionMsgs,ServiceConfigUpdate>,
        query: QueryMsg,
    }
}
