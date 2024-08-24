pub mod cosmos_cw;
use std::{collections::HashMap, future::Future, pin::Pin};

use async_trait::async_trait;
use cosmos_cw::CosmosCwConnector;
use cosmos_grpc_client::cosmos_sdk_proto::cosmos::base::v1beta1::Coin;

use valence_macros::connector_trait;

use crate::{
    account::AccountType,
    config::{Cfg, ChainInfo},
};

pub type PinnedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// We need some way of knowing which domain we are talking with
/// TODO: chain connection, execution, bridges for authorization.
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Domain {
    CosmosCw(String),
    // Solana
}

#[derive(Debug, Clone)]
pub struct DomainInfo {
    pub connector: Box<dyn Connector>,
}

impl DomainInfo {
    pub async fn from_domain(cfg: &Cfg, domain: &Domain) -> Self {
        match domain {
            Domain::CosmosCw(chain_name) => {
                let connector = CosmosCwConnector::new(
                    cfg.get_chain_info(chain_name.clone()),
                    cfg.get_code_ids(chain_name),
                )
                .await;

                DomainInfo { connector }
            }
        }
    }
}

#[async_trait]
pub trait Connector {
    /// Create a new connectors that is connected to be used on a specific domain
    fn new(chain_info: ChainInfo, code_ids: HashMap<String, u64>) -> PinnedFuture<'static, Self>;
    /// Get the account address for a specific account type
    /// This must be the predicted (init2 in cosmos) address, we instantiate the contract later in the flow.
    fn get_account_addr(
        &self,
        cfg: &Cfg,
        account_id: u64,
        account_type: AccountType,
    ) -> PinnedFuture<String>;
    fn init_account(&mut self, account_type: &AccountType) -> PinnedFuture<String>;
    fn get_balance(&mut self, addr: String) -> PinnedFuture<Option<Coin>>;
}

// impl_connector!(CosmosCwConnector);
