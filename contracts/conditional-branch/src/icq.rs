use cosmwasm_std::{to_json_vec, Response, StdResult};
use neutron_sdk::{
    bindings::{msg::NeutronMsg, types::KVKey},
    interchain_queries::{
        v045::new_register_balances_query_msg, v047::helpers::create_balances_query_keys,
    },
    NeutronResult,
};
use sha2::{Digest, Sha256};

pub fn register_balances_query(
    connection_id: String,
    addr: String,
    mut denoms: Vec<String>,
    update_period: u64,
    block_height: u64,
) -> NeutronResult<(Response<NeutronMsg>, String)> {
    denoms.sort();
    let kv_keys = create_balances_query_keys(addr.clone(), denoms.clone())?;
    let hash = get_query_hash(block_height, kv_keys)?;
    let msg = new_register_balances_query_msg(connection_id, addr, denoms, update_period)?;

    Ok((Response::new().add_message(msg), hash))
}

pub fn get_query_hash(block_height: u64, kv_keys: Vec<KVKey>) -> StdResult<String> {
    let mut buffer = block_height.to_be_bytes().to_vec();
    buffer.extend_from_slice(&to_json_vec(&kv_keys)?);
    let digest = Sha256::digest(&buffer);
    Ok(hex::encode(digest))
}
