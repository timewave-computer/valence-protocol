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
        ensure, BankMsg, Coin, DepsMut, Env, Fraction, MessageInfo, Response, Uint128, Uint64,
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

    /// helper function to tag the given obligation as completed with an error and return a
    /// non-error response in order to not block future obligation processing.
    fn swallow_obligation_registration_err(
        deps: DepsMut,
        id: Uint64,
        err: String,
    ) -> Result<Response, LibraryError> {
        OBLIGATION_ID_TO_STATUS_MAP.save(deps.storage, id.u64(), &ObligationStatus::Error(err))?;
        Ok(Response::default())
    }

    /// registers a withdraw obligation by pushing it to the clearing queue and
    /// creating a obligation status map entry.
    /// because the clearing queue operates in a FIFO manner, we must prevent
    /// invalid obligations from being registered to prevent blocking the queue.
    /// some of the ways that an obligation may block the queue are:
    /// - invalid recipient address
    /// - payout coins with zero-amounts
    ///   for that reason, registration should swallow any errors that may lead
    ///   to a blocked queue by immediately tagging the obligation as completed
    ///   with error.
    fn try_register_withdraw_obligation(
        deps: DepsMut,
        env: Env,
        mut cfg: Config,
        recipient: String,
        payout_amount: Uint128,
        id: Uint64,
    ) -> Result<Response, LibraryError> {
        // find the obligation id we expect to receive
        let expected_id = match cfg.latest_id {
            Some(lid) => lid
                .checked_add(Uint64::one())
                .map_err(|_| LibraryError::ExecutionError("id overflow".to_string()))?,
            // none indicates that no registration have been registered yet.
            // we expect `0`.
            None => Uint64::zero(),
        };

        // validate that id of the obligation being registered is monotonically increasing
        ensure!(
            expected_id == id,
            LibraryError::ExecutionError(format!(
                "obligation registration id out of order: expected {expected_id}, got {id}"
            ))
        );

        // we validate the obligation recipient address:
        // - if address is valid, we proceed
        // - if address is invalid, we immediately mark the obligation as processed
        // with an error message to not block further obligations from being registered
        // and return
        let validated_recipient = match deps.api.addr_validate(&recipient) {
            Ok(addr) => addr,
            Err(e) => return swallow_obligation_registration_err(deps, id, e.to_string()),
        };

        // validate the payout amount. in case 0 is passed in, we immediately tag the
        // obligation as completed and return
        if payout_amount.is_zero() {
            return swallow_obligation_registration_err(
                deps,
                id,
                "cannot register obligation with zero payout amount".to_string(),
            );
        }

        // we apply the configured settlement ratio to get the deposit denom amount.
        // we do not swallow the error here because any error here means a config error.
        let mars_amount = payout_amount
            .checked_multiply_ratio(
                cfg.mars_settlement_ratio.numerator(),
                cfg.mars_settlement_ratio.denominator(),
            )
            .map_err(|e| LibraryError::ExecutionError(e.to_string()))?;

        let mut payout_coins: Vec<Coin> = vec![];

        // push the mars obligation to the payout coins array
        payout_coins.push(Coin {
            denom: cfg.denom.to_string(),
            amount: mars_amount,
        });

        // this should never error given that mars_amount is at most the payout_amount
        let supervaults_amount = payout_amount
            .checked_sub(mars_amount)
            .map_err(|e| LibraryError::ExecutionError(e.to_string()))?;

        // if supervaults amount is non-zero, we perform the deposit simulation
        // to estimate the supervaults lp shares amount equivalent to the supervaults_amount
        // of deposit token. We do this for each of the supervaults we are withdrawing from.
        if !supervaults_amount.is_zero() {
            for info in cfg.supervaults_settlement_info.iter() {
                // first we query the supervaults to pairwise match the config denom
                // to the supervault pair data
                let supervaults_config: mmvault::state::Config = deps.querier.query_wasm_smart(
                    &info.supervault_addr,
                    &mmvault::msg::QueryMsg::GetConfig {},
                )?;

                let supervaults_simulate_lp_msg =
                    if cfg.denom.eq(&supervaults_config.pair_data.token_0.denom) {
                        mmvault::msg::QueryMsg::SimulateProvideLiquidity {
                            amount_0: supervaults_amount,
                            amount_1: Uint128::zero(),
                            sender: info.supervault_sender.clone(),
                        }
                    } else if cfg.denom.eq(&supervaults_config.pair_data.token_1.denom) {
                        mmvault::msg::QueryMsg::SimulateProvideLiquidity {
                            amount_0: Uint128::zero(),
                            amount_1: supervaults_amount,
                            sender: info.supervault_sender.clone(),
                        }
                    } else {
                        return Err(LibraryError::ConfigurationError(
                            "supervault config denom mismatch".to_string(),
                        ));
                    };

                // perform the supervaults liquidity provision simulation.
                // we know that the offer_amount here is non-zero, so we surface
                // any errors that may happen during this query (e.g. due to
                // exceeded cap, changed api, etc)
                let supervaults_lp_equivalent: Uint128 = deps
                    .querier
                    .query_wasm_smart(&info.supervault_addr, &supervaults_simulate_lp_msg)?;

                // push the supervaults obligation to the payout coins array
                payout_coins.push(Coin {
                    denom: supervaults_config.lp_denom,
                    amount: supervaults_lp_equivalent,
                });
            }
        }

        // filter out any zero-amount denoms as attempting to settle them later
        // would fail
        let filtered_payout: Vec<Coin> = payout_coins
            .into_iter()
            .filter(|c| !c.amount.is_zero())
            .collect();

        // if all coins are 0-amount, there is nothing to transfer; return
        if filtered_payout.is_empty() {
            return swallow_obligation_registration_err(
                deps,
                id,
                "all obligations 0-amount".to_string(),
            );
        }

        // construct the valid withdrawal obligation to be queued
        let withdraw_obligation = WithdrawalObligation {
            recipient: validated_recipient,
            payout_coins: filtered_payout,
            id,
            enqueue_block: env.block,
        };

        // push the obligation to the back of the fifo queue
        CLEARING_QUEUE.push_back(deps.storage, &withdraw_obligation)?;

        // store the id of the registered obligation in the map with
        // value `InQueue` to indicate that this obligation is not yet
        // settled/complete.
        OBLIGATION_ID_TO_STATUS_MAP.save(deps.storage, id.u64(), &ObligationStatus::InQueue)?;

        // set the latest registered obligation id to the current id being registered
        cfg.latest_id = Some(id);

        // save the config with the incremented id
        valence_library_base::save_config(deps.storage, &cfg)?;

        Ok(Response::default())
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

        // before attempting to settle, we validate that the settlement account
        // is topped up sufficiently to fulfill the obligation for each of the
        // payout coins
        for payout_coin in &obligation.payout_coins {
            let settlement_acc_bal = deps.querier.query_balance(
                cfg.settlement_acc_addr.as_str(),
                payout_coin.denom.to_string(),
            )?;

            ensure!(
                settlement_acc_bal.amount >= payout_coin.amount,
                LibraryError::ExecutionError(format!(
                    "insufficient settlement acc balance to fulfill obligation: {} < {}",
                    settlement_acc_bal, payout_coin
                ))
            );
        }

        let fill_msg = BankMsg::Send {
            to_address: obligation.recipient.to_string(),
            amount: obligation.payout_coins,
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
