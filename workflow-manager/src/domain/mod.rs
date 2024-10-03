pub mod cosmos_cw;
// pub mod cosmos_evm;

use std::fmt;

use anyhow::anyhow;
use async_trait::async_trait;
use cosmos_cw::{CosmosCosmwasmConnector, CosmosCosmwasmError};

use serde::{Deserialize, Serialize};

// use cosmos_evm::CosmosEvmError;
use thiserror::Error;

use crate::{
    account::InstantiateAccountData,
    config::{ConfigError, CONFIG},
    service::ServiceConfig,
    workflow_config::WorkflowConfig,
};

pub type ConnectorResult<T> = Result<T, ConnectorError>;

pub const POLYTONE_TIMEOUT: u64 = 300;

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
/// chain connection, execution, bridges for authorization.
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    CosmosCosmwasm(String),
    // CosmosEvm(&'static str),
    // Solana
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // IMPORTANT: to get fromStr, we need to separate everything using ":"
        match self {
            Domain::CosmosCosmwasm(chain_name) => write!(f, "{}:{}", "CosmosCosmwasm", chain_name),
            // Domain::CosmosEvm(chain_name) => write!(f, "{}", chain_name),
        }
    }
}

impl Domain {
    pub fn from_string(input: String) -> Result<Domain, anyhow::Error> {
        let mut split = input.split(":");

        let _cosmos_cw = Domain::CosmosCosmwasm("".to_string()).to_string();

        match split.next() {
            Some(_cosmos_cw) => Ok(Domain::CosmosCosmwasm(split.next().unwrap().to_string())),
            // "CosmosEvm" => Ok(Domain::CosmosEvm(split[1])),
            _ => Err(anyhow!("Failed to parse domain from string")),
        }
    }

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
    async fn instantiate_account(
        &mut self,
        workflow_id: u64,
        processor_addr: String,
        data: &InstantiateAccountData,
    ) -> ConnectorResult<()>;

    /// Instantiate a service contract based on the given data
    async fn instantiate_service(
        &mut self,
        workflow_id: u64,
        auth_addr: String,
        processor_addr: String,
        service_id: u64,
        service_config: ServiceConfig,
        salt: Vec<u8>,
    ) -> ConnectorResult<()>;

    /// Instantiate a processor contract
    async fn instantiate_processor(
        &mut self,
        workflow_id: u64,
        salt: Vec<u8>,
        admin: String,
        polytone_addr: Option<valence_processor_utils::msg::PolytoneContracts>,
    ) -> ConnectorResult<()>;

    /// We need to do 2 things here:
    /// 1. Instantiate the bridge account
    /// 2. Verify it was created
    ///
    /// For polytone, we create account it when we instantiate the processor contract, but because its async
    /// we needd to verify that it was created.
    async fn instantiate_processor_bridge_account(
        &mut self,
        processor_addr: String,
        retry: u8,
    ) -> ConnectorResult<()>;

    /// Verify the account was instantiated correct and its one of our accounts
    async fn verify_account(&mut self, account_addr: String) -> ConnectorResult<()>;

    // Verify the service has an address and it was instantiated
    async fn verify_service(&mut self, service_addr: Option<String>) -> ConnectorResult<()>;

    // Verify the processor was instantiated
    async fn verify_processor(&mut self, processor_addr: String) -> ConnectorResult<()>;

    // Verify the bridge account was instantiated
    async fn verify_bridge_account(&mut self, bridge_addr: String) -> ConnectorResult<()>;

    // ---------------------------------------------------------------------------------------
    // Below are functions that sohuld only be implemented on a specific domain
    // For example authorization contract methods should only be implemented on the main domain
    // And they should have a default to prevent other connectors the need to implement them.

    /// We want this function to only be implemented on neutron connector
    /// We provide a defualt implemention that errors out if it is used on a different connector.
    #[allow(unused_variables)]
    async fn reserve_workflow_id(&mut self) -> ConnectorResult<u64> {
        unimplemented!("'reserve_workflow_id' should only be implemented on neutron domain");
    }

    /// Instantiate the authorization contract, only on the main domain for a workflow
    /// Currently Neutron is the only main domain we use, this might change in the future.
    /// CosmosCosmwasmConnector is the only connector that should implement it fully,
    /// while checking that this operation only happens on the main domain.
    /// Other connectors should return an error.
    /// Should return the address of the authorization contract.
    #[allow(unused_variables)]
    async fn instantiate_authorization(
        &mut self,
        workflow_id: u64,
        salt: Vec<u8>,
        processor_addr: String,
    ) -> ConnectorResult<()> {
        unimplemented!("'instantiate_authorization' should only be implemented on main domain");
    }

    /// We need to instantiate the bridge accounts for the authorization contract on all other domains
    /// There are 2 things we need to here:
    /// 1. Instantiate the bridge account
    /// 2. Verify it was created
    ///
    /// For polytone, we create account when we add the external domain but because its async
    /// we still need to verify it was created, so this is what we will be doing for polytone
    #[allow(unused_variables)]
    async fn instantiate_authorization_bridge_account(
        &mut self,
        authorization_addr: String,
        domain: String,
        retry: u8,
    ) -> ConnectorResult<()> {
        unimplemented!(
            "'instantiate_authorization_bridge_account' should only be implemented on main domain"
        );
    }

    /// Add an external domain to the processor contract
    /// This is only called on the authorization contract, so will only be called on the main domain
    #[allow(unused_variables)]
    async fn add_external_domain(
        &mut self,
        main_domain: &str,
        domain: &str,
        authorization_addr: String,
        processor_addr: String,
        processor_bridge_account_addr: String,
    ) -> ConnectorResult<()> {
        unimplemented!("'add_external_domain' should only be implemented on main domain");
    }

    /// Change the owner of the authorization contract
    /// This will only be called on our main domain as there is where our authorization contract is
    #[allow(unused_variables)]
    async fn change_authorization_owner(
        &mut self,
        authorization_addr: String,
        owner: String,
    ) -> ConnectorResult<()> {
        unimplemented!("'change_authorization_owner' should only be implemented on main domain");
    }

    #[allow(unused_variables)]
    async fn query_workflow_registry(
        &mut self,
        main_domain: &str,
        id: u64,
    ) -> ConnectorResult<valence_workflow_registry_utils::WorkflowResponse> {
        unimplemented!("'query_workflow_registry' should only be implemented on neutron domain");
    }

    #[allow(unused_variables)]
    async fn verify_authorization_addr(&mut self, addr: String) -> ConnectorResult<()> {
        unimplemented!("'verify_authorization_addr' should only be implemented on neutron domain");
    }

    #[allow(unused_variables)]
    async fn save_workflow_config(&mut self, config: WorkflowConfig) -> ConnectorResult<()> {
        unimplemented!("'save_workflow_config' should only be implemented on neutron domain");
    }
}
