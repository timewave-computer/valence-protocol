use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{error::AnyResult, App, AppResponse, Executor};
use valence_library_utils::{
    msg::ExecuteMsg,
    testing::{LibraryTestSuite, LibraryTestSuiteBase},
};

use crate::msg::{FunctionMsgs, LibraryConfig, LibraryConfigUpdate};

pub(crate) const DENOM_1: &str = "DENOM_1";
pub(crate) const DENOM_2: &str = "DENOM_2";

pub(crate) const USER_1: &str = "USER_1";
pub(crate) const USER_2: &str = "USER_2";
pub(crate) const USER_3: &str = "USER_3";

pub struct ClearingQueueTestingSuite {
    pub inner: LibraryTestSuiteBase,
    pub clearing_queue: Addr,
    pub input_addr: Addr,
    pub processor: Addr,
    pub owner: Addr,
    pub user_1: Addr,
    pub user_2: Addr,
    pub user_3: Addr,
}

impl ClearingQueueTestingSuite {
    pub fn register_new_obligation(
        &mut self,
        recipient: String,
        payout_coins: Vec<Coin>,
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
                    payout_coins,
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

    pub fn update_config(
        &mut self,
        addr: Addr,
        new_config: LibraryConfig,
    ) -> AnyResult<AppResponse> {
        let owner = self.owner().clone();
        let updated_config = LibraryConfigUpdate {
            input_addr: Some(new_config.input_addr),
        };
        self.app_mut().execute_contract(
            owner,
            addr,
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
