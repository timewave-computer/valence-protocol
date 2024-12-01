use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use neutron_sdk::bindings::msg::NeutronMsg;
use neutron_sdk::interchain_queries::types::QueryType;
use valence_icq_lib_utils::QueryReconstructionRequest;
use valence_icq_lib_utils::QueryReconstructionResponse;
use valence_icq_lib_utils::QueryRegistrationInfoRequest;
use valence_icq_lib_utils::QueryRegistrationInfoResponse;

use crate::error::ContractError;
use crate::msg::OsmosisTypes;
use crate::state::CONNECTION_ID;

use valence_icq_lib_utils::ExecuteMsg as DomainRegistryExecuteMsg;
use valence_icq_lib_utils::InstantiateMsg as DomainRegistryInstantiateMsg;
use valence_icq_lib_utils::QueryMsg as DomainRegistryQueryMsg;

// version info for migration info
const _CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const _CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        DomainRegistryQueryMsg::ReconstructQuery(query_reconstruction_request) => {
            reconstruct_icq_result(query_reconstruction_request)
        }
    }
}

fn reconstruct_icq_result(query: QueryReconstructionRequest) -> StdResult<Binary> {
    let underlying_type = OsmosisTypes::from_str(&query.query_type)?;

    let reconstructed_json_value = underlying_type.reconstruct_response(&query)?;

    let resp = QueryReconstructionResponse {
        json_value: reconstructed_json_value,
    };

    to_json_binary(&resp)
}

fn get_registration_config(deps: Deps, query: QueryRegistrationInfoRequest) -> StdResult<Binary> {
    let osmo_type = OsmosisTypes::from_str(&query.module)?;

    let (kv_key, response_code_id) = osmo_type.get_registration_config(query.params)?;

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
