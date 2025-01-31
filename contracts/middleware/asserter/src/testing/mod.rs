pub mod tests;

use crate::msg::{AssertionConfig, InstantiateMsg, QueryMsg};
use cosmwasm_std::{Addr, Coin, StdResult, Uint128};
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
use valence_account_utils::msg::InstantiateMsg as StorageAccountInstantiateMsg;
use valence_middleware_utils::type_registry::types::ValenceType;
use valence_storage_account::msg::ExecuteMsg;

pub const STORAGE_SLOT_KEY: &str = "pool";

struct Suite {
    pub app: App,
    pub admin: Addr,
    pub asserter: Addr,
    pub storage_account: Addr,
    pub storage_slot_key: String,
}

impl Default for Suite {
    fn default() -> Self {
        let mut app = App::default();

        let admin = app.api().addr_make("owner");

        let asserter_wrapper = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        let storage_acc_wrapper = ContractWrapper::new(
            valence_storage_account::contract::execute,
            valence_storage_account::contract::instantiate,
            valence_storage_account::contract::query,
        );

        let asserter_code_id = app.store_code(Box::new(asserter_wrapper));
        let storage_acc_code_id = app.store_code(Box::new(storage_acc_wrapper));

        let storage_acc_instantiate_msg = StorageAccountInstantiateMsg {
            admin: admin.to_string(),
            approved_libraries: vec![],
        };
        let asserter_instantiate_msg = InstantiateMsg {};

        let asserter_addr = app
            .instantiate_contract(
                asserter_code_id,
                admin.clone(),
                &asserter_instantiate_msg,
                &[],
                "valence_asserter".to_string(),
                None,
            )
            .unwrap();

        let storage_acc_addr = app
            .instantiate_contract(
                storage_acc_code_id,
                admin.clone(),
                &storage_acc_instantiate_msg,
                &[],
                "valence_storage_account".to_string(),
                None,
            )
            .unwrap();

        Suite {
            app,
            admin,
            asserter: asserter_addr,
            storage_account: storage_acc_addr,
            storage_slot_key: STORAGE_SLOT_KEY.to_string(),
        }
    }
}

impl Suite {
    fn post_valence_type(&mut self, key: &str, valence_type: ValenceType) -> AppResponse {
        let msg = ExecuteMsg::StoreValenceType {
            key: key.to_string(),
            variant: valence_type,
        };

        self.app
            .execute_contract(self.admin.clone(), self.storage_account.clone(), &msg, &[])
            .unwrap()
    }

    fn query_assert(&self, assertion_config: AssertionConfig) -> StdResult<String> {
        let msg = QueryMsg::Assert(assertion_config);

        self.app
            .wrap()
            .query_wasm_smart(self.asserter.clone(), &msg)
    }

    fn default_coins() -> Vec<Coin> {
        vec![
            Coin {
                denom: "untrn".to_string(),
                amount: Uint128::new(1000000),
            },
            Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::new(500000),
            },
        ]
    }
}
