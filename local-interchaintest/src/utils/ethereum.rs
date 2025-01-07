use alloy::{
    primitives::{Address, U256}, providers::{Provider, ProviderBuilder, RootProvider}, transports::http::{reqwest::Url, Client, Http}
};
use std::error::Error;
use tokio::runtime::Runtime;

pub struct EthClient {
    provider: RootProvider<Http<Client>>,
    rt: Runtime,
}

impl EthClient {
    pub fn new(url: &str) -> Result<Self, Box<dyn Error>> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let url = Url::parse(url)?;
        let provider = ProviderBuilder::new().on_http(url);

        Ok(Self { provider, rt })
    }

    pub fn get_block_number(&self) -> Result<u64, Box<dyn Error>> {
        let number = self
            .rt
            .block_on(self.provider.get_block_number())?;
        Ok(number)
    }
    
    pub fn get_balance(&self, address: Address) -> Result<U256, Box<dyn Error>> {
        let balance = self.rt.block_on(async {
            let balance = self.provider.get_balance(address).await;
            balance
        })?;
        Ok(balance)
    }
}
