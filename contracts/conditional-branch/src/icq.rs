use cosmwasm_std::Response;
use neutron_sdk::{
    bindings::msg::NeutronMsg, interchain_queries::v045::new_register_balances_query_msg,
    NeutronResult,
};

pub fn register_balances_query(
    connection_id: String,
    addr: String,
    denoms: Vec<String>,
    update_period: u64,
) -> NeutronResult<Response<NeutronMsg>> {
    let msg = new_register_balances_query_msg(connection_id, addr, denoms, update_period)?;
    Ok(Response::new().add_message(msg))
}
