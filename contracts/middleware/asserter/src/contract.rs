use crate::msg::{AssertionConfig, AssertionValue, ExecuteMsg, InstantiateMsg, QueryMsg};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Response, StdError,
    StdResult,
};
use cw2::set_contract_version;
use valence_middleware_utils::{
    type_registry::{
        queries::{ValencePrimitive, ValenceTypeQuery},
        types::ValenceType,
    },
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
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, MiddlewareError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Assert(assertion_config) => match evaluate_assertion(deps, assertion_config)? {
            true => to_json_binary("pass"),
            false => Err(StdError::generic_err("fail")),
        },
    }
}

/// evaluates the assertion by deserializing both comparison values into a mutual type identified
/// by `assertion_config.ty` before evaluating the predicate and returning boolean result.
fn evaluate_assertion(deps: Deps, assertion_config: AssertionConfig) -> StdResult<bool> {
    // first we fetch the values we want to compare
    let a_cmp = get_comparable_value(deps.querier, assertion_config.a)?;
    let b_cmp = get_comparable_value(deps.querier, assertion_config.b)?;

    let result = match (a_cmp, b_cmp) {
        (ValencePrimitive::Decimal(a), ValencePrimitive::Decimal(b)) => {
            assertion_config.predicate.eval(a, b)
        }
        (ValencePrimitive::Uint256(a), ValencePrimitive::Uint256(b)) => {
            assertion_config.predicate.eval(a, b)
        }
        (ValencePrimitive::Uint128(a), ValencePrimitive::Uint128(b)) => {
            assertion_config.predicate.eval(a, b)
        }
        (ValencePrimitive::Uint64(a), ValencePrimitive::Uint64(b)) => {
            assertion_config.predicate.eval(a, b)
        }
        (ValencePrimitive::String(a), ValencePrimitive::String(b)) => {
            assertion_config.predicate.eval(a, b)
        }
        // comparisons can be performed only if both values are of the same type
        _ => return Err(StdError::generic_err("value type mismatch")),
    };

    Ok(result)
}

/// prepares a value for assertion evaluation.
/// handles AssertionValue variants differently:
/// - if the value is constant -> unpack the underlying variant and return it
/// - if the value is variable -> query the storage account slot and perform the
///   Valence Type query that is configured under `query_info.query`
fn get_comparable_value(
    querier: QuerierWrapper,
    value: AssertionValue,
) -> StdResult<ValencePrimitive> {
    match value {
        AssertionValue::Variable(query_info) => {
            let valence_type: ValenceType = querier.query_wasm_smart(
                &query_info.storage_account,
                &StorageAccountQueryMsg::QueryValenceType {
                    key: query_info.storage_slot_key,
                },
            )?;
            valence_type.query(query_info.query)
        }
        AssertionValue::Constant(value_type) => Ok(value_type),
    }
}
