use cosmwasm_schema::write_api;

use valence_osmosis_gamm_lper::{
    msg::{ActionsMsgs, QueryMsg},
    valence_service_integration::{OptionalServiceConfig, ServiceConfig},
};
use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg<ServiceConfig>,
        execute: ExecuteMsg<ActionsMsgs, OptionalServiceConfig>,
        query: QueryMsg,
    }
}
