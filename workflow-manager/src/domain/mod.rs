pub mod cosmos_cw;
// pub mod cosmos_evm;
use std::fmt;

use async_trait::async_trait;
use cosmos_cw::{CosmosCosmwasmConnector, CosmosCosmwasmError};
// use cosmos_evm::CosmosEvmError;
use strum::Display;
use thiserror::Error;

use crate::{
    account::InstantiateAccountData,
    config::{ConfigError, CONFIG},
    service::ServiceConfig,
};

pub type ConnectorResult<T> = Result<T, ConnectorError>;

#[derive(Error, Debug)]
pub enum ConnectorError {
    #[error(transparent)]
    ConfigError(#[from] ConfigError),

    #[error("Cosmos Cosmwasm")]
    CosmosCosmwasm(#[from] CosmosCosmwasmError),

    // #[error("Cosmos Evm")]
    // CosmosEvm(#[from] CosmosEvmError),
}

/// We need some way of knowing which domain we are talking with
/// TODO: chain connection, execution, bridges for authorization.
#[derive(Debug, Display, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Domain {
    CosmosCosmwasm(&'static str),
    // CosmosEvm(&'static str),
    // Solana
}

impl Domain {
    pub fn get_chain_name(&self) -> &str {
        match self {
            Domain::CosmosCosmwasm(chain_name) => chain_name,
            // Domain::CosmosEvm(chain_name) => chain_name,
        }
    }
    pub async fn generate_connector(&self) -> ConnectorResult<Box<dyn Connector>> {
        Ok(match self {
            Domain::CosmosCosmwasm(chain_name) => Box::new(
                CosmosCosmwasmConnector::new(
                    CONFIG.get_chain_info(chain_name)?,
                    CONFIG.get_code_ids(chain_name)?,
                )
                .await?,
            ),
            // Domain::CosmosEvm(_) => Box::new(CosmosEvmConnector::new().await?),
        })
    }
}

#[async_trait]
pub trait Connector: fmt::Debug + Send + Sync {
    /// We want this function to only be implemented on neutron connector
    /// We provide a defualt implemention that errors out if it is used on a different connector.
    async fn reserve_workflow_id(&mut self) -> ConnectorResult<u64> {
        unimplemented!("Should only be implemented on neutron connector");
    }
    /// Predict the address of a contract
    /// returns the address and the salt that should be used.
    async fn get_address(
        &mut self,
        workflow_id: u64,
        contract_name: &str,
        extra_salt: &str,
    ) -> ConnectorResult<(String, Vec<u8>)>;
    /// Bridge account need specific information to create an account.
    async fn get_address_bridge(
        &mut self,
        sender_addr: &str,
        main_chain: &str,
        sender_chain: &str,
        receiving_chain: &str,
    ) -> ConnectorResult<String>;
    /// Instantiate an account based on the provided data
    async fn instantiate_account(&mut self, data: &InstantiateAccountData) -> ConnectorResult<()>;
    /// Instantiate a service contract based on the given data
    async fn instantiate_service(
        &mut self,
        service_id: u64,
        service_config: &ServiceConfig,
        salt: Vec<u8>,
    ) -> ConnectorResult<()>;
    /// Instantiate the authorization contract, only on the main domain for a workflow
    /// Currently Neutron is the only main domain we use, this might change in the future.
    /// CosmosCosmwasmConnector is the only connector that should implement it fully,
    /// while checking that this operation only happens on the main domain.
    /// Other connectors should return an error.
    /// Should return the address of the authorization contract.
    async fn instantiate_authorization(
        &mut self,
        workflow_id: u64,
        salt: Vec<u8>,
        processor_addr: String,
    ) -> ConnectorResult<()>;
    async fn change_authorization_owner(
        &mut self,
        authorization_addr: String,
        owner: String,
    ) -> ConnectorResult<()>;
    async fn instantiate_processor(
        &mut self,
        workflow_id: u64,
        salt: Vec<u8>,
        admin: String,
        polytone_addr: Option<valence_processor_utils::msg::PolytoneContracts>,
    ) -> ConnectorResult<()>;
    async fn add_external_domain(
        &mut self,
        main_domain: &str,
        domain: &str,
        authorrization_addr: String,
        processor_addr: String,
        processor_bridge_account_addr: String,
    ) -> ConnectorResult<()>;
    async fn instantiate_processor_bridge_account(
        &mut self,
        processor_addr: String,
        retry: u8,
    ) -> ConnectorResult<()>;
}
