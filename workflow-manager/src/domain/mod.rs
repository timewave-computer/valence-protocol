pub mod cosmos_cw;
use std::{future::Future, pin::Pin};

use cosmos_cw::CosmosCwConnector;
use cosmos_grpc_client::cosmos_sdk_proto::cosmos::base::v1beta1::Coin;

use valence_macros::connector_trait;

use crate::{
    account::AccountType,
    config::{Cfg, ChainInfo},
};

pub type PinnedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// We need some way of knowing which domain we are talking with
/// TODO: chain connection, execution, bridges for authorization.
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Domain {
    CosmosCw(String),
    // Solana
}

#[derive(Debug, Clone)]
pub struct DomainInfo {
    pub connector: ConnectorWrapper,
}

impl DomainInfo {
    pub async fn from_domain(cfg: &Cfg, domain: &Domain) -> Self {
        match domain {
            Domain::CosmosCw(chain_name) => {
                // TODO: Get rpc / info for a specific domain somehow
                let connector = ConnectorWrapper::new::<CosmosCwConnector>(
                    cfg.get_chain_info(chain_name.clone()),
                )
                .await;

                DomainInfo { connector }
            }
        }
    }
}

#[connector_trait]
pub trait Connector {
    fn new(chain_info: ChainInfo) -> PinnedFuture<'static, Self>;
    fn init_account(&mut self, account_type: &AccountType) -> PinnedFuture<String>;
    fn get_balance(&mut self, addr: String) -> PinnedFuture<Option<Coin>>;
}

impl_connector!(CosmosCwConnector);
