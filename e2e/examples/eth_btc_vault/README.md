# Eth-BTC-Neutron Vault strategy

Strategy is separated in a domain-driven manner.

The strategist operates over multiple domains, hence the configuration
and setup follows the same domain-driven logic.

## Setting up the on-chain environment

In order for the strategist to operate the vault, each domain in the
environment must be made ready.

This is done in the following order:

`Neutron -> Cosmos Hub -> Ethereum`

The reason for that is the following.
To set up the Ethereum domain, we need to know the destination address
for the IBC Eureka transfer library. This address is the Cosmos Hub ICA
created by our Valence Interchain Account. In order to obtain the ICA
address, the entire Neutron program must be deployed and advanced to the
point where Cosmos Hub ICA is created and ready for operation.

Each domain setup will depend on some inputs and produce some outputs (
build artifacts).

For instance, Neutron program setup will require valid signer, node connection,
contract code IDs. As a result, it will produce a `.toml` file containing the
build artifacts. See `./neutron/deploy.rs`.

Once Neutron program build artifact is available, we can extract its relevant
fields and pass them to the ethereum deploy script.

Once all build artifacts are available, we can move to the next step - starting
the strategist.

> Note: cosmos hub (gaia) build artifacts are very minimal - only the node
connection information is required in order to observe the ICA state.

## Starting the strategist

In the usual situation, strategist should be started with the following method:

```rust
pub async fn from_files<P: AsRef<Path>>(
    neutron_path: P,
    gaia_path: P,
    eth_path: P,
) -> Result<Self, Box<dyn Error>> {
    let neutron_cfg = NeutronStrategyConfig::from_file(neutron_path)?;
    let eth_cfg = EthereumStrategyConfig::from_file(eth_path)?;
    let gaia_cfg = GaiaStrategyConfig::from_file(gaia_path)?;

    let strategy_cfg = StrategyConfig {
        ethereum: eth_cfg,
        neutron: neutron_cfg,
        gaia: gaia_cfg,
    };

    Self::new(strategy_cfg).await
}
```

Each domain is defined by its respective config file identified by a path that we pass
to this method. Under the hood, `from_files` calls into `Self::new` which initializes
the `valence-domain-clients` for each domain: neutron, gaia, and ethereum.

Following the setup, the returned object will expose `.start()` method. Calling `.start()`
will trigger an infinite loop which will continuously call `cycle()` implemented on the
same object.
