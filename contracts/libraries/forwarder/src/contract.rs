#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_library_utils::{
    error::LibraryError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<LibraryConfig>,
) -> Result<Response, LibraryError> {
    valence_library_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<FunctionMsgs, LibraryConfigUpdate>,
) -> Result<Response, LibraryError> {
    valence_library_base::execute(
        deps,
        env,
        info,
        msg,
        functions::process_function,
        execute::update_config,
    )
}

mod functions {
    use cosmwasm_std::{
        Addr, CosmosMsg, DepsMut, Env, MessageInfo, QuerierWrapper, Response, StdResult,
    };
    use valence_library_utils::{error::LibraryError, execute_on_behalf_of};

    use crate::{
        msg::{Config, FunctionMsgs},
        state::LAST_SUCCESSFUL_FORWARD,
    };

    pub fn process_function(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        msg: FunctionMsgs,
        cfg: Config,
    ) -> Result<Response, LibraryError> {
        match msg {
            FunctionMsgs::Forward {} => {
                ensure_forwarding_interval(&cfg, &deps, &env)?;

                // Determine the amount to transfer for each denom
                let transfer_amounts = prepare_transfer_amounts(&cfg, &deps.querier);

                // Prepare messages to send the coins to the output account
                let transfer_messages =
                    prepare_transfer_messages(transfer_amounts, cfg.output_addr())?;

                // Wrap the transfer messages to be executed on behalf of the input account
                let input_account_msgs = execute_on_behalf_of(transfer_messages, cfg.input_addr())?;

                // Save last successful forward
                LAST_SUCCESSFUL_FORWARD.save(deps.storage, &env.block)?;

                Ok(Response::new()
                    .add_attribute("method", "forward")
                    .add_message(input_account_msgs))
            }
        }
    }

    // Prepare transfer messages for each denom
    fn prepare_transfer_messages<I>(
        coins_to_transfer: I,
        output_addr: &Addr,
    ) -> Result<Vec<CosmosMsg>, LibraryError>
    where
        I: IntoIterator<
            Item = (
                cosmwasm_std::Uint128,
                valence_library_utils::denoms::CheckedDenom,
            ),
        >,
    {
        let transfer_messages = coins_to_transfer
            .into_iter()
            .map(|(amount, denom)| denom.get_transfer_to_message(output_addr, amount))
            .collect::<StdResult<Vec<CosmosMsg>>>()?;
        Ok(transfer_messages)
    }

    // Prepare transfer amounts for each denom
    fn prepare_transfer_amounts<C>(
        cfg: &Config,
        querier: &QuerierWrapper<C>,
    ) -> Vec<(
        cosmwasm_std::Uint128,
        valence_library_utils::denoms::CheckedDenom,
    )>
    where
        C: cosmwasm_std::CustomQuery,
    {
        cfg.forwarding_configs()
            .iter()
            .filter_map(|fwd_cfg| {
                fwd_cfg
                    .denom()
                    .query_balance(querier, cfg.input_addr())
                    .ok()
                    .filter(|balance| !balance.is_zero())
                    .map(|balance| {
                        // Take minimum of input account balance and configured max amount for denom
                        let amount = balance.min(*fwd_cfg.max_amount());
                        (amount, fwd_cfg.denom().clone())
                    })
            })
            .collect::<Vec<_>>()
    }

    // Ensure the forwarding interval constraint is met
    fn ensure_forwarding_interval(
        cfg: &Config,
        deps: &DepsMut<'_>,
        env: &Env,
    ) -> Result<(), LibraryError> {
        if let Some(min_interval) = cfg.forwarding_constraints().min_interval() {
            if let Some(last_successful_forward) = LAST_SUCCESSFUL_FORWARD.may_load(deps.storage)? {
                if !min_interval
                    .after(&last_successful_forward)
                    .is_expired(&env.block)
                {
                    return Err(LibraryError::ExecutionError(
                        "Forwarding constraint not met.".to_string(),
                    ));
                }
            }
        };
        Ok(())
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use valence_library_utils::error::LibraryError;

    use crate::msg::LibraryConfigUpdate;

    pub fn update_config(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        new_config: LibraryConfigUpdate,
    ) -> Result<(), LibraryError> {
        new_config.update_config(deps)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => {
            to_json_binary(&valence_library_base::get_ownership(deps.storage)?)
        }
        QueryMsg::GetProcessor {} => {
            to_json_binary(&valence_library_base::get_processor(deps.storage)?)
        }
        QueryMsg::GetLibraryConfig {} => {
            let config: Config = valence_library_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
        QueryMsg::GetRawLibraryConfig {} => {
            let raw_config: LibraryConfig =
                valence_library_utils::raw_config::query_raw_library_config(deps.storage)?;
            to_json_binary(&raw_config)
        }
    }
}
