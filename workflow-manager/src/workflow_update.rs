use std::collections::BTreeMap;

use cw_ownable::Expiration;
use serde::{Deserialize, Serialize};
use valence_authorization_utils::authorization::{AuthorizationInfo, Priority};
use valence_service_utils::Id;

use crate::{
    account::AccountInfoUpdate,
    connectors::Connectors,
    domain::Domain,
    error::{ManagerError, ManagerResult},
    service::ServiceInfoUpdate,
    workflow_config::{Link, WorkflowConfig},
    NEUTRON_CHAIN,
};

/// The job of the update, is to output a set of instructions to the user to update the workflow configuration.  
/// The user can only update service configs and authorizations.

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkflowConfigUpdate {
    /// This is the id of the workflow
    /// Required for update
    pub id: u64,
    /// New owner, if the owner is to be updated
    pub owner: Option<String>,
    /// The list service data by id
    pub services: BTreeMap<Id, ServiceInfoUpdate>,
    /// A list of authorizations
    pub authorizations: Vec<AuthorizationInfoUpdate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthorizationInfoUpdate {
    Add(AuthorizationInfo),
    Modify {
        label: String,
        not_before: Option<Expiration>,
        expiration: Option<Expiration>,
        max_concurrent_executions: Option<u64>,
        priority: Option<Priority>,
    },
    /// Disable by label
    Disable(String),
    /// Disable by label
    Enable(String),
}

pub struct UpdateResponse {
    pub instructions: Vec<String>,
    pub warnings: Vec<String>,
}

impl WorkflowConfigUpdate {
    /// Modify an existing config with a new config
    pub async fn update(&mut self, connectors: &Connectors) -> ManagerResult<()> {
        let neutron_domain = Domain::CosmosCosmwasm(NEUTRON_CHAIN.to_string());

        // get the old workflow config from registry
        let mut neutron_connector = connectors.get_or_create_connector(&neutron_domain).await?;

        if self.id == 0 {
            return Err(ManagerError::IdIsZero);
        }

        let mut config = neutron_connector.get_workflow_config(self.id).await?;

        // Verify the update config
        self.verify_update_config(&config)?;

        // TODO: Generate service config update instructions
        // TODO: Generate authorization update instructions
        
        // Verify the config is working
        // Save config to registry

        Ok(())
    }

    fn verify_update_config(&self, old_config: &WorkflowConfig) -> ManagerResult<()> {
        // Verify the update config
        if self.id == 0 {
            return Err(ManagerError::IdIsZero);
        }

        // TODO: Verify all services Ids exists in the old workflow config
        for (id, service) in &self.services {
            if !old_config.services.contains_key(id) {
                return Err(ManagerError::ServiceIdIsMissing(service.name.clone()));
            }
        }

        Ok(())
    }
}
