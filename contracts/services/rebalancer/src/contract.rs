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
        to_json_binary, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg,
    };
    use valence_service_utils::error::ServiceError;

    use crate::{
        helpers::get_balances,
        msg::{ActionMsgs, Config},
        ATOM_DENOM, NEWT_DENOM, NTRN_DENOM, USDC_DENOM,
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
                let mut targets: HashSet<rebalancer_package::services::rebalancer::Target> =
                    HashSet::new();
                // ATOM
                targets.insert(rebalancer_package::services::rebalancer::Target {
                    denom: ATOM_DENOM.to_string(),
                    bps: 1,
                    min_balance: None,
                });
                // NTRN
                targets.insert(rebalancer_package::services::rebalancer::Target {
                    denom: NTRN_DENOM.to_string(),
                    bps: 1,
                    min_balance: None,
                });
                // NEWT
                targets.insert(rebalancer_package::services::rebalancer::Target {
                    denom: NEWT_DENOM.to_string(),
                    bps: 1,
                    min_balance: None,
                });
                // USDC
                targets.insert(rebalancer_package::services::rebalancer::Target {
                    denom: USDC_DENOM.to_string(),
                    bps: 9997,
                    min_balance: Some(min_balance.u128().into()),
                });

                let rebalancer_config = rebalancer_package::services::rebalancer::RebalancerData {
                    trustee,
                    base_denom: USDC_DENOM.to_string(),
                    targets,
                    pid,
                    max_limit_bps,
                    target_override_strategy: rebalancer_package::services::rebalancer::TargetOverrideStrategy::Proportional,
                    account_type: rebalancer_package::services::rebalancer::RebalancerAccountType::Workflow,
                };

                let register_msg = rebalancer_package::msgs::core_execute::ServicesManagerExecuteMsg::RegisterToService { 
                    service_name: rebalancer_package::services::ValenceServices::Rebalancer, 
                    data: Some(to_json_binary(&rebalancer_config)?.to_vec().into()) 
                };
                let rebalancer_wasm_msg = WasmMsg::Execute {
                    contract_addr: config.rebalancer_manager_addr.to_string(),
                    msg: to_json_binary(&register_msg)?,
                    funds: vec![],
                };

                // query the balance of the input address
                let balances = get_balances(deps.as_ref(), config.input_addr.to_string())?;
                // input addr must have 1 ntrn for rebalancer fee
                balances
                    .iter()
                    .find(|b| b.denom == NTRN_DENOM && b.amount > Uint128::from(1000000_u128))
                    .ok_or_else(|| {
                        ServiceError::ExecutionError(
                            "Input address must have at least 1 NTRN".to_string(),
                        )
                    })?;

                // send all the funds from the input addr to the rebalancer account
                let send_msgs: Vec<CosmosMsg> = balances
                    .iter()
                    .map(|b| {
                        BankMsg::Send {
                            to_address: config.rebalancer_account.to_string(),
                            amount: vec![b.clone()],
                        }
                        .into()
                    })
                    .collect();

                // register the account to the rebalancer

                Ok(Response::default()
                    .add_messages(send_msgs)
                    .add_message(rebalancer_wasm_msg))
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
