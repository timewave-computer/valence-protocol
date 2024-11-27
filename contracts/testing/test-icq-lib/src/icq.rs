use cosmos_sdk_proto::{
    cosmos::{
        bank::v1beta1::MsgSend,
        tx::v1beta1::{TxBody, TxRaw},
    },
    prost::Message,
};
use cosmwasm_std::{to_json_string, Binary, DepsMut, Env, Response};
use neutron_sdk::{
    bindings::{
        msg::NeutronMsg,
        query::NeutronQuery,
        types::{Height, KVKey, StorageValue},
    },
    interchain_queries::{
        get_registered_query,
        helpers::decode_and_convert,
        query_kv_result,
        types::{KVReconstruct, QueryPayload},
        v045::{
            new_register_balances_query_msg, new_register_distribution_fee_pool_query_msg,
            new_register_transfers_query_msg,
        },
        v047::{
            helpers::create_account_denom_balance_key,
            register_queries::new_register_delegator_delegations_query_msg,
            types::{BANK_STORE_KEY, DISTRIBUTION_STORE_KEY, SUPPLY_PREFIX},
        },
    },
    NeutronResult,
};

use osmosis_std::{shim::Any, types::osmosis::gamm::v1beta1::Pool as GammPool};

use cosmwasm_std::{StdError, StdResult};

use neutron_sdk::bindings::query::QueryRegisteredQueryResponse;
use neutron_sdk::interchain_queries::v047::types::{COSMOS_SDK_TRANSFER_MSG_URL, RECIPIENT_FIELD};

use neutron_sdk::interchain_queries::types::{
    TransactionFilterItem, TransactionFilterOp, TransactionFilterValue,
};
use prost::Message as _;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json_wasm;

use crate::state::{Transfer, CATCHALL, RECIPIENT_TXS, TRANSFERS};

const MAX_ALLOWED_MESSAGES: usize = 20;

pub fn register_balances_query(
    connection_id: String,
    addr: String,
    denoms: Vec<String>,
    update_period: u64,
) -> NeutronResult<Response<NeutronMsg>> {
    let msg = new_register_balances_query_msg(connection_id, addr, denoms, update_period)?;

    Ok(Response::new().add_message(msg))
}

/// sudo_check_tx_query_result is an example callback for transaction query results that stores the
/// deposits received as a result on the registered query in the contract's state.
pub fn sudo_tx_query_result(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    query_id: u64,
    _height: Height,
    data: Binary,
) -> StdResult<Response<NeutronMsg>> {
    // Decode the transaction data
    let tx: TxRaw = TxRaw::decode(data.as_slice())
        .map_err(|_| StdError::generic_err("sudo_tx_query_result failed to decode tx_raw"))?;
    let body: TxBody = TxBody::decode(tx.body_bytes.as_slice())
        .map_err(|_| StdError::generic_err("sudo_tx_query_result failed to decode tx_body"))?;

    // Get the registered query by ID and retrieve the raw query string
    let registered_query: QueryRegisteredQueryResponse =
        get_registered_query(deps.as_ref(), query_id).map_err(|_| {
            StdError::generic_err("sudo_tx_query_result failed to get registered query response")
        })?;
    let transactions_filter = registered_query.registered_query.transactions_filter;

    #[allow(clippy::match_single_binding)]
    match registered_query.registered_query.query_type {
        _ => {
            // For transfer queries, query data looks like `[{"field:"transfer.recipient", "op":"eq", "value":"some_address"}]`
            let query_data: Vec<TransactionFilterItem> =
                serde_json_wasm::from_str(transactions_filter.as_str()).map_err(|_| {
                    StdError::generic_err("sudo_tx_query_result failed to parse tx query type")
                })?;

            let recipient = query_data
                .iter()
                .find(|x| x.field == RECIPIENT_FIELD && x.op == TransactionFilterOp::Eq)
                .map(|x| match &x.value {
                    TransactionFilterValue::String(v) => v.as_str(),
                    _ => "",
                })
                .unwrap_or("");

            let deposits = recipient_deposits_from_tx_body(body, recipient).map_err(|_| {
                StdError::generic_err(
                    "sudo_tx_query_result failed to decode recipient deposits from tx body",
                )
            })?;
            // If we didn't find a Send message with the correct recipient, return an error, and
            // this query result will be rejected by Neutron: no data will be saved to state.
            if deposits.is_empty() {
                return Err(StdError::generic_err(
                    "failed to find a matching transaction message",
                ));
            }

            let mut stored_transfers: u64 = TRANSFERS.load(deps.storage).unwrap_or_default();
            stored_transfers += deposits.len() as u64;
            TRANSFERS.save(deps.storage, &stored_transfers)?;

            let mut stored_deposits: Vec<Transfer> = RECIPIENT_TXS
                .load(deps.storage, recipient.to_string())
                .unwrap_or_default();
            stored_deposits.extend(deposits);
            RECIPIENT_TXS.save(deps.storage, recipient.to_string(), &stored_deposits)?;
            Ok(Response::new())
        }
    }
}

