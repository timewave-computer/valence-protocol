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
        Addr, CosmosMsg, Decimal, DepsMut, Empty, Env, Fraction, MessageInfo, QuerierWrapper,
        Response, StdResult, Uint128,
    };

    use valence_service_utils::{denoms::CheckedDenom, error::ServiceError, execute_on_behalf_of};

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
                let transfer_amounts = prepare_transfer_amounts(&cfg, &deps.querier)?;

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
    fn prepare_transfer_messages<I>(coins_to_transfer: I) -> Result<Vec<CosmosMsg>, ServiceError>
    where
        I: IntoIterator<
            Item = (
                cosmwasm_std::Uint128,
                valence_service_utils::denoms::CheckedDenom,
                Addr,
            ),
        >,
    {
        let transfer_messages = coins_to_transfer
            .into_iter()
            .map(|(amount, denom, account)| denom.get_transfer_to_message(&account, amount))
            .collect::<StdResult<Vec<CosmosMsg>>>()?;
        Ok(transfer_messages)
    }

    fn prepare_transfer_amounts(
        cfg: &Config,
        querier: &QuerierWrapper<Empty>,
    ) -> Result<
        Vec<(
            cosmwasm_std::Uint128,
            valence_service_utils::denoms::CheckedDenom,
            Addr,
        )>,
        ServiceError,
    > {
        // Get input account balances for each denom (one balance query per denom)
        let mut denom_balances: HashMap<String, Uint128> = HashMap::new();
        // Compute cumulative sum of amounts for each denom
        let mut denom_amounts: HashMap<String, Uint128> = HashMap::new();
        let mut denom_amount_count = 0;

        for split in cfg.splits() {
            let denom = split.denom();
            let key = denom_key(denom);
            // Query denom balance if not already in cache
            if let Entry::Vacant(e) = denom_balances.entry(key.clone()) {
                let balance = denom.query_balance(querier, cfg.input_addr())?;
                e.insert(balance);
            }
            // Increment the split amount for the denom
            if let Some(amount) = split.amount() {
                let denom_amount = denom_amounts.entry(key.clone()).or_insert(Uint128::zero());
                *denom_amount += amount;
                denom_amount_count += 1;
            }
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

        if denom_amount_count == cfg.splits().len() {
            // If all splits have an amount (and we have checked the balances),
            // we can return the amounts as-is.
            return cfg
                .splits()
                .iter()
                .map(|split| {
                    let denom = split.denom();
                    let amount = split.amount().unwrap();
                    Ok((amount, denom.clone(), split.account().clone()))
                })
                .collect::<Result<Vec<_>, _>>();
        }

        // If not all splits have an amount, we need to compute the amounts based on the ratios

        // Prepare transfer messages for each split config
        let amounts_in_base_denom = cfg
            .splits()
            .iter()
            .map(|split| {
                // Lookup denom balance
                let denom = split.denom();
                let balance = denom_balances.get(&denom_key(denom)).unwrap();
                split
                    .amount()
                    .map(|amount| {
                        assert_eq!(denom, cfg.base_denom());
                        Ok((
                            amount,
                            Decimal::one(),
                            split.factor(),
                            denom.clone(),
                            split.account().clone(),
                        ))
                    })
                    .or_else(|| {
                        split
                            .ratio()
                            .as_ref()
                            .map(|ratio_config| match ratio_config {
                                RatioConfig::FixedRatio(ratio) => {
                                    let mut amount = balance
                                        .multiply_ratio(ratio.numerator(), ratio.denominator());
                                    if let Some(factor) = split.factor() {
                                        amount = amount
                                            .checked_div((*factor as u128).into())
                                            .map_err(|err| ServiceError::Std(err.into()))?;
                                    }
                                    Ok((
                                        amount,
                                        *ratio,
                                        split.factor(),
                                        denom.clone(),
                                        split.account().clone(),
                                    ))
                                }
                                RatioConfig::DynamicRatio { .. } => todo!(),
                            })
                    })
                    .expect("Split config must have either an amount or a ratio")
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

    fn denom_key(denom: &CheckedDenom) -> String {
        format!("{:?}", denom)
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
