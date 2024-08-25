use std::collections::HashMap;

use crate::{
    config::Config,
    domain::{Connector, Domain},
};

#[derive(Debug, Default)]
pub struct Context {
    connectors: HashMap<Domain, Box<dyn Connector>>,
    pub config: Config,
}

impl Context {
    /// Get the domain from ctx if exists
    /// otherwise it gets a new domain info and save it ctx
    pub async fn get_or_create_connector(&mut self, domain: &Domain) -> &mut Box<dyn Connector> {
        if !self.connectors.contains_key(domain) {
            let connector = domain.generate_connector(&self.config).await;
            self.connectors.insert(domain.clone(), connector);
        }

        self.connectors.get_mut(domain).unwrap()
    }
}
