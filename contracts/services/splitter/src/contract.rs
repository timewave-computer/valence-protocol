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
    use std::collections::{hash_map::Entry, HashMap};

    use cosmwasm_std::{
        Addr, CosmosMsg, Decimal, DepsMut, Empty, Env, Fraction, MessageInfo, QuerierWrapper,
        Response, StdResult, Uint128,
    };

    use itertools::Itertools;
    use valence_service_utils::{
        denoms::CheckedDenom,
        error::ServiceError,
        execute_on_behalf_of,
        msg::{DynamicRatioQueryMsg, DynamicRatioResponse},
    };

    use crate::msg::{ActionMsgs, Config, SplitAmount};

    pub fn process_action(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: ActionMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        match msg {
            ActionMsgs::Split {} => {
                // Determine the amounts to transfer per split config
                let transfer_amounts = prepare_transfer_amounts(&deps.querier, &cfg)?;

                // Prepare messages to send the coins to the output account
                let transfer_messages = prepare_transfer_messages(transfer_amounts)?;

                // Wrap the transfer messages to be executed on behalf of the input account
                let input_account_msgs = execute_on_behalf_of(transfer_messages, cfg.input_addr())?;

                Ok(Response::new()
                    .add_attribute("method", "split")
                    .add_message(input_account_msgs))
            }
        }
    }

    // Prepare transfer messages for each denom
    fn prepare_transfer_messages<'a, I>(
        coins_to_transfer: I,
    ) -> Result<Vec<CosmosMsg>, ServiceError>
    where
        I: IntoIterator<
            Item = (
                cosmwasm_std::Uint128,
                &'a valence_service_utils::denoms::CheckedDenom,
                &'a Addr,
            ),
        >,
    {
        let transfer_messages = coins_to_transfer
            .into_iter()
            .map(|(amount, denom, account)| denom.get_transfer_to_message(account, amount))
            .collect::<StdResult<Vec<CosmosMsg>>>()?;
        Ok(transfer_messages)
    }

    fn prepare_transfer_amounts<'a>(
        querier: &QuerierWrapper<Empty>,
        cfg: &'a Config,
    ) -> Result<
        Vec<(
            cosmwasm_std::Uint128,
            &'a valence_service_utils::denoms::CheckedDenom,
            &'a Addr,
        )>,
        ServiceError,
    > {
        // Get input account balances for each denom (one balance query per denom)
        let mut denom_balances: HashMap<String, Uint128> = HashMap::new();
        // Dynamic ratios
        let mut dynamic_ratios: HashMap<String, Decimal> = HashMap::new();

        for split in cfg.splits() {
            let denom = split.denom();
            let key = denom_key(denom);
            // Query denom balance if not already in cache
            if let Entry::Vacant(e) = denom_balances.entry(key.clone()) {
                let balance = denom.query_balance(querier, cfg.input_addr())?;
                e.insert(balance);
            }

            if let SplitAmount::DynamicRatio {
                contract_addr,
                params,
            } = split.amount()
            {
                let key = dyn_ratio_key(denom, contract_addr, params);
                if let Entry::Vacant(e) = dynamic_ratios.entry(key) {
                    let ratio = query_dynamic_ratio(querier, contract_addr, params, denom)?;
                    e.insert(ratio);
                }
            }
        }

        // Prepare transfer messages for each split config
        let amounts = cfg
            .splits()
            .iter()
            .map(|split| {
                let balance = denom_balances.get(&denom_key(split.denom())).unwrap();
                let amount = match split.amount() {
                    SplitAmount::FixedAmount(amount) => *amount,
                    SplitAmount::FixedRatio(ratio) => {
                        balance.multiply_ratio(ratio.numerator(), ratio.denominator())
                    }
                    SplitAmount::DynamicRatio {
                        contract_addr,
                        params,
                    } => {
                        let ratio = dynamic_ratios
                            .get(&dyn_ratio_key(split.denom(), contract_addr, params))
                            .unwrap();
                        balance.multiply_ratio(ratio.numerator(), ratio.denominator())
                    }
                };
                Ok((amount, split.denom(), split.account()))
            })
            .collect::<Result<Vec<_>, ServiceError>>()?;

        amounts.iter()
            .into_group_map_by(|(_, denom, _)| denom_key(denom))
            .into_iter()
            .map(|(denom, amounts)| {
            let total_amount: Uint128 = amounts.iter().map(|(amount, _, _)| *amount).sum();
            let balance = denom_balances.get(&denom).unwrap();
            if total_amount > *balance {
                return Err(ServiceError::ExecutionError(format!(
                    "Insufficient balance for denom '{}' in split config (required: {}, available: {}).",
                    denom, total_amount, balance,
                )));
            }
            Ok(())
        }).collect::<Result<Vec<()>, ServiceError>>()?;

        Ok(amounts)
    }

    fn query_dynamic_ratio(
        querier: &QuerierWrapper<Empty>,
        contract_addr: &Addr,
        params: &str,
        denom: &CheckedDenom,
    ) -> Result<Decimal, ServiceError> {
        let denom_name = denom.to_string();
        let res: DynamicRatioResponse = querier.query_wasm_smart(
            contract_addr,
            &DynamicRatioQueryMsg::DynamicRatio {
                denoms: vec![denom_name.clone()],
                params: params.to_string(),
            },
        )?;
        res.denom_ratios
            .get(&denom_name)
            .copied()
            .ok_or(ServiceError::ExecutionError(format!(
                "Dynamic ratio not found for denom '{}'.",
                denom
            )))
    }

    fn denom_key(denom: &CheckedDenom) -> String {
        format!("{:?}", denom)
    }

    fn dyn_ratio_key(denom: &CheckedDenom, contract_addr: &Addr, params: &str) -> String {
        format!("{:?}-{}/{}", denom, contract_addr, params)
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use valence_service_utils::error::ServiceError;

    use crate::msg::{Config, ServiceConfigUpdate};

    pub fn update_config(
        deps: &DepsMut,
        _env: Env,
        _info: MessageInfo,
        config: &mut Config,
        new_config: ServiceConfigUpdate,
    ) -> Result<(), ServiceError> {
        new_config.update_config(deps, config)
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
            let raw_config: ServiceConfig =
                valence_service_utils::raw_config::query_raw_service_config(deps.storage)?;
            to_json_binary(&raw_config)
        }
    }
}
