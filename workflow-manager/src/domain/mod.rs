pub mod cosmos_cw;
pub mod cosmos_evm;
use std::fmt;

use async_trait::async_trait;
use cosmos_cw::CosmosCosmwasmConnector;
use cosmos_evm::CosmosEvmConnector;
use strum::Display;

use crate::{
    account::InstantiateAccountData, config::Config, error::ManagerResult, service::ServiceConfig,
};

/// We need some way of knowing which domain we are talking with
/// TODO: chain connection, execution, bridges for authorization.
#[derive(Debug, Display, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Domain {
    CosmosCosmwasm(&'static str),
    CosmosEvm(&'static str),
    // Solana
}

impl Domain {
    pub async fn generate_connector(&self, cfg: &Config) -> ManagerResult<Box<dyn Connector>> {
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
    ) -> ManagerResult<(String, Vec<u8>)>;
    /// Instantiate an account based onthe provided data
    async fn instantiate_account(&mut self, data: &InstantiateAccountData) -> ManagerResult<()>;
    async fn instantiate_service(
        &mut self,
        service_id: u64,
        service_config: &ServiceConfig,
        salt: Vec<u8>,
    ) -> ManagerResult<()>;
}
