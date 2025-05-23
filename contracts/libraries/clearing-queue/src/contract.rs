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

mod functions {
    use cosmwasm_std::{ensure, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Uint256};

    use valence_library_utils::{
        error::{LibraryError, UnauthorizedReason},
        execute_on_behalf_of,
    };

    use crate::{
        msg::{Config, FunctionMsgs},
        state::{WithdrawalObligation, CLEARING_QUEUE},
    };

    pub fn process_function(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: FunctionMsgs,
        cfg: Config,
    ) -> Result<Response, LibraryError> {
        match msg {
            FunctionMsgs::RegisterObligation {
                recipient,
                payout_coins,
                id,
            } => {
                try_register_withdraw_obligation(deps, env, info, cfg, recipient, payout_coins, id)
            }
            FunctionMsgs::SettleNextObligation {} => try_settle_next_obligation(deps, cfg),
        }
    }

    fn try_register_withdraw_obligation(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        cfg: Config,
        recipient: String,
        payout_coins: Vec<Coin>,
        id: Uint256,
    ) -> Result<Response, LibraryError> {
        // only the approved strategist can register new obligations
        ensure!(
            cfg.strategist == info.sender,
            LibraryError::Unauthorized(UnauthorizedReason::NotAllowed {})
        );

        let withdraw_obligation = WithdrawalObligation {
            recipient,
            payout_coins,
            id,
            enque_block: env.block,
        };

        // push the obligation to the back of the fifo queue
        CLEARING_QUEUE.push_back(deps.storage, &withdraw_obligation)?;

        Ok(Response::new())
    }

    fn try_settle_next_obligation(deps: DepsMut, cfg: Config) -> Result<Response, LibraryError> {
        let obligations_head = CLEARING_QUEUE.pop_front(deps.storage)?;

        let obligation = obligations_head
            .ok_or_else(|| LibraryError::ExecutionError("no pending obligations".to_string()))?;

        let mut transfer_coins = vec![];

        // ensure that the settlement account is sufficiently topped up
        // to fulfill the obligation
        for obligation_coin in obligation.payout_coins {
            let input_acc_bal = deps
                .querier
                .query_balance(cfg.input_addr.as_str(), obligation_coin.denom.to_string())?;

            ensure!(
                input_acc_bal.amount >= obligation_coin.amount,
                LibraryError::ExecutionError(format!(
                    "insufficient settlement acc balance to fulfill obligation: {} < {}",
                    input_acc_bal, obligation_coin
                ))
            );

            // push the validated coin to be paid out
            transfer_coins.push(obligation_coin);
        }

        let fill_msg = BankMsg::Send {
            to_address: obligation.recipient,
            amount: transfer_coins,
        };

        let input_account_msg = execute_on_behalf_of(vec![fill_msg.into()], &cfg.input_addr)?;

        Ok(Response::new().add_message(input_account_msg))
    }
}

mod query {
    use crate::{
        msg::{ObligationsResponse, QueueInfoResponse},
        state::CLEARING_QUEUE,
    };
    use cosmwasm_std::{Deps, StdResult};

    pub fn get_queue_info(deps: Deps) -> StdResult<QueueInfoResponse> {
        let queue_length = CLEARING_QUEUE.len(deps.storage)?;

        Ok(QueueInfoResponse {
            count: queue_length,
            start_index: 0,
            end_index: queue_length,
        })
    }

    pub fn get_obligations(
        deps: Deps,
        from: Option<u64>,
        to: Option<u64>,
    ) -> StdResult<ObligationsResponse> {
        let obligations =
            CLEARING_QUEUE.query(deps.storage, from, to, cosmwasm_std::Order::Ascending)?;

        Ok(ObligationsResponse { obligations })
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
        QueryMsg::QueueInfo {} => to_json_binary(&query::get_queue_info(deps)?),
        QueryMsg::Obligations { from, to } => {
            to_json_binary(&query::get_obligations(deps, from, to)?)
        }
    }
}
