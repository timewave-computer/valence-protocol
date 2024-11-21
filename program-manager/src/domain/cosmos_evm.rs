use std::fmt;

use async_trait::async_trait;
use thiserror::Error;
use valence_authorization_utils::domain::ExternalDomain;
use valence_processor::msg::PolytoneContracts;

use crate::{account::InstantiateAccountData, library::LibraryConfig};

use super::{Connector, ConnectorResult};

const _MNEMONIC: &str = "crazy into this wheel interest enroll basket feed fashion leave feed depth wish throw rack language comic hand family shield toss leisure repair kite";

#[derive(Error, Debug)]
pub enum CosmosEvmError {
    #[error(transparent)]
    Error(#[from] anyhow::Error),
}

pub struct CosmosEvmConnector {}

impl fmt::Debug for CosmosEvmConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CosmosEvmConnector").finish_non_exhaustive()
    }
}

impl CosmosEvmConnector {
    pub async fn new() -> Result<Self, CosmosEvmError> {
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
    ) -> ConnectorResult<(String, Vec<u8>)> {
        unimplemented!("predict_address")
    }

    async fn instantiate_account(&mut self, _data: &InstantiateAccountData) -> ConnectorResult<()> {
        unimplemented!("instantiate_account")
    }

    async fn instantiate_library(
        &mut self,
        _library_id: u64,
        _library_config: &LibraryConfig,
        _salt: Vec<u8>,
    ) -> ConnectorResult<()> {
        unimplemented!("instantiate_library")
    }

    async fn instantiate_authorization(
        &mut self,
        _program_id: u64,
        _salt: Vec<u8>,
        _processor_addr: String,
        _external_domains: Vec<ExternalDomain>,
    ) -> ConnectorResult<()> {
        unimplemented!("instantiate_authorization for cosmos_evm")
    }

    async fn instantiate_processor(
        &mut self,
        _program_id: u64,
        _salt: Vec<u8>,
        _admin: String,
        _authorization: String,
        _polytone_addr: Option<PolytoneContracts>,
    ) -> ConnectorResult<()> {
        unimplemented!("instantiate_processor")
    }
}
