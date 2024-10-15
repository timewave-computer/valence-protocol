use cosmwasm_schema::write_api;

use valence_reverse_splitter_service::msg::{
    ActionMsgs, ServiceConfigUpdate, QueryMsg, ServiceConfig,
};
use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<ServiceConfig>,
        execute: ExecuteMsg<ActionMsgs, ServiceConfigUpdate>,
        query: QueryMsg,
    }
}
