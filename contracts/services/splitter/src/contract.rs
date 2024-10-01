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
        Addr, CosmosMsg, DepsMut, Empty, Env, Fraction, MessageInfo, QuerierWrapper, Response,
        StdResult, Uint128,
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
        let mut denom_balances: HashMap<String, Uint128> = HashMap::new();
        // Compute cumulative sum of amounts per denom
        let mut denom_amounts: HashMap<String, Uint128> = HashMap::new();

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
                denom_amounts
                    .entry(key)
                    .and_modify(|denom_amount| *denom_amount += amount)
                    .or_insert(*amount);
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

        // Prepare transfer messages for each split config
        let amounts = cfg
            .splits()
            .iter()
            .map(|split| {
                split
                    .amount()
                    .map(|amount| Ok((amount, split.denom(), split.account())))
                    .or_else(|| {
                        split
                            .ratio()
                            .as_ref()
                            .map(|ratio_config| match ratio_config {
                                RatioConfig::FixedRatio(ratio) => {
                                    let balance =
                                        denom_balances.get(&denom_key(split.denom())).unwrap();
                                    let amount = balance
                                        .multiply_ratio(ratio.numerator(), ratio.denominator());
                                    Ok((amount, split.denom(), split.account()))
                                }
                                RatioConfig::DynamicRatio { .. } => todo!(),
                            })
                    })
                    .expect("Split config must have either an amount or a ratio")
            })
            .collect::<Result<Vec<_>, ServiceError>>()?;

        Ok(amounts)
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
