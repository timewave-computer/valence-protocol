for specific domain 
```
impl ConnectorInner for CosmosConnector {
    fn connect(&self) -> Result<(), StdError> {
        // This can just call the implementation from the Connector trait
        <Self as Connector>::connect(self)
    }

    fn get_balance(
        &mut self,
        addr: String,
    ) -> PinnedFuture<Option<Coin>> {
        <Self as Connector>::get_balance(self, addr)
    }

    // Implement other methods similarly...
}

```

For the whole domain thing
```
pub trait Connector: ConnectorInner {
    fn new(
        endpoint: String,
        wallet_mnemonic: String,
    ) -> PinnedFuture<'static, Self>;
    fn connect(&self) -> Result<(), StdError>;
    fn get_balance(&mut self, addr: String) -> PinnedFuture<Option<Coin>>;
}

#[derive(Debug)]
pub struct ConnectorWrapper(Box<dyn ConnectorInner>);
// This Box contains a type that implements Connector, but we don't know which type at compile time

pub trait ConnectorInner: Send + Sync + std::fmt::Debug {
    fn connect(&self) -> Result<(), StdError>;
    fn get_balance(&mut self, addr: String) -> PinnedFuture<Option<Coin>>;
    // Other methods from Connector, except `new`...
}

impl ConnectorWrapper {
    pub async fn new<T>(endpoint: String, wallet_mnemonic: String) -> Self
    where
        T: Connector + ConnectorInner + 'static,
    {
        let connector = T::new(endpoint, wallet_mnemonic).await;
        ConnectorWrapper(Box::new(connector))
    }
}

impl ConnectorInner for ConnectorWrapper {
    fn connect(&self) -> Result<(), StdError> {
        self.0.connect()
    }

    fn get_balance(&mut self, addr: String) -> PinnedFuture<Option<Coin>> {
        self.0.get_balance(addr)
    }
    // Implement other methods similarly...
}

```