#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_library_utils::{
    error::LibraryError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::{
    msg::{
        Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, ObligationStatusResponse,
        QueryMsg,
    },
    state::REGISTERED_OBLIGATION_IDS,
};

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
    use cosmwasm_std::{ensure, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Uint64};

    use valence_library_utils::{error::LibraryError, execute_on_behalf_of};

    use crate::{
        msg::{Config, FunctionMsgs},
        state::{WithdrawalObligation, CLEARING_QUEUE, REGISTERED_OBLIGATION_IDS},
    };

    pub fn process_function(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        msg: FunctionMsgs,
        cfg: Config,
    ) -> Result<Response, LibraryError> {
        match msg {
            FunctionMsgs::RegisterObligation {
                recipient,
                payout_coins,
                id,
            } => try_register_withdraw_obligation(deps, env, cfg, recipient, payout_coins, id),
            FunctionMsgs::SettleNextObligation {} => try_settle_next_obligation(deps, cfg),
        }
    }

    fn try_register_withdraw_obligation(
        deps: DepsMut,
        env: Env,
        _cfg: Config,
        recipient: String,
        payout_coins: Vec<Coin>,
        id: Uint64,
    ) -> Result<Response, LibraryError> {
        // validate that this obligation is not registered yet
        ensure!(
            !REGISTERED_OBLIGATION_IDS.has(deps.storage, id.u64()),
            LibraryError::ExecutionError(format!(
                "obligation #{id} is already registered in the queue"
            ))
        );

        // obligation payouts cannot be empty
        ensure!(
            !payout_coins.is_empty(),
            LibraryError::ExecutionError(
                "obligation must have payout coins in order to be registered".to_string()
            )
        );

        // each coin in the obligation to be paid out must have non-zero amount
        for payout_coin in &payout_coins {
            ensure!(
                !payout_coin.amount.is_zero(),
                LibraryError::ExecutionError(format!(
                    "obligation payout coin {} amount cannot be zero",
                    payout_coin.denom
                ))
            );
        }

        let withdraw_obligation = WithdrawalObligation {
            recipient: deps.api.addr_validate(&recipient)?,
            payout_coins,
            id,
            enqueue_block: env.block,
        };

        // push the obligation to the back of the fifo queue
        CLEARING_QUEUE.push_back(deps.storage, &withdraw_obligation)?;

        // store the id of the registered obligation in the map with
        // value `false` to indicate that this obligation is not yet
        // settled/complete.
        // this map also serves as a check to prevent registering (and
        // thus settling) the same obligation twice. because of this,
        // upon settlement, the key remains - only the value is updated.
        REGISTERED_OBLIGATION_IDS.save(deps.storage, id.u64(), &false)?;

        Ok(Response::new())
    }

    fn try_settle_next_obligation(deps: DepsMut, cfg: Config) -> Result<Response, LibraryError> {
        // pop the head of the queue (oldest obligation)
        let obligations_head = CLEARING_QUEUE.pop_front(deps.storage)?;

        let obligation = match obligations_head {
            Some(o) => o,
            None => {
                return Err(LibraryError::ExecutionError(
                    "no pending obligations".to_string(),
                ))
            }
        };

        let mut transfer_coins = vec![];

        // ensure that the settlement account is sufficiently topped up
        // to fulfill the obligation
        for obligation_coin in obligation.payout_coins {
            let settlement_acc_bal = deps.querier.query_balance(
                cfg.settlement_acc_addr.as_str(),
                obligation_coin.denom.to_string(),
            )?;

            ensure!(
                settlement_acc_bal.amount >= obligation_coin.amount,
                LibraryError::ExecutionError(format!(
                    "insufficient settlement acc balance to fulfill obligation: {} < {}",
                    settlement_acc_bal, obligation_coin
                ))
            );

            // push the validated coin to be paid out
            transfer_coins.push(obligation_coin);
        }

        let fill_msg = BankMsg::Send {
            to_address: obligation.recipient.to_string(),
            amount: transfer_coins,
        };

        let input_account_msg =
            execute_on_behalf_of(vec![fill_msg.into()], &cfg.settlement_acc_addr)?;

        // update the registered obligation entry value to `true` to indicate that
        // this obligation had been settled.
        REGISTERED_OBLIGATION_IDS.save(deps.storage, obligation.id.u64(), &true)?;

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

        Ok(QueueInfoResponse { len: queue_length })
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
        QueryMsg::PendingObligations { from, to } => {
            to_json_binary(&query::get_obligations(deps, from, to)?)
        }
        QueryMsg::ObligationStatus { id } => {
            let settled = REGISTERED_OBLIGATION_IDS.load(deps.storage, id)?;
            to_json_binary(&ObligationStatusResponse { settled })
        }
    }
}
