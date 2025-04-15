use tonic::async_trait;

use crate::{
    common::error::StrategistError,
    cosmos::{base_client::BaseClient, grpc_client::GrpcSigningClient, wasm_client::WasmClient},
};

const CHAIN_NAME: &str = "babylon";
const CHAIN_PREFIX: &str = "bbn";
const CHAIN_DENOM: &str = "ubbn";

pub struct BabylonClient {
    grpc_url: String,
    mnemonic: String,
    chain_id: String,
    chain_denom: String,
    gas_price: f64,
}

impl BabylonClient {
    pub async fn new(
        rpc_url: &str,
        rpc_port: &str,
        mnemonic: &str,
        chain_id: &str,
    ) -> Result<Self, StrategistError> {
        let avg_gas_price = Self::query_chain_gas_config(CHAIN_NAME, CHAIN_DENOM).await?;

        Ok(Self {
            grpc_url: format!("{rpc_url}:{rpc_port}"),
            mnemonic: mnemonic.to_string(),
            chain_id: chain_id.to_string(),
            chain_denom: CHAIN_DENOM.to_string(),
            gas_price: avg_gas_price,
        })
    }
}

#[async_trait]
impl BaseClient for BabylonClient {}

#[async_trait]
impl WasmClient for BabylonClient {}

#[async_trait]
impl GrpcSigningClient for BabylonClient {
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

    fn gas_price(&self) -> f64 {
        self.gas_price
    }

    fn gas_adjustment(&self) -> f64 {
        1.8
    }
}
