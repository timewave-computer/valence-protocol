pub mod cosmos_cw;
pub mod cosmos_evm;
use std::fmt;

use async_trait::async_trait;
use cosmos_cw::{CosmosCosmwasmConnector, CosmosCosmwasmError};
use cosmos_evm::{CosmosEvmConnector, CosmosEvmError};
use strum::Display;
use thiserror::Error;

use crate::{
    account::InstantiateAccountData,
    config::{Config, ConfigError},
    service::ServiceConfig,
};

pub type ConnectorResult<T> = Result<T, ConnectorError>;

#[derive(Error, Debug)]
pub enum ConnectorError {
    #[error(transparent)]
    ConfigError(#[from] ConfigError),

    #[error("Cosmos Cosmwasm")]
    CosmosCosmwasm(#[from] CosmosCosmwasmError),

    #[error("Cosmos Evm")]
    CosmosEvm(#[from] CosmosEvmError),
}

/// We need some way of knowing which domain we are talking with
/// TODO: chain connection, execution, bridges for authorization.
#[derive(Debug, Display, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Domain {
    CosmosCosmwasm(&'static str),
    CosmosEvm(&'static str),
    // Solana
}

impl Domain {
    pub async fn generate_connector(&self, cfg: &Config) -> ConnectorResult<Box<dyn Connector>> {
        Ok(match self {
            Domain::CosmosCosmwasm(chain_name) => Box::new(
                CosmosCosmwasmConnector::new(
                    cfg.get_chain_info(chain_name)?,
                    cfg.get_code_ids(chain_name)?,
                )
                .await?,
            ),
            Domain::CosmosEvm(_) => Box::new(CosmosEvmConnector::new().await?),
        })
    }
}

#[async_trait]
pub trait Connector: fmt::Debug {
    /// Predict the address of a contract
    /// returns the address and the salt that should be used.
    async fn predict_address(
        &mut self,
        id: &u64,
        contract_name: &str,
        extra_salt: &str,
    ) -> ConnectorResult<(String, Vec<u8>)>;
    /// Instantiate an account based onthe provided data
    async fn instantiate_account(&mut self, data: &InstantiateAccountData) -> ConnectorResult<()>;
    async fn instantiate_service(
        &mut self,
        service_id: u64,
        service_config: &ServiceConfig,
        salt: Vec<u8>,
    ) -> ConnectorResult<()>;
}
