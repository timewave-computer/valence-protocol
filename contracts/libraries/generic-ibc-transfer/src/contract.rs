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
    use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
    use valence_library_utils::{error::LibraryError, execute_on_behalf_of};

    use crate::msg::{Config, FunctionMsgs, IbcTransferAmount};

    pub fn process_function(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        msg: FunctionMsgs,
        cfg: Config,
    ) -> Result<Response, LibraryError> {
        match msg {
            FunctionMsgs::IbcTransfer {} => {
                let balance = cfg.denom().query_balance(&deps.querier, cfg.input_addr())?;

                let amount = match cfg.amount() {
                    IbcTransferAmount::FullAmount => balance,
                    IbcTransferAmount::FixedAmount(amount) => {
                        if balance < *amount {
                            return Err(LibraryError::ExecutionError(format!(
                                "Insufficient balance for denom '{}' in config (required: {}, available: {}).",
                                cfg.denom(), amount, balance,
                            )));
                        }
                        *amount
                    }
                };

                let ibc_send_msg = valence_ibc_utils::generic::ibc_send_message(
                    env,
                    cfg.remote_chain_info().channel_id.clone(),
                    cfg.output_addr().to_string(),
                    cfg.denom().to_string(),
                    amount.u128(),
                    cfg.memo().clone(),
                    cfg.remote_chain_info().ibc_transfer_timeout.map(Into::into),
                    cfg.denom_to_pfm_map().clone(),
                )
                .map_err(|err| LibraryError::ExecutionError(err.to_string()))?;

                let input_account_msgs =
                    execute_on_behalf_of(vec![ibc_send_msg], cfg.input_addr())?;

                Ok(Response::new()
                    .add_attribute("method", "ibc-transfer")
                    .add_message(input_account_msgs))
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
