use std::str::FromStr;

use crate::msg::{
    AssertionConfig, AssertionValue, ExecuteMsg, InstantiateMsg, Predicate, QueryInfo, QueryMsg,
    ValueType,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    Response, StdError, StdResult,
};
use cw2::set_contract_version;
use valence_middleware_utils::{
    type_registry::{queries::ValenceTypeQuery, types::ValenceType},
    MiddlewareError,
};
use valence_storage_account::msg::QueryMsg as StorageAccountQueryMsg;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, MiddlewareError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, MiddlewareError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Assert(assertion_config) => {
            let assertion_success = evaluate_assertion(deps, assertion_config)?;
            if assertion_success {
                to_json_binary("pass")
            } else {
                Err(StdError::generic_err("Assertion failed"))
            }
        }
    }
}

fn evaluate_assertion(deps: Deps, assertion_config: AssertionConfig) -> StdResult<bool> {
    match assertion_config.ty {
        ValueType::Decimal => {
            let a_comparable = match assertion_config.a {
                AssertionValue::Constant(str) => Decimal::from_str(&str)?,
                AssertionValue::Variable(query_info) => {
                    let binary_resp = fetch_variable_assertion_value(deps.querier, query_info)?;
                    from_json(&binary_resp)?
                }
            };
            let b_comparable = match assertion_config.b {
                AssertionValue::Constant(str) => Decimal::from_str(&str)?,
                AssertionValue::Variable(query_info) => {
                    let binary_resp = fetch_variable_assertion_value(deps.querier, query_info)?;
                    from_json(&binary_resp)?
                }
            };
            println!("a_comparable: {:?}", a_comparable);
            println!("b_comparable: {:?}", b_comparable);
            match assertion_config.predicate {
                Predicate::LT => Ok(a_comparable < b_comparable),
                Predicate::LTE => Ok(a_comparable <= b_comparable),
                Predicate::EQ => Ok(a_comparable == b_comparable),
                Predicate::GT => Ok(a_comparable > b_comparable),
                Predicate::GTE => Ok(a_comparable >= b_comparable),
            }
        }
        ValueType::Uint64 => unimplemented!(),
        ValueType::Uint128 => unimplemented!(),
        ValueType::Uint256 => unimplemented!(),
        ValueType::String => unimplemented!(),
    }
}

fn fetch_variable_assertion_value(
    querier: QuerierWrapper,
    query_info: QueryInfo,
) -> StdResult<Binary> {
    let valence_type: ValenceType = querier.query_wasm_smart(
        &query_info.storage_account,
        &StorageAccountQueryMsg::QueryValenceType {
            key: query_info.storage_slot_key,
        },
    )?;
    let bin_result = valence_type.query(query_info.query)?;

    Ok(bin_result)
}
