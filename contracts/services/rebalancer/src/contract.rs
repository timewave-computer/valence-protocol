#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_service_utils::{
    error::ServiceError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{ActionMsgs, Config, QueryMsg, ServiceConfig, ServiceConfigUpdate};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<ServiceConfig>,
) -> Result<Response, ServiceError> {
    valence_service_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<ActionMsgs, ServiceConfigUpdate>,
) -> Result<Response, ServiceError> {
    valence_service_base::execute(
        deps,
        env,
        info,
        msg,
        actions::process_action,
        execute::update_config,
    )
}

mod actions {
    use std::collections::HashSet;

    use cosmwasm_std::{
        coins, to_json_binary, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg,
    };
    use valence_service_utils::error::ServiceError;

    use crate::{
        msg::{ActionMsgs, Config},
        rebalancer_custom::{
            RebalancerAccountType, RebalancerData, ServicesManagerExecuteMsg, Target,
            TargetOverrideStrategy, ValenceServices,
        },
        NTRN_DENOM,
    };

    pub fn process_action(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: ActionMsgs,
        _cfg: Config,
    ) -> Result<Response, ServiceError> {
        match msg {
            ActionMsgs::StartRebalance {
                trustee,
                pid,
                max_limit_bps,
                min_balance,
            } => {
                let config: Config = valence_service_base::load_config(deps.storage)?;

                // TODO: Change this to get the full list of targets the rebalancer supports
                let mut targets: HashSet<Target> = HashSet::new();
                config.denoms.iter().for_each(|denom| {
                    targets.insert(Target {
                        denom: denom.to_string(),
                        bps: 1,
                        min_balance: None,
                    });
                });
                // main denom - USDC
                targets.insert(Target {
                    denom: config.base_denom.clone(),
                    bps: 10000 - (config.denoms.len() as u64),
                    min_balance: Some(min_balance.u128().into()),
                });

                let rebalancer_config = RebalancerData {
                    trustee,
                    base_denom: config.base_denom,
                    targets,
                    pid,
                    max_limit_bps,
                    target_override_strategy: TargetOverrideStrategy::Proportional,
                    account_type: RebalancerAccountType::Workflow,
                };

                let register_msg = ServicesManagerExecuteMsg::RegisterToService {
                    service_name: ValenceServices::Rebalancer,
                    data: Some(to_json_binary(&rebalancer_config)?.to_vec().into()),
                };
                let rebalancer_wasm_msg = WasmMsg::Execute {
                    contract_addr: config.rebalancer_manager_addr.to_string(),
                    msg: to_json_binary(&register_msg)?,
                    funds: coins(1_000_000_u128, NTRN_DENOM),
                };

                // query the balance of the rebalancer address for NTRN
                let ntrn_balance = deps
                    .querier
                    .query_balance(config.rebalancer_account, NTRN_DENOM.to_string())?;

                // rebalancer addr must have 1 ntrn for rebalancer fee
                if ntrn_balance.amount < Uint128::from(1000000_u128) {
                    return Err(ServiceError::ExecutionError(
                        "Input address must have at least 1 NTRN".to_string(),
                    ));
                }

                Ok(Response::default().add_message(rebalancer_wasm_msg))
            }
            ActionMsgs::UpdateRebalancerConfig {
                trustee: _,
                pid: _,
                max_limit_bps: _,
            } => todo!(),
        }
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use valence_service_utils::error::ServiceError;

    use crate::msg::ServiceConfigUpdate;

    pub fn update_config(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        new_config: ServiceConfigUpdate,
    ) -> Result<(), ServiceError> {
        new_config.update_config(deps)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => {
            to_json_binary(&valence_service_base::get_ownership(deps.storage)?)
        }
        QueryMsg::GetProcessor {} => {
            to_json_binary(&valence_service_base::get_processor(deps.storage)?)
        }
        QueryMsg::GetServiceConfig {} => {
            let config: Config = valence_service_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
        QueryMsg::GetRawServiceConfig {} => {
            let raw_config: ServiceConfig = valence_service_base::load_raw_config(deps.storage)?;
            to_json_binary(&raw_config)
        }
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        coin,
        testing::{message_info, mock_dependencies, mock_dependencies_with_balances, mock_env},
        Uint128,
    };
    use valence_service_utils::{msg::InstantiateMsg, ServiceAccountType};

    use crate::{msg::ServiceConfig, rebalancer_custom::PID, NTRN_DENOM};

    use super::{execute, instantiate};

    #[test]
    fn test() {
        let deps = mock_dependencies();
        let env = mock_env();

        let addr = deps.api.addr_make("sender");
        let owner = deps.api.addr_make("owner");
        let processor = deps.api.addr_make("processor");
        let reb_acc = deps.api.addr_make("reb_acc");
        let reb_manager = deps.api.addr_make("reb_manager");
        let info_processor = message_info(&processor.clone(), &[]);
        let info = message_info(&addr.clone(), &[]);

        let mut deps = mock_dependencies_with_balances(&[(
            reb_acc.as_str(),
            &[coin(1_000_000_u128, NTRN_DENOM.to_string())],
        )]);

        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                owner: owner.to_string(),
                processor: processor.to_string(),
                config: ServiceConfig {
                    rebalancer_account: ServiceAccountType::Addr(reb_acc.to_string()),
                    rebalancer_manager_addr: ServiceAccountType::Addr(reb_manager.to_string()),
                    denoms: vec![NTRN_DENOM.to_string(), "denom2".to_string()],
                    base_denom: NTRN_DENOM.to_string(),
                },
            },
        )
        .unwrap();

        let res = execute(
            deps.as_mut(),
            env,
            info_processor,
            valence_service_utils::msg::ExecuteMsg::ProcessAction(
                crate::msg::ActionMsgs::StartRebalance {
                    trustee: None,
                    pid: PID {
                        p: "0.1".to_string(),
                        i: "0".to_string(),
                        d: "0".to_string(),
                    },
                    max_limit_bps: None,
                    min_balance: Uint128::from(1_000_000_u128),
                },
            ),
        )
        .unwrap();
        println!("{:?}", res);
    }
}
