use cosmwasm_std::{coin, Addr, Coin, Empty, Uint64};
use cw_multi_test::{App, ContractWrapper};
use valence_library_utils::{
    msg::InstantiateMsg,
    testing::{LibraryTestSuite, LibraryTestSuiteBase},
};

use crate::msg::LibraryConfig;

use super::suite::{ClearingQueueTestingSuite, DENOM_1};

const USER_1: &str = "USER_1";
const USER_2: &str = "USER_2";
const USER_3: &str = "USER_3";

pub struct ClearingQueueTestingSuiteBuilder {
    pub inner: LibraryTestSuiteBase,
    pub input_bal: Coin,
    pub input_addr: Addr,
    pub processor: Addr,
    pub code_id: u64,
    pub latest_id: Option<Uint64>,
}

impl LibraryTestSuite<Empty, Empty> for ClearingQueueTestingSuiteBuilder {
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

impl Default for ClearingQueueTestingSuiteBuilder {
    fn default() -> Self {
        let mut inner = LibraryTestSuiteBase::new();

        let input_addr = inner.get_contract_addr(inner.account_code_id(), "input_account");

        // Template contract
        let clearing_contract_wrapper = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );

        let clearing_code_id = inner
            .app_mut()
            .store_code(Box::new(clearing_contract_wrapper));

        let processor = inner.processor().clone();

        Self {
            inner,
            input_bal: coin(1_000, DENOM_1),
            input_addr,
            code_id: clearing_code_id,
            processor,
            latest_id: None,
        }
    }
}

impl ClearingQueueTestingSuiteBuilder {
    pub fn with_input_balance(mut self, input_bal: Coin) -> Self {
        self.input_bal = input_bal;
        self
    }

    pub fn with_input_acc(mut self, input_addr: &str) -> Self {
        self.input_addr = Addr::unchecked(input_addr);
        self
    }

    pub fn with_latest_obligation_id(mut self, obligation_id: u64) -> Self {
        self.latest_id = Some(obligation_id.into());
        self
    }

    pub fn build(mut self) -> ClearingQueueTestingSuite {
        let cfg = LibraryConfig::new(
            valence_library_utils::LibraryAccountType::Addr(self.input_addr.to_string()),
            self.input_bal.denom.to_string(),
            self.latest_id,
        );

        let init_msg = InstantiateMsg {
            owner: self.owner().to_string(),
            processor: self.processor().to_string(),
            config: cfg.clone(),
        };

        let addr = self.contract_init(self.code_id, "clearing_queue_lib", &init_msg, &[]);

        let input_addr = self.input_addr.clone();

        if self.app_mut().contract_data(&input_addr).is_err() {
            let account_addr = self.account_init("input_account", vec![addr.to_string()]);
            assert_eq!(account_addr, input_addr);

            if !self.input_bal.amount.is_zero() {
                self.init_balance(&input_addr, vec![self.input_bal.clone()]);
            }
        }

        let user_1_addr = self.api().addr_make(USER_1);
        let user_2_addr = self.api().addr_make(USER_2);
        let user_3_addr = self.api().addr_make(USER_3);

        ClearingQueueTestingSuite {
            inner: self.inner,
            clearing_queue: addr,
            input_addr,
            processor: self.processor,
            user_1: user_1_addr,
            user_2: user_2_addr,
            user_3: user_3_addr,
        }
    }
}
