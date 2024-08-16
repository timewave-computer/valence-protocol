/// We need some way of knowing which domain we are talking with
pub enum Domains {
    Comsos(String),
    // Solana
}

/// Given a domain, the bare minimum we need to know now, is the rpc endpoint of the domain.
/// later we will have a connector that implements the same interface for all domains we support
pub struct DomainInfo {
    pub rpc: String,
}

impl From<Domains> for DomainInfo {
    fn from(domain: Domains) -> DomainInfo {
        match domain {
            Domains::Comsos(_chain_name) => {
                // TODO: Get rpc / info for a specific domain somehow
                let rpc = "some_rpc".to_string();

                DomainInfo { rpc }
            }
        }
    }
}
