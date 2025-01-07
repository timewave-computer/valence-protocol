use std::error::Error;

use alloy_primitives::{Address, U256};
use alloy_rpc_client::RpcClient;
use alloy_transport_http::{reqwest::Url, Client, Http};
use tokio::runtime::Runtime;

pub struct EthClient {
    client: RpcClient<Http<Client>>,
    rt: Runtime,
}

impl EthClient {
    pub fn new(url: &str) -> Result<Self, Box<dyn Error>> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
            
        let url = Url::parse(url)?;
        let transport = Http::new(url);
        let client = RpcClient::new(transport, true);
        
        Ok(Self { client, rt })
    }

    pub fn get_block_number(&self) -> Result<U256, Box<dyn Error>> {
        let number = self.rt.block_on(
            self.client.request("eth_blockNumber", ())
        )?;
        Ok(number)
    }

    pub fn get_balance(&self, address: Address) -> Result<U256, Box<dyn Error>> {
        let balance = self.rt.block_on(
            self.client.request(
                "eth_getBalance",
                (address, "latest")
            )
        )?;
        Ok(balance)
    }
}