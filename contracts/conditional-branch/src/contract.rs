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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
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
            };

            let res = match operator {
                ComparisonOperator::Equal => lhs == rhs,
                ComparisonOperator::NotEqual => lhs != rhs,
                ComparisonOperator::LessThan => lhs < rhs,
                ComparisonOperator::LessThanOrEqual => lhs <= rhs,
                ComparisonOperator::GreaterThan => lhs > rhs,
                ComparisonOperator::GreaterThanOrEqual => lhs >= rhs,
            };

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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
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
