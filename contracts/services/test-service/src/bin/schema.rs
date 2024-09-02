use cosmwasm_schema::write_api;
use valence_test_service::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

fn main() {
    write_api! {instantiate: InstantiateMsg, execute: ExecuteMsg, query: QueryMsg, migrate: MigrateMsg }
}
