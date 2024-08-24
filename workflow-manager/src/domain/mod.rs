pub mod cosmos_cw;
use std::fmt;

use async_trait::async_trait;
use cosmos_cw::CosmosCwConnector;
use cosmos_grpc_client::cosmos_sdk_proto::cosmos::base::v1beta1::Coin;

use crate::{account::AccountType, config::Cfg};

/// We need some way of knowing which domain we are talking with
/// TODO: chain connection, execution, bridges for authorization.
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Domain {
    CosmosCw(String),
    // Solana
}

#[derive(Debug)]
pub struct DomainInfo {
    pub connector: Box<dyn Connector>,
}

impl DomainInfo {
    pub async fn from_domain(cfg: &Cfg, domain: &Domain) -> Self {
        match domain {
            Domain::CosmosCw(chain_name) => {
                let connector = Box::new(
                    CosmosCwConnector::new(
                        cfg.get_chain_info(chain_name.clone()),
                        cfg.get_code_ids(chain_name),
                    )
                    .await,
                );

                DomainInfo { connector }
            }
        }
    }
}

#[async_trait]
pub trait Connector: fmt::Debug {
    async fn get_account_addr(&mut self, account_id: u64, account_type: &AccountType) -> String;
    async fn init_account(&mut self, account_type: &AccountType) -> String;
    async fn get_balance(&mut self, addr: String) -> Option<Coin>;
}
