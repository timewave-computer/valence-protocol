use cosmwasm_schema::write_api;

use valence_astroport_lper::msg::{ActionsMsgs, OptionalServiceConfig, QueryMsg, ServiceConfig};
use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<ServiceConfig>,
        execute: ExecuteMsg<ActionsMsgs,OptionalServiceConfig>,
        query: QueryMsg,
    }
}
