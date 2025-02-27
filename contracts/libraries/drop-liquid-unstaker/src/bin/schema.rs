use cosmwasm_schema::write_api;

use valence_drop_liquid_unstaker::msg::{
    FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg,
};
use valence_library_utils::msg::{ExecuteMsg, InstantiateMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<LibraryConfig>,
        execute: ExecuteMsg<FunctionMsgs, LibraryConfigUpdate>,
        query: QueryMsg,
    }
}
