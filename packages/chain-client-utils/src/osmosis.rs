use tonic::async_trait;

use crate::cosmos::{
    base_client::BaseClient, grpc_client::GrpcSigningClient, wasm_client::WasmClient,
};

const CHAIN_PREFIX: &str = "osmo";

/// client for interacting with the osmosis chain
pub struct OsmosisClient {
    grpc_url: String,
    mnemonic: String,
    chain_id: String,
    chain_denom: String,
}

impl OsmosisClient {
    pub fn new(
        rpc_url: &str,
        rpc_port: &str,
        mnemonic: &str,
        chain_id: &str,
        chain_denom: &str,
    ) -> Self {
        Self {
            grpc_url: format!("{rpc_url}:{rpc_port}"),
            mnemonic: mnemonic.to_string(),
            chain_id: chain_id.to_string(),
            chain_denom: chain_denom.to_string(),
        }
    }
}

/// osmosis is a base cosmos chain
#[async_trait]
impl BaseClient for OsmosisClient {}

/// osmosis has a wasm module
#[async_trait]
impl WasmClient for OsmosisClient {}

#[async_trait]
impl GrpcSigningClient for OsmosisClient {
    fn grpc_url(&self) -> String {
        self.grpc_url.to_string()
    }

    fn mnemonic(&self) -> String {
        self.mnemonic.to_string()
    }

    fn chain_prefix(&self) -> String {
        CHAIN_PREFIX.to_string()
    }

    fn chain_id(&self) -> String {
        self.chain_id.to_string()
    }

    fn chain_denom(&self) -> String {
        self.chain_denom.to_string()
    }
}
