use crate::msg::{
    AssertionConfig, AssertionValue, ExecuteMsg, InstantiateMsg, QueryMsg, ValueType,
};
use cosmwasm_schema::serde::de::DeserializeOwned;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    Response, StdError, StdResult, Uint128, Uint256, Uint64,
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
    let assertion_bool = match assertion_config.ty {
        ValueType::Decimal => {
            let a_comparable: Decimal = get_comparable_value(deps.querier, assertion_config.a)?;
            let b_comparable: Decimal = get_comparable_value(deps.querier, assertion_config.b)?;
            assertion_config.predicate.eval(a_comparable, b_comparable)
        }
        ValueType::Uint64 => {
            let a_comparable: Uint64 = get_comparable_value(deps.querier, assertion_config.a)?;
            let b_comparable: Uint64 = get_comparable_value(deps.querier, assertion_config.b)?;
            assertion_config.predicate.eval(a_comparable, b_comparable)
        }
        ValueType::Uint128 => {
            let a_comparable: Uint128 = get_comparable_value(deps.querier, assertion_config.a)?;
            let b_comparable: Uint128 = get_comparable_value(deps.querier, assertion_config.b)?;
            assertion_config.predicate.eval(a_comparable, b_comparable)
        }
        ValueType::Uint256 => {
            let a_comparable: Uint256 = get_comparable_value(deps.querier, assertion_config.a)?;
            let b_comparable: Uint256 = get_comparable_value(deps.querier, assertion_config.b)?;
            assertion_config.predicate.eval(a_comparable, b_comparable)
        }
        ValueType::String => {
            let a_comparable: String = get_comparable_value(deps.querier, assertion_config.a)?;
            let b_comparable: String = get_comparable_value(deps.querier, assertion_config.b)?;
            assertion_config.predicate.eval(a_comparable, b_comparable)
        }
    };

    Ok(assertion_bool)
}

/// prepares a value for assertion evaluation.
/// handles AssertionValue variants differently:
/// - if the value is constant -> deserialize the value into the target type `T`
/// - if the value is variable -> query the storage account slot and perform the
///   Valence Type query that is configured under `query_info.query`
fn get_comparable_value<T: PartialOrd + PartialEq + DeserializeOwned>(
    querier: QuerierWrapper,
    value: AssertionValue,
) -> StdResult<T> {
    let binary_value = match value {
        AssertionValue::Variable(query_info) => {
            let valence_type: ValenceType = querier.query_wasm_smart(
                &query_info.storage_account,
                &StorageAccountQueryMsg::QueryValenceType {
                    key: query_info.storage_slot_key,
                },
            )?;
            valence_type.query(query_info.query)?
        }
        AssertionValue::Constant(b64) => b64,
    };
    let comparable = from_json(&binary_value)?;
    Ok(comparable)
}
