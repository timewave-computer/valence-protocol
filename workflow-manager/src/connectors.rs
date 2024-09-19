use dashmap::DashMap;

use crate::{
    domain::{Connector, Domain},
    error::{ManagerError, ManagerResult},
};

#[derive(Debug, Default)]
pub struct Connectors {
    connectors: DashMap<Domain, Box<dyn Connector>>,
}

impl Connectors {
    /// Get the domain from ctx if exists
    /// otherwise it gets a new domain connector and save it in cache
    pub async fn get_or_create_connector(
        &self,
        domain: &Domain,
    ) -> ManagerResult<dashmap::mapref::one::RefMut<'_, Domain, Box<dyn Connector>>> {
        if !self.connectors.contains_key(domain) {
            let connector = domain.generate_connector().await?;
            self.connectors.insert(domain.clone(), connector);
        }

        self.connectors
            .get_mut(domain)
            .ok_or(ManagerError::generic_err(
                "Failed to get connector from cache",
            ))
    }
}
