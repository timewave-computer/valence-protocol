use cosmwasm_std::{Addr, Coin, Empty, Uint128};
use cw_multi_test::{error::AnyResult, App, AppResponse, Executor};
use valence_library_utils::{
    msg::ExecuteMsg,
    testing::{LibraryTestSuite, LibraryTestSuiteBase},
    OptionUpdate,
};

use crate::{
    msg::{
        FunctionMsgs, LibraryConfig, LibraryConfigUpdate, ObligationsResponse, QueryMsg,
        QueueInfoResponse,
    },
    state::ObligationStatus,
};

pub(crate) const DENOM_1: &str = "DENOM_1";

pub struct ClearingQueueTestingSuite {
    pub inner: LibraryTestSuiteBase,
    pub clearing_queue: Addr,
    pub input_addr: Addr,
    pub processor: Addr,
    pub user_1: Addr,
    pub user_2: Addr,
    pub user_3: Addr,
}

impl ClearingQueueTestingSuite {
    pub fn register_new_obligation(
        &mut self,
        recipient: String,
        payout_amount: Uint128,
        id: u64,
    ) -> AnyResult<AppResponse> {
        let processor = self.processor.clone();
        let clearing_queue = self.clearing_queue.clone();

        self.app_mut().execute_contract(
            processor,
            clearing_queue,
            &ExecuteMsg::<FunctionMsgs, LibraryConfigUpdate>::ProcessFunction(
                FunctionMsgs::RegisterObligation {
                    recipient,
                    payout_amount,
                    id: id.into(),
                },
            ),
            &[],
        )
    }

    pub fn settle_next_obligation(&mut self) -> AnyResult<AppResponse> {
        let processor = self.processor.clone();
        let clearing_queue = self.clearing_queue.clone();

        self.app_mut().execute_contract(
            processor,
            clearing_queue,
            &ExecuteMsg::<FunctionMsgs, LibraryConfigUpdate>::ProcessFunction(
                FunctionMsgs::SettleNextObligation {},
            ),
            &[],
        )
    }

    pub fn update_clearing_config(&mut self, new_config: LibraryConfig) -> AnyResult<AppResponse> {
        let owner = self.owner().clone();
        let clearing_lib = self.clearing_queue.clone();

        let updated_config = LibraryConfigUpdate {
            settlement_acc_addr: Some(new_config.settlement_acc_addr),
            denom: Some(new_config.denom),
            latest_id: OptionUpdate::Set(new_config.latest_id),
        };

        self.app_mut().execute_contract(
            owner,
            clearing_lib,
            &ExecuteMsg::<FunctionMsgs, LibraryConfigUpdate>::UpdateConfig {
                new_config: updated_config,
            },
            &[],
        )
    }
}

impl ClearingQueueTestingSuite {
    pub fn query_input_acc_bal(&self, denom: &str) -> Coin {
        self.inner.query_balance(&self.input_addr, denom)
    }

    pub fn query_user_bal(&self, user: &str, denom: &str) -> Coin {
        self.inner.query_balance(&Addr::unchecked(user), denom)
    }

    pub fn query_queue_info(&self) -> QueueInfoResponse {
        self.inner
            .query_wasm(&self.clearing_queue, &QueryMsg::QueueInfo {})
    }

    pub fn query_obligation_status(&self, obligation_id: u64) -> ObligationStatus {
        self.inner.query_wasm(
            &self.clearing_queue,
            &QueryMsg::ObligationStatus { id: obligation_id },
        )
    }

    pub fn query_obligations(&self, from: Option<u64>, to: Option<u64>) -> ObligationsResponse {
        self.inner.query_wasm(
            &self.clearing_queue,
            &QueryMsg::PendingObligations { from, to },
        )
    }
}

impl LibraryTestSuite<Empty, Empty> for ClearingQueueTestingSuite {
    fn app(&self) -> &App {
        self.inner.app()
    }

    fn app_mut(&mut self) -> &mut App {
        self.inner.app_mut()
    }

    fn owner(&self) -> &Addr {
        self.inner.owner()
    }

    fn processor(&self) -> &Addr {
        self.inner.processor()
    }

    fn account_code_id(&self) -> u64 {
        self.inner.account_code_id()
    }

    fn cw20_code_id(&self) -> u64 {
        self.inner.cw20_code_id()
    }
}
