pub mod tests;

use cosmwasm_std::{Addr, StdResult};
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
use valence_middleware_utils::type_registry::types::ValenceType;
use valence_storage_account::msg::ExecuteMsg;

use crate::msg::{AssertionConfig, InstantiateMsg, QueryMsg};

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

        let storage_acc_instantiate_msg = InstantiateMsg {};
        let asserter_instantiate_msg =
            valence_middleware_utils::type_registry::types::RegistryInstantiateMsg {};

        Suite {
            app,
            admin,
            asserter: Addr::unchecked("todo"),
            storage_account: Addr::unchecked("todo"),
            storage_slot_key: "todo".to_string(),
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

    fn query_assert(&self, key: &str) -> StdResult<()> {
        let msg = QueryMsg::Assert(AssertionConfig {
            a: todo!(),
            predicate: todo!(),
            b: todo!(),
            ty: todo!(),
        });

        self.app
            .wrap()
            .query_wasm_smart(self.asserter.clone(), &msg)
    }
}
