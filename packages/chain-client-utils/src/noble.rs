use cosmrs::Any;
use log::info;
use tonic::async_trait;

use crate::{
    common::{error::StrategistError, transaction::TransactionResponse},
    cosmos::{base_client::BaseClient, grpc_client::GrpcSigningClient, CosmosServiceClient},
};

const CHAIN_PREFIX: &str = "noble";
const CHAIN_DENOM: &str = "uusdc";
const CCTP_MODULE_NAME: &str = "cctp";
// u128::max as str
const ALLOWANCE: &str = "340282366920938463463374607431768211455";
const DUMMY_ADDRESS: &[u8; 32] = &[0x01; 32];

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

    /// Sets up the noble client for testing the burn functionality by:
    /// - Configuring the module account as a minter controller.
    /// - Configuring the module account as a minter with a specified allowance.
    /// - Adding a remote dummy token messenger.
    /// - Linking a local token with a remote dummy token.
    pub async fn set_up_test_environment(&self, sender: &str, domain_id: u32, denom: &str) {
        // First get the module account for the cctp module
        let cctp_module_account_address = self
            .query_module_account(CCTP_MODULE_NAME)
            .await
            .unwrap()
            .base_account
            .unwrap()
            .address;

        // Configure the module account as a minter controller
        let tx_response = self
            .configure_minter_controller(sender, sender, &cctp_module_account_address)
            .await
            .unwrap();
        info!("Minter controller configured response: {:?}", tx_response);
        self.poll_for_tx(&tx_response.hash).await.unwrap();

        // Configure the module account as a minter with a large mint allowance
        let tx_response = self
            .configure_minter(sender, &cctp_module_account_address, ALLOWANCE, denom)
            .await
            .unwrap();
        info!("Minter configured response: {:?}", tx_response);
        self.poll_for_tx(&tx_response.hash).await.unwrap();

        // Add a remote token messenger address for the given domain_id.
        // Any address will do as this is for testing the burn functionality.
        let tx_response = self
            .add_remote_token_messenger(sender, domain_id, DUMMY_ADDRESS)
            .await;

        match tx_response {
            Ok(response) => {
                self.poll_for_tx(&response.hash).await.unwrap();
                info!("Remote token messenger added response: {:?}", response);
            }
            Err(_) => {
                info!("Remote token messenger already added!");
            }
        }

        // Link the local token with a remote token.
        // Any remote token will do for testing.
        let tx_response = self
            .link_token_pair(sender, domain_id, DUMMY_ADDRESS, denom)
            .await;
        match tx_response {
            Ok(response) => {
                self.poll_for_tx(&response.hash).await.unwrap();
                info!("Token pair linked response: {:?}", response);
            }
            Err(_) => {
                info!("Token pair already linked!");
            }
        }
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

        TransactionResponse::try_from(broadcast_tx_response.tx_response)
    }

    async fn configure_minter_controller(
        &self,
        sender: &str,
        controller: &str,
        minter: &str,
    ) -> Result<TransactionResponse, StrategistError> {
        let signing_client = self.get_signing_client().await?;

        let configure_minter_controller_msg = MsgConfigureMinterController {
            from: sender.to_string(),
            controller: controller.to_string(),
            minter: minter.to_string(),
        };

        let any_msg = Any::from_msg(&configure_minter_controller_msg)?;

        let simulation_response = self.simulate_tx(any_msg.clone()).await?;
        let fee = self.get_tx_fee(simulation_response)?;

        let raw_tx = signing_client.create_tx(any_msg, fee, None).await?;

        let channel = self.get_grpc_channel().await?;

        let mut grpc_client = CosmosServiceClient::new(channel);

        let broadcast_tx_response = grpc_client.broadcast_tx(raw_tx).await?.into_inner();

        TransactionResponse::try_from(broadcast_tx_response.tx_response)
    }

    async fn configure_minter(
        &self,
        sender: &str,
        address: &str,
        allowance: &str,
        denom: &str,
    ) -> Result<TransactionResponse, StrategistError> {
        let signing_client = self.get_signing_client().await?;

        let configure_minter_msg = MsgConfigureMinter {
            from: sender.to_string(),
            address: address.to_string(),
            allowance: Some(cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
                denom: denom.to_string(),
                amount: allowance.to_string(),
            }),
        };

        let any_msg = Any::from_msg(&configure_minter_msg)?;

        let simulation_response = self.simulate_tx(any_msg.clone()).await?;
        let fee = self.get_tx_fee(simulation_response)?;

        let raw_tx = signing_client.create_tx(any_msg, fee, None).await?;

        let channel = self.get_grpc_channel().await?;

        let mut grpc_client = CosmosServiceClient::new(channel);

        let broadcast_tx_response = grpc_client.broadcast_tx(raw_tx).await?.into_inner();

        TransactionResponse::try_from(broadcast_tx_response.tx_response)
    }

    async fn add_remote_token_messenger(
        &self,
        signer: &str,
        domain_id: u32,
        address: &[u8],
    ) -> Result<TransactionResponse, StrategistError> {
        let signing_client = self.get_signing_client().await?;

        let add_remote_token_messenger_msg = MsgAddRemoteTokenMessenger {
            from: signer.to_string(),
            domain_id,
            address: address.to_vec(),
        };

        let any_msg = Any::from_msg(&add_remote_token_messenger_msg)?;

        let simulation_response = self.simulate_tx(any_msg.clone()).await?;
        let fee = self.get_tx_fee(simulation_response)?;

        let raw_tx = signing_client.create_tx(any_msg, fee, None).await?;

        let channel = self.get_grpc_channel().await?;

        let mut grpc_client = CosmosServiceClient::new(channel);

        let broadcast_tx_response = grpc_client.broadcast_tx(raw_tx).await?.into_inner();

        TransactionResponse::try_from(broadcast_tx_response.tx_response)
    }

    async fn link_token_pair(
        &self,
        signer: &str,
        remote_domain: u32,
        remote_token: &[u8],
        local_token: &str,
    ) -> Result<TransactionResponse, StrategistError> {
        let signing_client = self.get_signing_client().await?;

        let link_token_pair_msg = MsgLinkTokenPair {
            from: signer.to_string(),
            remote_domain,
            remote_token: remote_token.to_vec(),
            local_token: local_token.to_string(),
        };

        let any_msg = Any::from_msg(&link_token_pair_msg)?;

        let simulation_response = self.simulate_tx(any_msg.clone()).await?;
        let fee = self.get_tx_fee(simulation_response)?;

        let raw_tx = signing_client.create_tx(any_msg, fee, None).await?;

        let channel = self.get_grpc_channel().await?;

        let mut grpc_client = CosmosServiceClient::new(channel);

        let broadcast_tx_response = grpc_client.broadcast_tx(raw_tx).await?.into_inner();

        TransactionResponse::try_from(broadcast_tx_response.tx_response)
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
        "circle.fiattokenfactory.v1.MsgMint".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/circle.fiattokenfactory.v1.MsgMint".into()
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgConfigureMinterController {
    /// the sender address
    #[prost(string, tag = "1")]
    pub from: ::prost::alloc::string::String,
    /// the controller address
    #[prost(string, tag = "2")]
    pub controller: ::prost::alloc::string::String,
    /// the minter address
    #[prost(string, tag = "3")]
    pub minter: ::prost::alloc::string::String,
}

impl ::prost::Name for MsgConfigureMinterController {
    const NAME: &'static str = "MsgConfigureMinterController";
    const PACKAGE: &'static str = "circle.fiattokenfactory.v1";
    fn full_name() -> ::prost::alloc::string::String {
        "circle.fiattokenfactory.v1.MsgConfigureMinterController".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/circle.fiattokenfactory.v1.MsgConfigureMinterController".into()
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgConfigureMinter {
    /// the sender address
    #[prost(string, tag = "1")]
    pub from: ::prost::alloc::string::String,
    /// the minter address
    #[prost(string, tag = "2")]
    pub address: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "3")]
    /// how much the minter can mint
    pub allowance: ::core::option::Option<cosmos_sdk_proto::cosmos::base::v1beta1::Coin>,
}

impl ::prost::Name for MsgConfigureMinter {
    const NAME: &'static str = "MsgConfigureMinter";
    const PACKAGE: &'static str = "circle.fiattokenfactory.v1";
    fn full_name() -> ::prost::alloc::string::String {
        "circle.fiattokenfactory.v1.MsgConfigureMinter".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/circle.fiattokenfactory.v1.MsgConfigureMinter".into()
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgAddRemoteTokenMessenger {
    /// the signer address
    #[prost(string, tag = "1")]
    pub from: ::prost::alloc::string::String,
    /// the domain ID
    #[prost(uint32, tag = "2")]
    pub domain_id: u32,
    /// the remote token messenger address
    #[prost(bytes, tag = "3")]
    pub address: ::prost::alloc::vec::Vec<u8>,
}

impl ::prost::Name for MsgAddRemoteTokenMessenger {
    const NAME: &'static str = "MsgAddRemoteTokenMessenger";
    const PACKAGE: &'static str = "circle.cctp.v1";
    fn full_name() -> ::prost::alloc::string::String {
        "circle.cctp.v1.MsgAddRemoteTokenMessenger".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/circle.cctp.v1.MsgAddRemoteTokenMessenger".into()
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgLinkTokenPair {
    /// the signer address
    #[prost(string, tag = "1")]
    pub from: ::prost::alloc::string::String,
    /// the remote domain ID
    #[prost(uint32, tag = "2")]
    pub remote_domain: u32,
    /// the remote token address
    #[prost(bytes, tag = "3")]
    pub remote_token: ::prost::alloc::vec::Vec<u8>,
    /// the local token denom
    #[prost(string, tag = "4")]
    pub local_token: ::prost::alloc::string::String,
}

impl ::prost::Name for MsgLinkTokenPair {
    const NAME: &'static str = "MsgLinkTokenPair";
    const PACKAGE: &'static str = "circle.cctp.v1";
    fn full_name() -> ::prost::alloc::string::String {
        "circle.cctp.v1.MsgLinkTokenPair".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/circle.cctp.v1.MsgLinkTokenPair".into()
    }
}
