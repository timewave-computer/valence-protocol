use std::collections::HashMap;

use crate::{
    config::Cfg,
    domain::{Domain, DomainInfo},
};

#[derive(Debug, Default)]
pub struct Context {
    pub domain_infos: HashMap<Domain, DomainInfo>,
    pub config: Cfg,
}

impl Context {
    /// Get the domain from ctx if exists
    /// otherwise it gets a new domain info and save it ctx
    pub async fn get_or_create_domain_info(&mut self, domain: &Domain) -> &mut DomainInfo {
        if !self.domain_infos.contains_key(domain) {
            let domain_info = DomainInfo::from_domain(&self.config, domain).await;
            self.domain_infos.insert(domain.clone(), domain_info);
        }

        self.domain_infos.get_mut(domain).unwrap()
    }
}
