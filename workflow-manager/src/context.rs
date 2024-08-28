use std::collections::HashMap;

use crate::{
    config::Config,
    domain::{Connector, Domain},
    error::{ManagerError, ManagerResult},
};

#[derive(Debug, Default)]
pub struct Context {
    connectors: HashMap<Domain, Box<dyn Connector>>,
    pub config: Config,
}

impl Context {
    /// Get the domain from ctx if exists
    /// otherwise it gets a new domain info and save it in context
    pub async fn get_or_create_connector(
        &mut self,
        domain: &Domain,
    ) -> ManagerResult<&mut Box<dyn Connector>> {
        if !self.connectors.contains_key(domain) {
            let connector = domain.generate_connector(&self.config).await?;
            self.connectors.insert(domain.clone(), connector);
        }

        self.connectors
            .get_mut(domain)
            .ok_or(ManagerError::generic_err(
                "Failed to get connector from context",
            ))
    }
}
