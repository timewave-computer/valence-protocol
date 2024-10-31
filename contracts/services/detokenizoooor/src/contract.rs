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
    use std::collections::{HashMap, HashSet};

    use cosmwasm_std::{coins, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128};
    use valence_service_utils::error::ServiceError;

    use crate::msg::{ActionMsgs, Config};

    pub fn process_action(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        msg: ActionMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        match msg {
            ActionMsgs::Detokenize { addresses } => detokenize(deps, env, cfg, addresses),
        }
    }

    fn detokenize(
        deps: DepsMut,
        env: Env,
        cfg: Config,
        addresses: HashSet<String>,
    ) -> Result<Response, ServiceError> {
        let mut voucher_balances: HashMap<String, Uint128> = HashMap::new();
        let mut total_vouchers = Uint128::zero();
        // Query asset balance for each address
        for address in addresses {
            // Validate it's a valid address
            deps.api.addr_validate(&address)?;

            // Query balance of voucher denom
            let balance = deps.querier.query_balance(
                address.clone(),
                cfg.detokenizoooor_config.voucher_denom.clone(),
            )?;
            // Ignore addresses that don't have tokens
            if balance.amount.is_zero() {
                continue;
            }
            voucher_balances.insert(address, balance.amount);
            // Add it to the total balance to know how much we are sending to the service (detokenizing)
            total_vouchers += balance.amount;
        }

        // How much has been detokenized already (balance of the token in the service)
        let service_voucher_balance = deps.querier.query_balance(
            env.contract.address.clone(),
            cfg.detokenizoooor_config.voucher_denom.clone(),
        )?;

        // Substract this from the total supply to know exactly what are the amount of vouchers that have not been detokenized
        let total_supply = deps
            .querier
            .query_supply(cfg.detokenizoooor_config.voucher_denom.clone())?;
        let remaining_supply = total_supply
            .amount
            .saturating_sub(service_voucher_balance.amount);

        let mut response = Response::new();
        // Create the send message to the service to consider the tokens detokenized in the future
        let bank_send = CosmosMsg::Bank(BankMsg::Send {
            to_address: env.contract.address.to_string(),
            amount: coins(
                total_vouchers.u128(),
                cfg.detokenizoooor_config.voucher_denom.clone(),
            ),
        });
        response = response.add_message(bank_send);

        // Get the redeemable denoms balance of the input address

        Ok(response)
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
            let raw_config: ServiceConfig =
                valence_service_utils::raw_config::query_raw_service_config(deps.storage)?;
            to_json_binary(&raw_config)
        }
    }
}