/// parses tx body and retrieves transactions to the given recipient.
fn recipient_deposits_from_tx_body(
    tx_body: TxBody,
    recipient: &str,
) -> NeutronResult<Vec<Transfer>> {
    let mut deposits: Vec<Transfer> = vec![];
    // Only handle up to MAX_ALLOWED_MESSAGES messages, everything else
    // will be ignored to prevent 'out of gas' conditions.
    // Note: in real contracts you will have to somehow save ignored
    // data in order to handle it later.
    for msg in tx_body.messages.iter().take(MAX_ALLOWED_MESSAGES) {
        // Skip all messages in this transaction that are not Send messages.
        if msg.type_url != *COSMOS_SDK_TRANSFER_MSG_URL.to_string() {
            continue;
        }

        // Parse a Send message and check that it has the required recipient.
        let transfer_msg: MsgSend = MsgSend::decode(msg.value.as_slice())?;
        if transfer_msg.to_address == recipient {
            for coin in transfer_msg.amount {
                deposits.push(Transfer {
                    sender: transfer_msg.from_address.clone(),
                    amount: coin.amount.clone(),
                    denom: coin.denom,
                    recipient: recipient.to_string(),
                });
            }
        }
    }
    Ok(deposits)
}

/// sudo_kv_query_result is the contract's callback for KV query results. Note that only the query
/// id is provided, so you need to read the query result from the state.
pub fn sudo_kv_query_result(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    query_id: u64,
) -> StdResult<Response<NeutronMsg>> {
    deps.api.debug(
        format!(
            "WASMDEBUG: sudo_kv_query_result received; query_id: {:?}",
            query_id,
        )
        .as_str(),
    );

    let reconstruct_result: NeutronResult<PoolWrapper> = query_kv_result(deps.as_ref(), query_id);

    match reconstruct_result {
        Ok(val) => {
            let json_str_pool = to_json_string(&val.0)?;

            CATCHALL.save(
                deps.storage,
                "json_deserialize_pool".to_string(),
                &json_str_pool,
            )?;
        }
        Err(e) => {
            CATCHALL.save(
                deps.storage,
                "json_deserialize_pool_error".to_string(),
                &e.to_string(),
            )?;
        }
    };

    Ok(Response::default())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PoolWrapper(GammPool);

impl KVReconstruct for PoolWrapper {
    fn reconstruct(kvs: &[StorageValue]) -> NeutronResult<PoolWrapper> {
        if let Some(kv) = kvs.first() {
            // need to go to Any first and then to type:
            let any_msg: Any = Any::decode(kv.value.as_slice()).unwrap();

            let osmo_pool: osmosis_std::types::osmosis::gamm::v1beta1::Pool =
                any_msg.try_into().unwrap();

            return Ok(PoolWrapper(osmo_pool));
        }

        Err(StdError::generic_err("failed to reconstruct pool".to_string()).into())
    }
}

pub fn register_transfers_query(
    connection_id: String,
    recipient: String,
    update_period: u64,
    min_height: Option<u64>,
) -> NeutronResult<Response<NeutronMsg>> {
    let msg =
        new_register_transfers_query_msg(connection_id, recipient, update_period, min_height)?;

    Ok(Response::new().add_message(msg))
}

pub fn register_kv_query(
    connection_id: String,
    update_period: u64,
    path: String,
    key: Vec<u8>,
) -> NeutronResult<Response<NeutronMsg>> {
    let kv_key = KVKey {
        path,
        key: Binary::new(key),
    };

    let msg = NeutronMsg::register_interchain_query(
        QueryPayload::KV(vec![kv_key]),
        connection_id,
        update_period,
    )?;

    Ok(Response::new().add_message(msg))
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{to_json_string, Binary};
    use neutron_sdk::bindings::types::StorageValue;
    use osmosis_std::{shim::Any, types::osmosis::gamm::v1beta1::Pool};
    use prost::Message;
    use serde_json::Value;

    #[test]
    fn try_decode_osmo_pool_from_binary() {
        let key_utf8 = "\u{2}\0\0\0\0\0\0\0\u{1}";
        let binary_key = Binary::from(key_utf8.as_bytes());

        let b64_value = "Chovb3Ntb3Npcy5nYW1tLnYxYmV0YTEuUG9vbBKGAgo/b3NtbzE5ZTJtZjdjeXdrdjd6YXVnNm5rNWY4N2QwN2Z4cmRncmxhZHZ5bWgyZ3d2NWNydm0zdm5zdWV3aGg3EAEaBgoBMBIBMCIEMTI4aCokCgtnYW1tL3Bvb2wvMRIVMTAwMDAwMDAwMDAwMDAwMDAwMDAwMl8KUQpEaWJjLzRFNDFFRDhGM0RDQUVBMTVGNEQ2QURDNkVERDdDMDRBNjc2MTYwNzM1Qzk3MTBCOTA0QjdCRjUzNTI1QjU2RDYSCTEwMDAwMDAwMBIKMTA3Mzc0MTgyNDIgChIKBXVvc21vEgkxMDAwMDAwMDASCjEwNzM3NDE4MjQ6CjIxNDc0ODM2NDg=";
        let binary_value = Binary::from_base64(b64_value).unwrap();

        let storage_value = StorageValue {
            storage_prefix: "gamm".to_string(),
            key: binary_key,
            value: binary_value,
        };

        // need to go to Any first and then to type:
        let any_msg: Any = Any::decode(storage_value.value.as_slice()).unwrap();
        assert_eq!(any_msg.type_url, "/osmosis.gamm.v1beta1.Pool");

        let osmo_pool: Pool = any_msg.try_into().unwrap();

        println!("osmo pool : {:?}", osmo_pool);

        let json_str: String = to_json_string(&osmo_pool).unwrap();
        let json_value: Value = serde_json::from_str(&json_str).unwrap();
        println!("json value: {:?}", json_value);
    }
}
