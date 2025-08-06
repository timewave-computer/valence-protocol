use cosmwasm_schema::write_api;
use cosmwasm_std::Empty;
use valence_verification_utils::verifier::{InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: Empty,
        query: QueryMsg,
    }
}
