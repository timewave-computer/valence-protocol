use alloy::rpc::types::{TransactionReceipt, TransactionRequest};
use tonic::async_trait;

use crate::common::error::StrategistError;

use alloy::network::Ethereum;
use alloy::providers::{
    fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
    Identity, RootProvider,
};
use alloy::transports::http::{Client, Http};

pub type CustomProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RootProvider<Http<Client>>,
    Http<Client>,
    Ethereum,
>;

#[async_trait]
pub trait EvmBaseClient {
    async fn get_request_provider(&self) -> Result<CustomProvider, StrategistError>;

    async fn latest_block_height(&self) -> Result<u64, StrategistError>;

    async fn query_balance(&self, address: &str) -> Result<u128, StrategistError>;

    async fn execute_tx(
        &self,
        tx: TransactionRequest,
    ) -> Result<TransactionReceipt, StrategistError>;
}
