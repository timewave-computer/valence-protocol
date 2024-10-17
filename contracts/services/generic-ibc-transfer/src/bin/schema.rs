use cosmwasm_schema::write_api;

use valence_generic_ibc_transfer_service::msg::{
    ActionMsgs, QueryMsg, ServiceConfig, ServiceConfigUpdate,
};
use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<ServiceConfig>,
        execute: ExecuteMsg<ActionMsgs, ServiceConfigUpdate>,
        query: QueryMsg,
    }
}
