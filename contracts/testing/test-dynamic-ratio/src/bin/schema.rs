use cosmwasm_schema::write_api;

use valence_service_utils::msg::DynamicRatioQueryMsg;
use valence_test_dynamic_ratio::msg::{ExecuteMsg, InstantiateMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: DynamicRatioQueryMsg,
    }
}
