use cosmwasm_schema::write_api;
use cosmwasm_std::Empty;
use valence_encoder_utils::msg::QueryMsg;

fn main() {
    write_api! {
        instantiate: Empty,
        execute: Empty,
        query: QueryMsg,
    }
}
