#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_library_utils::{
    error::LibraryError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::{
    msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg},
    state::OBLIGATION_ID_TO_STATUS_MAP,
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
    use cosmwasm_std::{
        ensure, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Uint128, Uint64,
    };

    use valence_library_utils::{error::LibraryError, execute_on_behalf_of};

    use crate::{
        msg::{Config, FunctionMsgs},
        state::{
            ObligationStatus, WithdrawalObligation, CLEARING_QUEUE, OBLIGATION_ID_TO_STATUS_MAP,
        },
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
                payout_amount,
                id,
            } => try_register_withdraw_obligation(deps, env, cfg, recipient, payout_amount, id),
            FunctionMsgs::SettleNextObligation {} => try_settle_next_obligation(deps, cfg),
        }
    }

    fn try_register_withdraw_obligation(
        deps: DepsMut,
        env: Env,
        mut cfg: Config,
        recipient: String,
        payout_amount: Uint128,
        id: Uint64,
    ) -> Result<Response, LibraryError> {
        // increment the latest obligation id to the expected value
        cfg.latest_id = cfg
            .latest_id
            .checked_add(Uint64::one())
            .map_err(|_| LibraryError::ExecutionError("id overflow".to_string()))?;

        // save the config with the incremented id
        valence_library_base::save_config(deps.storage, &cfg)?;

        // validate that id of the obligation being registered is monotonically increasing
        ensure!(
            cfg.latest_id == id,
            LibraryError::ExecutionError(format!(
                "obligation being registered id out of order: expected {}, got {id}",
                cfg.latest_id
            ))
        );

        // we validate the obligation recipient address:
        // - if address is valid, we proceed
        // - if address is invalid, we immediately mark the obligation as processed
        // with an error message to not block further obligations from being registered
        let validated_recipient = match deps.api.addr_validate(&recipient) {
            Ok(addr) => addr,
            Err(e) => {
                OBLIGATION_ID_TO_STATUS_MAP.save(
                    deps.storage,
                    id.u64(),
                    &ObligationStatus::Error(e.to_string()),
                )?;
                return Ok(Response::default());
            }
        };

        // we validate the payout amount:
        // - if amount is non-zero, we proceed
        // - if amount is zero, we immediately mark the obligation as processed
        // with an error message to not block further obligations from being registered
        let payout_coin = if !payout_amount.is_zero() {
            Coin {
                amount: payout_amount,
                denom: cfg.denom.to_string(),
            }
        } else {
            OBLIGATION_ID_TO_STATUS_MAP.save(
                deps.storage,
                id.u64(),
                &ObligationStatus::Error("zero payout amount".to_string()),
            )?;
            return Ok(Response::default());
        };

        // construct the valid withdrawal obligation to be queued
        let withdraw_obligation = WithdrawalObligation {
            recipient: validated_recipient,
            payout_coin,
            id,
            enqueue_block: env.block,
        };

        // push the obligation to the back of the fifo queue
        CLEARING_QUEUE.push_back(deps.storage, &withdraw_obligation)?;

        // store the id of the registered obligation in the map with
        // value `InQueue` to indicate that this obligation is not yet
        // settled/complete.
        OBLIGATION_ID_TO_STATUS_MAP.save(deps.storage, id.u64(), &ObligationStatus::InQueue)?;

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

        // ensure that the settlement account is sufficiently topped up
        // to fulfill the obligation
        let settlement_acc_bal = deps.querier.query_balance(
            cfg.settlement_acc_addr.as_str(),
            obligation.payout_coin.denom.to_string(),
        )?;

        ensure!(
            settlement_acc_bal.amount >= obligation.payout_coin.amount,
            LibraryError::ExecutionError(format!(
                "insufficient settlement acc balance to fulfill obligation: {} < {}",
                settlement_acc_bal, obligation.payout_coin
            ))
        );

        let fill_msg = BankMsg::Send {
            to_address: obligation.recipient.to_string(),
            amount: vec![obligation.payout_coin],
        };

        let input_account_msg =
            execute_on_behalf_of(vec![fill_msg.into()], &cfg.settlement_acc_addr)?;

        // mark the obligation as processed
        OBLIGATION_ID_TO_STATUS_MAP.save(
            deps.storage,
            obligation.id.u64(),
            &ObligationStatus::Processed,
        )?;

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
            let obligation_status = OBLIGATION_ID_TO_STATUS_MAP.load(deps.storage, id)?;
            to_json_binary(&obligation_status)
        }
    }
}
