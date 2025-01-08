use alloy::{
    consensus::Account,
    network::Ethereum,
    primitives::{Address, TxHash, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider,
    },
    rpc::types::{Transaction, TransactionReceipt, TransactionRequest},
    transports::http::{reqwest::Url, Client, Http},
};
use std::error::Error;
use tokio::runtime::Runtime;

// Define the individual fillers in a nested structure
type BaseFillChain = JoinFill<NonceFiller, ChainIdFiller>;
type WithBlobGas = JoinFill<BlobGasFiller, BaseFillChain>;
type WithGas = JoinFill<GasFiller, WithBlobGas>;
type AllFillers = JoinFill<Identity, WithGas>;

pub struct EthClient {
    pub provider: FillProvider<AllFillers, RootProvider<Http<Client>>, Http<Client>, Ethereum>,
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

        Ok(Self { provider, rt })
    }

    pub fn get_block_number(&self) -> Result<u64, Box<dyn Error>> {
        let number = self.rt.block_on(self.provider.get_block_number())?;
        Ok(number)
    }

    pub fn get_balance(&self, address: Address) -> Result<U256, Box<dyn Error>> {
        let balance = self
            .rt
            .block_on(async { self.provider.get_balance(address).await })?;
        Ok(balance)
    }

    pub fn get_accounts_addresses(&self) -> Result<Vec<Address>, Box<dyn Error>> {
        let accounts = self
            .rt
            .block_on(async { self.provider.get_accounts().await })?;
        Ok(accounts)
    }

    pub fn get_account(&self, address: Address) -> Result<Account, Box<dyn Error>> {
        let account = self
            .rt
            .block_on(async { self.provider.get_account(address).await })?;
        Ok(account)
    }

    pub fn send_transaction(
        &self,
        tx: TransactionRequest,
    ) -> Result<TransactionReceipt, Box<dyn Error>> {
        self.rt.block_on(async {
            let tx_hash = self.provider.send_transaction(tx).await?;
            let receipt = tx_hash.get_receipt().await?;

            Ok(receipt)
        })
    }

    pub fn get_transaction_by_hash(
        &self,
        tx_hash: TxHash,
    ) -> Result<Option<Transaction>, Box<dyn Error>> {
        let tx = self
            .rt
            .block_on(async { self.provider.get_transaction_by_hash(tx_hash).await })?;
        Ok(tx)
    }
}
