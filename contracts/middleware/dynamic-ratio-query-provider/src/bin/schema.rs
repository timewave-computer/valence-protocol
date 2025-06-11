use cosmwasm_schema::write_api;

use valence_dynamic_ratio_query_provider::msg::{ExecuteMsg, InstantiateMsg};
use valence_library_utils::msg::DynamicRatioQueryMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: DynamicRatioQueryMsg,
    }
}
