use alloy::{
    consensus::Account,
    primitives::{Address, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::{Transaction, TransactionRequest},
    transports::http::{reqwest::Url, Client, Http},
};
use std::error::Error;
use tokio::runtime::Runtime;

pub struct EthClient {
    pub provider: Box<dyn Provider<Http<Client>>>,
    pub rt: Runtime,
}

impl EthClient {
    pub fn new(url: &str) -> Result<Self, Box<dyn Error>> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let url = Url::parse(url)?;
        let provider = ProviderBuilder::new()
            // Adds the `ChainIdFiller`, `GasFiller` and the `NonceFiller` layers.
            // This is the recommended way to set up the provider.
            .with_recommended_fillers()
            .on_http(url);

        Ok(Self {
            provider: Box::new(provider),
            rt,
        })
    }

    pub fn get_block_number(&self) -> Result<u64, Box<dyn Error>> {
        let number = self.rt.block_on(self.provider.get_block_number())?;
        Ok(number)
    }

    pub fn get_balance(&self, address: Address) -> Result<U256, Box<dyn Error>> {
        let balance = self.rt.block_on(async {
            let balance = self.provider.get_balance(address).await;
            balance
        })?;
        Ok(balance)
    }

    pub fn get_accounts_addresses(&self) -> Result<Vec<Address>, Box<dyn Error>> {
        let accounts = self.rt.block_on(async {
            let accounts = self.provider.get_accounts().await;
            accounts
        })?;
        Ok(accounts)
    }

    pub fn get_account(&self, address: Address) -> Result<Account, Box<dyn Error>> {
        let account = self.rt.block_on(async {
            let account = self.provider.get_account(address).await;
            account
        })?;
        Ok(account)
    }

    pub fn send_transaction(&self, tx: TransactionRequest) -> Result<FixedBytes<32>, Box<dyn Error>> {
        let tx_hash = self
            .rt
            .block_on(async {
                let tx_hash = self.provider.send_transaction(tx).await;
                tx_hash
            })?
            .tx_hash()
            .clone();
        Ok(tx_hash)
    }

    pub fn get_transaction_by_hash(&self, tx_hash: FixedBytes<32>) -> Result<Option<Transaction>, Box<dyn Error>> {
        let tx = self.rt.block_on(async {
            let tx = self.provider.get_transaction_by_hash(tx_hash).await;
            tx
        })?;
        Ok(tx)
    }
}
