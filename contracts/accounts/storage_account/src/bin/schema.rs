use cosmwasm_schema::write_api;

use valence_account_utils::msg::InstantiateMsg;
use valence_storage_account::msg::{ExecuteMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}
