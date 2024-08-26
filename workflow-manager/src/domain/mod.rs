pub mod cosmos_cw;
use std::fmt;

use async_trait::async_trait;
use cosmos_cw::CosmosCosmwasmConnector;
use strum::Display;

use crate::{account::InstantiateAccountData, config::Config, service::ServiceConfig};

/// We need some way of knowing which domain we are talking with
/// TODO: chain connection, execution, bridges for authorization.
#[derive(Debug, Display, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Domain {
    CosmosCosmwasm(String),
    // Solana
}

impl Domain {
    pub async fn generate_connector(&self, cfg: &Config) -> Box<dyn Connector> {
        match self {
            Domain::CosmosCosmwasm(chain_name) => Box::new(
                CosmosCosmwasmConnector::new(
                    cfg.get_chain_info(chain_name.clone()),
                    cfg.get_code_ids(chain_name),
                )
                .await,
            ),
        }
    }
}

#[async_trait]
pub trait Connector: fmt::Debug {
    /// Predict the address of a contract
    /// returns the address and the salt that should be used.
    async fn predict_address(
        &mut self,
        account_id: &u64,
        contract_name: &str,
        extra_salt: &str,
    ) -> (String, Vec<u8>);
    /// Instantiate an account based onthe provided data
    async fn instantiate_account(&mut self, data: &InstantiateAccountData) -> ();
    async fn instantiate_service(
        &mut self,
        service_id: u64,
        service_config: &ServiceConfig,
        salt: Vec<u8>,
    ) -> ();
}
