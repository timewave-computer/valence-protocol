pub mod tests;

use crate::msg::{AssertionValue, InstantiateMsg, Predicate};
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::{error::AnyResult, App, AppResponse, ContractWrapper, Executor};
use valence_account_utils::msg::InstantiateMsg as StorageAccountInstantiateMsg;
use valence_middleware_utils::type_registry::types::ValenceType;
use valence_storage_account::msg::ExecuteMsg;

pub const STORAGE_SLOT_KEY: &str = "pool_osmo";
pub const STORAGE_SLOT_KEY_2: &str = "pool_astro";

struct Suite {
    pub app: App,
    pub admin: Addr,
    pub asserter: Addr,
    pub storage_account: Addr,
}

impl Default for Suite {
    fn default() -> Self {
        let mut app = App::default();

        let admin = app.api().addr_make("owner");

        let asserter_wrapper = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            // using valence storage account query fn here as there are no queries
            // on the asserter but contractwrapper expects one
            valence_storage_account::contract::query,
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

    fn assert(
        &mut self,
        a: AssertionValue,
        predicate: Predicate,
        b: AssertionValue,
    ) -> AnyResult<AppResponse> {
        let msg = crate::msg::ExecuteMsg::Assert { a, predicate, b };

        self.app
            .execute_contract(self.admin.clone(), self.asserter.clone(), &msg, &[])
    }

    fn default_coins() -> Vec<Coin> {
        vec![
            Coin {
                denom: "untrn".to_string(),
                amount: Uint128::new(1_000_000),
            },
            Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::new(500_000),
            },
        ]
    }

    fn default_coins_2() -> Vec<Coin> {
        vec![
            Coin {
                denom: "untrn".to_string(),
                amount: Uint128::new(1_200_000),
            },
            Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::new(33_000),
            },
        ]
    }
}
