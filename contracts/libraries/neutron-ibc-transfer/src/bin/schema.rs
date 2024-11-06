use cosmwasm_schema::write_api;

use valence_library_utils::msg::{ExecuteMsg, InstantiateMsg};
use valence_neutron_ibc_transfer_library::msg::{
    ActionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg,
};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<LibraryConfig>,
        execute: ExecuteMsg<ActionMsgs, LibraryConfigUpdate>,
        query: QueryMsg,
    }
}
