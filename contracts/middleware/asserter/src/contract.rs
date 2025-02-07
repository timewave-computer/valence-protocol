use crate::msg::{AssertionValue, ExecuteMsg, InstantiateMsg, Predicate, QueryMsg};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Response, StdError, StdResult,
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
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, MiddlewareError> {
    match msg {
        ExecuteMsg::Assert { a, predicate, b } => {
            match evaluate_assertion(deps, a, predicate, b)? {
                true => Ok(Response::default()),
                false => Err(StdError::generic_err("assertion failed").into()),
            }
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

/// evaluates the assertion by deserializing both comparison values into a mutual type identified
/// by `assertion_config.ty` before evaluating the predicate and returning boolean result.
fn evaluate_assertion(
    deps: DepsMut,
    a: AssertionValue,
    predicate: Predicate,
    b: AssertionValue,
) -> StdResult<bool> {
    // first we fetch the values we want to compare
    let a_cmp = get_comparable_value(deps.querier, a)?;
    let b_cmp = get_comparable_value(deps.querier, b)?;

    match (a_cmp, b_cmp) {
        (ValencePrimitive::Decimal(a), ValencePrimitive::Decimal(b)) => Ok(predicate.eval(a, b)),
        (ValencePrimitive::Uint256(a), ValencePrimitive::Uint256(b)) => Ok(predicate.eval(a, b)),
        (ValencePrimitive::Uint128(a), ValencePrimitive::Uint128(b)) => Ok(predicate.eval(a, b)),
        (ValencePrimitive::Uint64(a), ValencePrimitive::Uint64(b)) => Ok(predicate.eval(a, b)),
        (ValencePrimitive::String(a), ValencePrimitive::String(b)) => Ok(predicate.eval(a, b)),
        // comparisons can be performed only if both values are of the same type
        _ => Err(StdError::generic_err("variant mismatch")),
    }
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
