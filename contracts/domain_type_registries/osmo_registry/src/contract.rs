#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use neutron_sdk::bindings::msg::NeutronMsg;
use neutron_sdk::bindings::types::KVKey;
use neutron_sdk::interchain_queries::helpers::decode_and_convert;
use neutron_sdk::interchain_queries::types::QueryType;
use neutron_sdk::interchain_queries::v047::helpers::create_account_denom_balance_key;
use neutron_sdk::interchain_queries::v047::types::BANK_STORE_KEY;
use valence_icq_lib_utils::QueryRegistrationInfoRequest;
use valence_icq_lib_utils::QueryRegistrationInfoResponse;

use crate::error::ContractError;
use crate::state::CONNECTION_ID;

use valence_icq_lib_utils::ExecuteMsg as DomainRegistryExecuteMsg;
use valence_icq_lib_utils::InstantiateMsg as DomainRegistryInstantiateMsg;
use valence_icq_lib_utils::QueryMsg as DomainRegistryQueryMsg;

// version info for migration info
const _CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const _CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const GAMM_QUERY_REGISTRATION_REPLY_ID: u64 = 31415;
const BANK_QUERY_REGISTRATION_REPLY_ID: u64 = 31416;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: DomainRegistryInstantiateMsg,
) -> Result<Response, ContractError> {
    CONNECTION_ID.save(deps.storage, &msg.connection_id)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: DomainRegistryExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: DomainRegistryQueryMsg) -> StdResult<Binary> {
    match msg {
        DomainRegistryQueryMsg::GetRegistrationConfig(request) => {
            get_registration_config(deps, request)
        }
    }
}

fn get_registration_config(deps: Deps, query: QueryRegistrationInfoRequest) -> StdResult<Binary> {
    let (kv_key, response_code_id) = match query.module.as_str() {
        "gamm" => {
            let pool_prefix_key: u8 = 0x02;
            let pool_id: u64 = 1;
            let mut pool_access_key = vec![pool_prefix_key];
            pool_access_key.extend_from_slice(&pool_id.to_be_bytes());

            (
                KVKey {
                    path: query.module.to_string(),
                    key: Binary::new(pool_access_key),
                },
                GAMM_QUERY_REGISTRATION_REPLY_ID,
            )
        }
        "bank" => {
            let addr = "osmo1hj5fveer5cjtn4wd6wstzugjfdxzl0xpwhpz63".to_string();
            let converted_addr_bytes = decode_and_convert(&addr).unwrap();
            let balance_key =
                create_account_denom_balance_key(converted_addr_bytes, "uosmo").unwrap();

            (
                KVKey {
                    path: BANK_STORE_KEY.to_string(),
                    key: Binary::new(balance_key),
                },
                BANK_QUERY_REGISTRATION_REPLY_ID,
            )
        }
        _ => return Err(ContractError::UnsupportedModule(query.module).into()),
    };

    let connection_id = CONNECTION_ID.load(deps.storage)?;

    let kv_registration_msg = NeutronMsg::RegisterInterchainQuery {
        query_type: QueryType::KV.into(),
        keys: vec![kv_key],
        transactions_filter: String::new(),
        connection_id,
        update_period: 5,
    };

    let query = QueryRegistrationInfoResponse {
        registration_msg: kv_registration_msg,
        reply_id: response_code_id,
    };

    to_json_binary(&query)
}
