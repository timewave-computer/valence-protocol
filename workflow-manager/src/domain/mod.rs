pub mod cosmos;
use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Mutex};

use cosmos::CosmosConnector;
use cosmos_grpc_client::{cosmos_sdk_proto::cosmos::base::v1beta1::Coin, StdError};

use lazy_static::lazy_static;
use valence_macros::connector_trait;

// Init cache for domains info which includes the connector and rpc endpoint
lazy_static! {
    static ref DOMAINS: Mutex<BTreeMap<Domain, DomainInfo>> = Mutex::new(BTreeMap::new());
}

pub type PinnedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// We need some way of knowing which domain we are talking with
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub enum Domain {
    Cosmos(String),
    // Solana
}

#[derive(Debug)]
pub struct DomainInfo {
    pub connector: ConnectorWrapper,
}

impl DomainInfo {
    pub async fn from_domain(domain: Domain) -> Self {
        match domain {
            Domain::Cosmos(_chain_name) => {
                // TODO: Get rpc / info for a specific domain somehow
                let connector = ConnectorWrapper::new::<CosmosConnector>(
                    "http://grpc-falcron.pion-1.ntrn.tech:80".to_string(),
                    "crazy into this wheel interest enroll basket feed fashion leave feed depth wish throw rack language comic hand family shield toss leisure repair kite".to_string(),
                ).await;

                DomainInfo { connector }
            }
        }
    }
}

#[connector_trait]
pub trait Connector {
    fn new(endpoint: String, wallet_mnemonic: String) -> PinnedFuture<'static, Self>;
    fn connect(&self) -> Result<(), StdError>;
    fn get_balance(&mut self, addr: String) -> PinnedFuture<Option<Coin>>;
}

impl_connector!(CosmosConnector);
