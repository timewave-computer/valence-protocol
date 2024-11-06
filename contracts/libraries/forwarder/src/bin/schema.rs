use cosmwasm_schema::write_api;

use valence_forwarder_library::msg::{ActionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};
use valence_library_utils::msg::{ExecuteMsg, InstantiateMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<LibraryConfig>,
        execute: ExecuteMsg<ActionMsgs,LibraryConfigUpdate>,
        query: QueryMsg,
    }
}
