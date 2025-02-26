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
    use cosmwasm_std::{to_json_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg};
    use valence_library_utils::{error::LibraryError, execute_on_behalf_of};
    use valence_liquid_staking_utils::drop::LiquidStakerExecuteMsg;

    use crate::msg::{Config, FunctionMsgs};

    pub fn process_function(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: FunctionMsgs,
        cfg: Config,
    ) -> Result<Response, LibraryError> {
        match msg {
            FunctionMsgs::LiquidStake { r#ref } => {
                // We will query the balance of the input address and stake the entire balance
                // to the liquid staker address.
                let balance = deps
                    .querier
                    .query_balance(cfg.input_addr.clone(), cfg.denom)?;

                // Check that input account has something to stake
                if balance.amount.is_zero() {
                    return Err(LibraryError::ExecutionError(
                        "No funds to stake".to_string(),
                    ));
                }

                let stake_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.liquid_staker_addr.to_string(),
                    msg: to_json_binary(&LiquidStakerExecuteMsg::Bond {
                        receiver: Some(cfg.output_addr.to_string()),
                        r#ref,
                    })?,
                    funds: vec![balance],
                });

                // Wrap the stake msg to be executed on behalf of the input account
                let input_account_msgs = execute_on_behalf_of(vec![stake_msg], &cfg.input_addr)?;

                Ok(Response::new()
                    .add_message(input_account_msgs)
                    .add_attribute("method", "liquid_stake"))
            }
        }
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
