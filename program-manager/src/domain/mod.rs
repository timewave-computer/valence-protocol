pub mod cosmos_cw;
// pub mod cosmos_evm;

use std::fmt;

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use cosmos_cw::{CosmosCosmwasmConnector, CosmosCosmwasmError};

use cosmwasm_schema::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// use cosmos_evm::CosmosEvmError;
use thiserror::Error;
use valence_authorization_utils::authorization::AuthorizationInfo;

use crate::{
    account::InstantiateAccountData, config::ConfigError, library::LibraryConfig,
    program_config::ProgramConfig,
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
#[derive(
    Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[schemars(crate = "cosmwasm_schema::schemars")]
pub enum Domain {
    CosmosCosmwasm(String),
    // CosmosEvm(String),
    // Solana
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // IMPORTANT: to get from_string, we need to separate everything using ":"
        match self {
            Domain::CosmosCosmwasm(chain_name) => write!(f, "CosmosCosmwasm:{}", chain_name),
            // Domain::CosmosEvm(chain_name) => write!(f, "CosmosEvm:{}", chain_name),
        }
    }
}

impl Domain {
    pub fn from_string(input: String) -> Result<Domain, anyhow::Error> {
        let mut split = input.split(':');

        let domain = split.next().context("Domain is missing")?;

        match domain {
            "CosmosCosmwasm" => Ok(Domain::CosmosCosmwasm(
                split
                    .next()
                    .context("CosmosCosmwasm Domain missing chain name")?
                    .to_string(),
            )),
            // "CosmosEvm" => Ok(Domain::CosmosEvm(
            //     split
            //         .next()
            //         .context("CosmosCosmwasm Domain missing chain name")?
            //         .to_string(),
            // )),
            s => Err(anyhow!(format!(
                "Failed to parse domain from string: {}",
                s
            ))),
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
            Domain::CosmosCosmwasm(chain_name) => {
                Box::new(CosmosCosmwasmConnector::new(chain_name.as_str()).await?)
            } // Domain::CosmosEvm(_) => {
              //     return Err(ConnectorError::ConfigError(
              //         ConfigError::ChainBridgeNotFound("test".to_string()),
              //     ))
              // }
        })
    }
}

#[async_trait]
pub trait Connector: fmt::Debug + Send + Sync {
    /// Predict the address of a contract
    /// returns the address and the salt that should be used.
    async fn get_address(
        &mut self,
        program_id: u64,
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
        program_id: u64,
        processor_addr: String,
        data: &InstantiateAccountData,
    ) -> ConnectorResult<()>;

    /// Instantiate a library contract based on the given data
    async fn instantiate_library(
        &mut self,
        program_id: u64,
        processor_addr: String,
        library_id: u64,
        library_config: LibraryConfig,
        salt: Vec<u8>,
    ) -> ConnectorResult<()>;

    /// Instantiate a processor contract
    async fn instantiate_processor(
        &mut self,
        program_id: u64,
        salt: Vec<u8>,
        admin: String,
        authorization: String,
        polytone_config: Option<valence_processor_utils::msg::PolytoneContracts>,
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

    // Verify the library has an address and it was instantiated
    async fn verify_library(&mut self, library_addr: Option<String>) -> ConnectorResult<()>;

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
    async fn reserve_program_id(&mut self) -> ConnectorResult<u64> {
        unimplemented!("'reserve_program_id' should only be implemented on neutron domain");
    }

    /// Instantiate the authorization contract, only on the main domain for a program
    /// Currently Neutron is the only main domain we use, this might change in the future.
    /// CosmosCosmwasmConnector is the only connector that should implement it fully,
    /// while checking that this operation only happens on the main domain.
    /// Other connectors should return an error.
    /// Should return the address of the authorization contract.
    #[allow(unused_variables)]
    async fn instantiate_authorization(
        &mut self,
        program_id: u64,
        salt: Vec<u8>,
        processor_addr: String,
    ) -> ConnectorResult<()> {
        unimplemented!("'instantiate_authorization' should only be implemented on main domain");
    }

    /// Add authorizations to the authorization contract
    #[allow(unused_variables)]
    async fn add_authorizations(
        &mut self,
        authorization_addr: String,
        authorizations: Vec<AuthorizationInfo>,
    ) -> ConnectorResult<()> {
        unimplemented!("'add_authorizations' should only be implemented on main domain");
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
    async fn query_program_registry(
        &mut self,
        id: u64,
    ) -> ConnectorResult<valence_program_registry_utils::ProgramResponse> {
        unimplemented!("'query_program_registry' should only be implemented on neutron domain");
    }

    #[allow(unused_variables)]
    async fn verify_authorization_addr(&mut self, addr: String) -> ConnectorResult<()> {
        unimplemented!("'verify_authorization_addr' should only be implemented on neutron domain");
    }

    #[allow(unused_variables)]
    async fn save_program_config(&mut self, config: ProgramConfig) -> ConnectorResult<()> {
        unimplemented!("'save_program_config' should only be implemented on neutron domain");
    }

    #[allow(unused_variables)]
    async fn update_program_config(&mut self, config: ProgramConfig) -> ConnectorResult<()> {
        unimplemented!("'update_program_config' should only be implemented on neutron domain");
    }

    #[allow(unused_variables)]
    async fn get_program_config(&mut self, id: u64) -> ConnectorResult<ProgramConfig> {
        unimplemented!("'get_program_config' should only be implemented on neutron domain");
    }

}
