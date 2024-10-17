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
                let transfer_amounts = prepare_transfer_amounts(&cfg, &deps.querier)?;

                // Prepare messages to send the coins to the output account
                let transfer_messages =
                    prepare_transfer_messages(transfer_amounts, cfg.output_addr())?;

                // Wrap the transfer messages to be executed on behalf of the input account
                let input_account_msgs = transfer_messages
                    .into_iter()
                    .map(|(msg, account)| execute_on_behalf_of(vec![msg], account))
                    .collect::<StdResult<Vec<_>>>()?;

                Ok(Response::new()
                    .add_attribute("method", "split")
                    .add_messages(input_account_msgs))
            }
        }
    }

    // Prepare transfer messages for each denom
    fn prepare_transfer_messages<'a, I>(
        coins_to_transfer: I,
        output_addr: &Addr,
    ) -> Result<Vec<(CosmosMsg, &'a Addr)>, ServiceError>
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
            .map(|(amount, denom, account)| {
                denom
                    .get_transfer_to_message(output_addr, amount)
                    .map(|msg| (msg, account))
            })
            .collect::<StdResult<Vec<_>>>()?;
        Ok(transfer_messages)
    }

    fn prepare_transfer_amounts<'a>(
        cfg: &'a Config,
        querier: &QuerierWrapper<Empty>,
    ) -> Result<
        Vec<(
            cosmwasm_std::Uint128,
            &'a valence_service_utils::denoms::CheckedDenom,
            &'a Addr,
        )>,
        ServiceError,
    > {
        // Get input account balances for each denom (one balance query per denom)
        let mut account_balances: HashMap<String, Uint128> = HashMap::new();
        // Dynamic ratios
        let mut dynamic_ratios: HashMap<String, Decimal> = HashMap::new();
        let mut denom_amount_count = 0;

        for split in cfg.splits() {
            let denom = split.denom();
            let key = account_key(split.account(), denom);
            // Query account/denom balance and add it to cache
            let balance = denom.query_balance(querier, split.account())?;
            account_balances.insert(key, balance);

            match split.amount() {
                SplitAmount::FixedAmount(amount) => {
                    // Stop if the specified amount is greater than the input account's balance
                    if *amount > balance {
                        return Err(ServiceError::ExecutionError(format!(
                            "Insufficient balance on account {} for denom '{:?}' in split config (required: {}, available: {}).",
                            split.account(), denom, amount, balance,
                        )));
                    }
                    denom_amount_count += 1;
                }
                SplitAmount::DynamicRatio {
                    contract_addr,
                    params,
                } => {
                    let key = dyn_ratio_key(denom, contract_addr, params);
                    if let Entry::Vacant(e) = dynamic_ratios.entry(key) {
                        let ratio = query_dynamic_ratio(querier, contract_addr, params, denom)?;
                        e.insert(ratio);
                    }
                }
                _ => {}
            }
        }

        if denom_amount_count == cfg.splits().len() {
            // If all splits have an amount (and we have checked the balances),
            // we can return the amounts as-is.
            return cfg
                .splits()
                .iter()
                .map(|split| match split.amount() {
                    SplitAmount::FixedAmount(amount) => {
                        Ok((*amount, split.denom(), split.account()))
                    }
                    _ => unreachable!(),
                })
                .collect::<Result<Vec<_>, _>>();
        }

        // If not all splits have an amount, we need to compute the amounts based on the ratios

        // Prepare transfer messages for each split config
        let amounts_in_base_denom = cfg
            .splits()
            .iter()
            .map(|split| {
                // Lookup account/denom balance
                let account = split.account();
                let denom = split.denom();
                let balance = account_balances.get(&account_key(account, denom)).unwrap();
                let (amount, ratio, factor) = if let SplitAmount::FixedAmount(amount) =
                    split.amount()
                {
                    (*amount, Decimal::one(), &None::<u64>)
                } else {
                    let ratio = match split.amount() {
                        SplitAmount::FixedRatio(ratio) => *ratio,
                        SplitAmount::DynamicRatio {
                            contract_addr,
                            params,
                        } => *dynamic_ratios
                            .get(&dyn_ratio_key(denom, contract_addr, params))
                            .unwrap(),
                        _ => unreachable!(),
                    };
                    let mut amount = balance.multiply_ratio(ratio.numerator(), ratio.denominator());
                    if let Some(factor) = split.factor() {
                        amount = amount
                            .checked_div((*factor as u128).into())
                            .map_err(|err| ServiceError::Std(err.into()))?;
                    }
                    (amount, ratio, split.factor())
                };
                Ok((amount, ratio, factor, denom, account))
            })
            .collect::<Result<Vec<_>, ServiceError>>()?;

        // Find the minimum amount in base denom
        let min_amount_in_base_denom = *amounts_in_base_denom
            .iter()
            .map(|(amount, _, _, _, _)| amount)
            .min()
            .unwrap();

        amounts_in_base_denom
            .into_iter()
            .map(|(_, ratio, factor, denom, account)| {
                // Divide min amount by ratio (invert ratio's numerator and denominator and multiply)
                let mut amount =
                    min_amount_in_base_denom.multiply_ratio(ratio.denominator(), ratio.numerator());
                if let Some(factor) = factor {
                    amount = amount
                        .checked_mul((*factor as u128).into())
                        .map_err(|err| ServiceError::Std(err.into()))?;
                }
                Ok((amount, denom, account))
            })
            .collect::<Result<Vec<_>, _>>()
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

    fn account_key(account: &Addr, denom: &CheckedDenom) -> String {
        format!("{}/{:?}", account, denom)
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
        deps: DepsMut,
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
