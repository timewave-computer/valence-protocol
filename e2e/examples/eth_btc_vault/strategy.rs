use valence_domain_clients::clients::{
    ethereum::EthereumClient, gaia::CosmosHubClient, neutron::NeutronClient,
};

// main strategy struct that wraps around the StrategyConfig
// and stores the initialized clients
pub struct Strategy {
    /// top level strategy configuration
    pub cfg: String,

    /// active ethereum client
    pub(crate) eth_client: EthereumClient,
    /// active cosmos hub client
    pub(crate) gaia_client: CosmosHubClient,
    /// active neutron client
    pub(crate) neutron_client: NeutronClient,
}
