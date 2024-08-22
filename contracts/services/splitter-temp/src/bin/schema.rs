use cosmwasm_schema::write_api;

use service_base::msg::ExecuteMsg;
use valence_splitter::msg::{ActionsMsgs, InstantiateMsg, OptionalServiceConfig, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<ActionsMsgs,OptionalServiceConfig>,
        query: QueryMsg,
    }
}
