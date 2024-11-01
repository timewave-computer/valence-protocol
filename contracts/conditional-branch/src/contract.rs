#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    WasmMsg, WasmQuery,
};

use crate::{
    error::ContractError,
    msg::{ComparisonOperator, ExecuteMsg, InstantiateMsg, QueryInstruction, QueryMsg},
};

#[cfg(feature = "icq_queries")]
use crate::state::ICQ_QUERIES;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", format!("{:?}", msg.owner)))
}

#[cfg(not(feature = "icq_queries"))]
type ExecuteResponse = Result<Response, ContractError>;
#[cfg(feature = "icq_queries")]
type ExecuteResponse = Result<Response<neutron_sdk::bindings::msg::NeutronMsg>, ContractError>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ExecuteResponse {
    #[cfg(feature = "icq_queries")]
    let msg_copy = msg.clone();

    match msg {
        ExecuteMsg::CompareAndBranch {
            query,
            operator,
            rhs_operand: rhs,
            true_branch,
            false_branch,
        } => {
            let lhs: Binary = match query {
                QueryInstruction::BalanceQuery { address, denom } => {
                    let balance = deps.querier.query_balance(&address, &denom)?;
                    to_json_binary(&balance.amount)?
                }
                QueryInstruction::WasmQuery {
                    contract_addr,
                    msg,
                    value_path,
                } => {
                    let response: serde_json::Value = deps
                        .querier
                        .query(&WasmQuery::Smart { contract_addr, msg }.into())?;
                    let result = value_path.iter().fold(&response, |acc, path| {
                        acc.get(path).expect("path not found")
                    });
                    to_json_binary(&result)?
                }
                #[cfg(feature = "icq_queries")]
                QueryInstruction::IcqBalanceQuery {
                    connection_id,
                    address,
                    denom,
                    update_period,
                    ..
                } => {
                    let (res, hash) = crate::icq::register_balances_query(
                        connection_id,
                        address,
                        vec![denom],
                        update_period,
                        env.block.height,
                    )
                    .map_err(|err| {
                        ContractError::ExecutionError(format!("ICQ query failed: {}", err))
                    })?;
                    ICQ_QUERIES.save(deps.storage, hash, &msg_copy)?;
                    return Ok(res);
                }
            };

            let res = evaluate_condition(operator, lhs, rhs);

            let msg: Option<WasmMsg> =
                (if res { true_branch } else { false_branch }).and_then(|msg| from_json(&msg).ok());

            match msg {
                None => {
                    if !res {
                        return Err(ContractError::ExecutionError(
                            "Condition check failed.".to_string(),
                        ));
                    }
                    Ok(Response::default())
                }
                Some(WasmMsg::Execute { .. }) => Ok(Response::new().add_message(msg.unwrap())),
                _ => Err(ContractError::ExecutionError(
                    "Only WasmMsg::Execute variant is permitted.".to_string(),
                )),
            }
        }
        ExecuteMsg::UpdateOwnership(action) => {
            let result = cw_ownable::update_ownership(
                deps.into_empty(),
                &env.block,
                &info.sender,
                action.clone(),
            )?;
            Ok(Response::default()
                .add_attribute("method", "update_ownership")
                .add_attribute("action", format!("{:?}", action))
                .add_attribute("result", format!("{:?}", result)))
        }
    }
}

fn evaluate_condition(operator: ComparisonOperator, lhs: Binary, rhs: Binary) -> bool {
    match operator {
        ComparisonOperator::Equal => lhs == rhs,
        ComparisonOperator::NotEqual => lhs != rhs,
        ComparisonOperator::LessThan => lhs < rhs,
        ComparisonOperator::LessThanOrEqual => lhs <= rhs,
        ComparisonOperator::GreaterThan => lhs > rhs,
        ComparisonOperator::GreaterThanOrEqual => lhs >= rhs,
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

// neutron uses the `sudo` entry point in their ICA/ICQ related logic
#[cfg(feature = "icq_queries")]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(
    deps: DepsMut<neutron_sdk::bindings::query::NeutronQuery>,
    env: Env,
    msg: neutron_sdk::sudo::msg::SudoMsg,
) -> StdResult<Response<neutron_sdk::bindings::msg::NeutronMsg>> {
    match msg {
        neutron_sdk::sudo::msg::SudoMsg::KVQueryResult { query_id } => {
            let registered_query =
                neutron_sdk::interchain_queries::get_registered_query(deps.as_ref(), query_id)
                    .map_err(|err| cosmwasm_std::StdError::generic_err(err.to_string()))?
                    .registered_query;
            let hash = crate::icq::get_query_hash(
                registered_query.registered_at_height,
                registered_query.keys,
            )?;
            if let ExecuteMsg::CompareAndBranch {
                query:
                    QueryInstruction::IcqBalanceQuery {
                        execution_id,
                        callback_address,
                        denom,
                        ..
                    },
                operator,
                rhs_operand,
                ..
            } = ICQ_QUERIES.load(deps.storage, hash.clone())?
            {
                ICQ_QUERIES.remove(deps.storage, hash);
                let response = neutron_sdk::interchain_queries::v047::queries::query_balance(
                    deps.as_ref(),
                    env,
                    query_id,
                )
                .map_err(|err| cosmwasm_std::StdError::generic_err(err.to_string()))?;

                let result = if let Some(balance) = response
                    .balances
                    .coins
                    .iter()
                    .find(|c| c.denom == denom)
                    .map(|bal| bal.amount.u128())
                {
                    evaluate_condition(operator, to_json_binary(&balance)?, rhs_operand)
                } else {
                    false
                };
                let exec_result = if result {
                    valence_authorization_utils::callback::ExecutionResult::Success
                } else {
                    valence_authorization_utils::callback::ExecutionResult::UnexpectedError(
                        "ICQ Balance check failed.".to_string(),
                    )
                };

                let cb_msg: cosmwasm_std::CosmosMsg<cosmwasm_std::Empty> = cosmwasm_std::CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: callback_address,
                    msg: to_json_binary(&valence_authorization_utils::msg::ExecuteMsg::InternalAuthorizationAction(
                        valence_authorization_utils::msg::InternalAuthorizationMsg::ProcessorCallback {
                            execution_id,
                            execution_result: exec_result,
                        },
                    ))
                    .unwrap(),
                    funds: vec![],
                });
                Ok(Response::default()
                    .add_message(cb_msg)
                    .change_custom::<neutron_sdk::bindings::msg::NeutronMsg>()
                    .unwrap())
            } else {
                Ok(Response::default())
            }
        }
        _ => Ok(Response::default()),
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{to_json_binary, Decimal};

    #[test]
    pub fn test_wasm_query_response_value_extraction() {
        let query_response: serde_json::Value = serde_json::json!({
            "denom_ratios": {
                "untrn": Decimal::percent(42)
            }
        });

        let value_path = ["denom_ratios".to_string(), "untrn".to_string()];
        let result = to_json_binary(value_path.iter().fold(&query_response, |acc, path| {
            acc.get(path).expect("path not found")
        }))
        .unwrap();

        assert_eq!(result, to_json_binary(&Decimal::percent(42)).unwrap());
    }
}
