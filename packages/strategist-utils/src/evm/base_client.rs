use std::str::FromStr;

use crate::common::error::StrategistError;
use alloy::contract::{CallBuilder, CallDecoder};
use alloy::network::Ethereum;
use alloy::network::Network;
use alloy::primitives::{Address, U256};
use alloy::providers::{
    fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
    Identity, RootProvider,
};
use alloy::transports::Transport;

use alloy::providers::Provider;
use alloy::rpc::types::{TransactionReceipt, TransactionRequest};
use alloy::transports::http::{Client, Http};
use tonic::async_trait;

use super::request_provider_client::RequestProviderClient;

pub type CustomProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RootProvider<Http<Client>>,
    Http<Client>,
    Ethereum,
>;

/// base client trait with default implementations for evm based clients.
///
/// for chains which are somehow unique in their common module implementations,
/// these function definitions can be overridden to match that of the chain.
#[async_trait]
pub trait EvmBaseClient: RequestProviderClient {
    async fn latest_block_height(&self) -> Result<u64, StrategistError> {
        let client = self.get_request_provider().await?;

        let block = client.get_block_number().await?;

        Ok(block)
    }

    async fn query_balance(&self, address: &str) -> Result<U256, StrategistError> {
        let client = self.get_request_provider().await?;

        let addr = Address::from_str(address)?;
        let balance = client.get_balance(addr).await?;

        Ok(balance)
    }

    async fn execute_tx(
        &self,
        tx: TransactionRequest,
    ) -> Result<TransactionReceipt, StrategistError> {
        let client = self.get_request_provider().await?;

        let signed_tx = tx.from(self.signer().address());

        let tx_response = client
            .send_transaction(signed_tx)
            .await?
            .get_receipt()
            .await?;

        Ok(tx_response)
    }

    async fn query<'a, T, P, D, N>(
        &'a self,
        builder: CallBuilder<T, P, D, N>,
    ) -> Result<D::CallOutput, StrategistError>
    where
        T: Transport + Clone + Send + Sync + 'static,
        P: Provider<T, N> + Send + Sync + 'a,
        N: Network + Send + Sync + 'static,
        D: CallDecoder + Send + Sync + 'static,
        N::TransactionRequest: Into<TransactionRequest> + Send,
        CallBuilder<T, P, D, N>: Clone + Send + 'a,
        D::CallOutput: Send,
    {
        let client = self.get_request_provider().await?;

        let tx_request: TransactionRequest = builder.clone().into_transaction_request().into();

        let raw_response = client.call(&tx_request).await?;

        let decoded = builder.decode_output(raw_response, true)?;

        Ok(decoded)
    }
}
