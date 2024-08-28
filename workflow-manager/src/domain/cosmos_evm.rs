use std::fmt;

use async_trait::async_trait;

use crate::{account::InstantiateAccountData, error::ManagerResult, service::ServiceConfig};

use super::Connector;

const _MNEMONIC: &str = "crazy into this wheel interest enroll basket feed fashion leave feed depth wish throw rack language comic hand family shield toss leisure repair kite";

pub struct CosmosEvmConnector {}

impl fmt::Debug for CosmosEvmConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CosmosEvmConnector").finish_non_exhaustive()
    }
}

impl CosmosEvmConnector {
    pub async fn new() -> ManagerResult<Self> {
        Ok(CosmosEvmConnector {})
    }
}

#[async_trait]
impl Connector for CosmosEvmConnector {
    async fn predict_address(
        &mut self,
        _id: &u64,
        _contract_name: &str,
        _extra_salt: &str,
    ) -> ManagerResult<(String, Vec<u8>)> {
        unimplemented!("predict_address")
    }

    async fn instantiate_account(&mut self, _data: &InstantiateAccountData) -> ManagerResult<()> {
        unimplemented!("instantiate_account")
    }

    async fn instantiate_service(
        &mut self,
        _service_id: u64,
        _service_config: &ServiceConfig,
        _salt: Vec<u8>,
    ) -> ManagerResult<()> {
        unimplemented!("instantiate_service")
    }
}
