use cosmwasm_schema::write_api;

use service_base::msg::{ExecuteMsg, InstantiateMsg};
use valence_splitter::msg::{ActionsMsgs, OptionalServiceConfig, QueryMsg, ServiceConfig};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<ServiceConfig>,
        execute: ExecuteMsg<ActionsMsgs,OptionalServiceConfig>,
        query: QueryMsg,
    }
}
