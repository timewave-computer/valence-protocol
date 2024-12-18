pub mod tests;

use std::collections::BTreeMap;

use cosmwasm_std::{Addr, Binary, StdResult};
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
use neutron_sdk::bindings::types::{InterchainQueryResult, KVKey};
use valence_middleware_utils::type_registry::types::{
    NativeTypeWrapper, RegistryQueryMsg, ValenceType,
};

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

struct Suite {
    pub app: App,
    pub owner: Addr,
    pub broker_addr: Addr,
    pub registry_addr: Addr,
}

impl Default for Suite {
    fn default() -> Self {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");

        let broker_wrapper = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        let registry_wrapper = ContractWrapper::new(
            valence_middleware_osmosis::contract::execute,
            valence_middleware_osmosis::contract::instantiate,
            valence_middleware_osmosis::contract::query,
        );

        let broker_code_id = app.store_code(Box::new(broker_wrapper));
        let registry_code_id = app.store_code(Box::new(registry_wrapper));

        let broker_instantiate_msg = InstantiateMsg {};
        let registry_instantiate_msg =
            valence_middleware_utils::type_registry::types::RegistryInstantiateMsg {};

        let broker_addr = app
            .instantiate_contract(
                broker_code_id,
                owner.clone(),
                &broker_instantiate_msg,
                &[],
                "osmo_broker".to_string(),
                None,
            )
            .unwrap();

        let registry_addr = app
            .instantiate_contract(
                registry_code_id,
                owner.clone(),
                &registry_instantiate_msg,
                &[],
                "osmo_registry".to_string(),
                None,
            )
            .unwrap();

        Suite {
            app,
            owner,
            broker_addr,
            registry_addr,
        }
    }
}

impl Suite {
    fn add_new_registry(&mut self, version: &str, addr: String) -> AppResponse {
        let msg = ExecuteMsg::SetRegistry {
            version: version.to_string(),
            address: addr,
        };

        self.app
            .execute_contract(self.owner.clone(), self.broker_addr.clone(), &msg, &[])
            .unwrap()
    }

    fn query_decode_proto(
        &mut self,
        query_id: &str,
        icq_result: InterchainQueryResult,
    ) -> StdResult<NativeTypeWrapper> {
        let msg = QueryMsg {
            registry_version: None,
            query: RegistryQueryMsg::ReconstructProto {
                query_id: query_id.to_string(),
                icq_result,
            },
        };
        self.app
            .wrap()
            .query_wasm_smart(self.broker_addr.clone(), &msg)
    }

    fn get_kv_key(&mut self, query_id: &str, params: BTreeMap<String, Binary>) -> StdResult<KVKey> {
        let msg = QueryMsg {
            registry_version: None,
            query: RegistryQueryMsg::KVKey {
                type_id: query_id.to_string(),
                params,
            },
        };

        self.app
            .wrap()
            .query_wasm_smart(self.broker_addr.clone(), &msg)
    }

    fn try_to_canonical(&mut self, type_url: &str, binary: Binary) -> StdResult<ValenceType> {
        let msg = QueryMsg {
            registry_version: None,
            query: RegistryQueryMsg::ToCanonical {
                type_url: type_url.to_string(),
                binary,
            },
        };

        self.app
            .wrap()
            .query_wasm_smart(self.broker_addr.clone(), &msg)
    }

    fn try_from_canonical(&mut self, obj: ValenceType) -> StdResult<NativeTypeWrapper> {
        let msg = QueryMsg {
            registry_version: None,
            query: RegistryQueryMsg::FromCanonical { obj },
        };

        self.app
            .wrap()
            .query_wasm_smart(self.broker_addr.clone(), &msg)
    }
}
