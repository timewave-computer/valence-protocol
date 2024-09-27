#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_service_utils::{
    error::ServiceError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{ActionMsgs, Config, OptionalServiceConfig, QueryMsg, ServiceConfig};

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
    msg: ExecuteMsg<ActionMsgs, OptionalServiceConfig>,
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
        CosmosMsg, DepsMut, Empty, Env, Fraction, MessageInfo, QuerierWrapper, Response, Uint128,
    };

    use valence_service_utils::{error::ServiceError, execute_on_behalf_of};

    use crate::msg::{ActionMsgs, Config, RatioConfig};

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
                let transfer_messages = prepare_transfer_messages(&cfg, &deps.querier)?;

                // Wrap the transfer messages to be executed on behalf of the input account
                let input_account_msgs = execute_on_behalf_of(transfer_messages, cfg.input_addr())?;

                Ok(Response::new()
                    .add_attribute("method", "split")
                    .add_message(input_account_msgs))
            }
        }
    }

    fn prepare_transfer_messages(
        cfg: &Config,
        querier: &QuerierWrapper<Empty>,
    ) -> Result<Vec<CosmosMsg>, ServiceError> {
        // Get input account balances for each denom (one balance query per denom)
        let mut denom_balances: HashMap<String, Uint128> = HashMap::new();
        // Compute cumulative sum of amounts for each denom
        let mut denom_amounts: HashMap<String, Uint128> = HashMap::new();
        for split in cfg.splits() {
            let denom = split.denom();
            let key = format!("{:?}", denom);
            // Query denom balance if not already in cache
            if let Entry::Vacant(e) = denom_balances.entry(key.clone()) {
                let balance = denom.query_balance(querier, cfg.input_addr())?;
                e.insert(balance);
            }
            // Increment the split amount for the denom
            let denom_amount = denom_amounts.entry(key).or_insert(Uint128::zero());
            *denom_amount += split.amount().unwrap_or_default();
        }
        // Check if the input account has sufficient balance for each denom
        denom_amounts.iter().try_for_each(|(denom, amount)| {
            let balance = denom_balances.get(denom).unwrap();
            if amount > balance {
                return Err(ServiceError::ExecutionError(format!(
                    "Insufficient balance for denom '{}' in split config (required: {}, available: {}).",
                    denom, amount, balance,
                )));
            }
            Ok(())
        })?;

        // Prepare transfer messages for each split config
        cfg.splits()
            .iter()
            .map(|split| {
                // Lookup denom balance
                let balance = denom_balances.get(&format!("{:?}", split.denom())).unwrap();
                split
                    .amount()
                    .map(Ok)
                    .or_else(|| {
                        split
                            .ratio()
                            .as_ref()
                            .map(|ratio_config| match ratio_config {
                                RatioConfig::FixedRatio(ratio) => {
                                    let amount = balance
                                    .multiply_ratio(ratio.numerator(), ratio.denominator());
                                    if amount > *balance {
                                        return Err(ServiceError::ExecutionError(format!(
                                            "Insufficient balance for denom '{}' in split config (required: {}, available: {}).",
                                            split.denom(), amount, balance,
                                        )));
                                    }
                                    Ok(amount)
                                }
                                RatioConfig::DynamicRatio { .. } => todo!(),
                            })
                    })
                    .expect("Split config must have either an amount or a ratio")
                    .and_then(|amount| {
                        split
                            .denom()
                            .get_transfer_to_message(split.account(), amount)
                            .map_err(ServiceError::from)
                    })
            })
            .collect::<Result<Vec<_>, _>>()
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use valence_service_utils::error::ServiceError;

    use crate::msg::{Config, OptionalServiceConfig};

    pub fn update_config(
        deps: &DepsMut,
        _env: Env,
        _info: MessageInfo,
        config: &mut Config,
        new_config: OptionalServiceConfig,
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
    }
}
