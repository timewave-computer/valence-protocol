use cosmwasm_schema::write_api;
use valence_library_utils::msg::{ExecuteMsg, InstantiateMsg};
use valence_magma_lper::msg::{FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<LibraryConfig>,
        execute: ExecuteMsg<FunctionMsgs,LibraryConfigUpdate>,
        query: QueryMsg,
    }
}
