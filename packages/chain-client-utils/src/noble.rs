use cosmrs::Any;
use tonic::async_trait;

use crate::{
    common::{error::StrategistError, transaction::TransactionResponse},
    cosmos::{base_client::BaseClient, grpc_client::GrpcSigningClient, CosmosServiceClient},
};

const CHAIN_PREFIX: &str = "noble";
const CHAIN_DENOM: &str = "uusdc";

/// client for interacting with the noble chain
pub struct NobleClient {
    grpc_url: String,
    mnemonic: String,
    chain_id: String,
    chain_denom: String,
    chain_prefix: String,
    gas_price: f64,
}

impl NobleClient {
    pub async fn new(
        rpc_url: &str,
        rpc_port: &str,
        mnemonic: &str,
        chain_id: &str,
        chain_denom: &str,
    ) -> Result<Self, StrategistError> {
        let avg_gas_price = Self::query_chain_gas_config("noble", CHAIN_DENOM).await?;

        Ok(Self {
            grpc_url: format!("{rpc_url}:{rpc_port}"),
            mnemonic: mnemonic.to_string(),
            chain_id: chain_id.to_string(),
            chain_denom: chain_denom.to_string(),
            chain_prefix: CHAIN_PREFIX.to_string(),
            gas_price: avg_gas_price,
        })
    }

    pub async fn mint_fiat(
        &self,
        sender: &str,
        receiver: &str,
        amount: &str,
        denom: &str,
    ) -> Result<TransactionResponse, StrategistError> {
        let signing_client = self.get_signing_client().await?;

        let mint_msg = MsgMint {
            from: sender.to_string(),
            address: receiver.to_string(),
            amount: Some(cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
                denom: denom.to_string(),
                amount: amount.to_string(),
            }),
        };

        let any_msg = Any::from_msg(&mint_msg)?;

        let simulation_response = self.simulate_tx(any_msg.clone()).await?;
        let fee = self.get_tx_fee(simulation_response)?;

        let raw_tx = signing_client.create_tx(any_msg, fee, None).await?;

        let channel = self.get_grpc_channel().await?;

        let mut grpc_client = CosmosServiceClient::new(channel);

        let broadcast_tx_response = grpc_client.broadcast_tx(raw_tx).await?.into_inner();

        match broadcast_tx_response.tx_response {
            Some(tx_response) => Ok(TransactionResponse::try_from(tx_response)?),
            None => Err(StrategistError::TransactionError("failed".to_string())),
        }
    }
}

/// noble is a base cosmos chain
#[async_trait]
impl BaseClient for NobleClient {}

#[async_trait]
impl GrpcSigningClient for NobleClient {
    fn grpc_url(&self) -> String {
        self.grpc_url.to_string()
    }

    fn mnemonic(&self) -> String {
        self.mnemonic.to_string()
    }

    fn chain_prefix(&self) -> String {
        self.chain_prefix.to_string()
    }

    fn chain_id(&self) -> String {
        self.chain_id.to_string()
    }

    fn chain_denom(&self) -> String {
        self.chain_denom.to_string()
    }

    fn gas_price(&self) -> f64 {
        self.gas_price
    }

    fn gas_adjustment(&self) -> f64 {
        1.8
    }
}

// Proto definitions to interact with noble
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgMint {
    /// the sender address
    #[prost(string, tag = "1")]
    pub from: ::prost::alloc::string::String,
    /// the recipient address
    #[prost(string, tag = "2")]
    pub address: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "3")]
    /// the coin to mint
    pub amount: ::core::option::Option<cosmos_sdk_proto::cosmos::base::v1beta1::Coin>,
}

impl ::prost::Name for MsgMint {
    const NAME: &'static str = "MsgMint";
    const PACKAGE: &'static str = "circle.fiattokenfactory.v1";
    fn full_name() -> ::prost::alloc::string::String {
        "ibc.applications.transfer.v1.MsgMint".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/circle.fiattokenfactory.v1.MsgMint".into()
    }
}
