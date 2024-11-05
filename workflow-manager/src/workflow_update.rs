use std::collections::BTreeMap;

use anyhow::Context;
use cosmwasm_std::{to_json_binary, CosmosMsg, WasmMsg};
use cw_ownable::Expiration;
use serde::{Deserialize, Serialize};
use valence_authorization_utils::authorization::{AuthorizationInfo, Priority};
use valence_service_utils::Id;

use crate::{
    connectors::Connectors,
    domain::Domain,
    error::{ManagerError, ManagerResult},
    service::ServiceConfigUpdate,
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
    pub services: BTreeMap<Id, ServiceConfigUpdate>,
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
    pub instructions: Vec<CosmosMsg>,
}

impl WorkflowConfigUpdate {
    /// Modify an existing config with a new config
    pub async fn update(&mut self, connectors: &Connectors) -> ManagerResult<UpdateResponse> {
        let neutron_domain = Domain::CosmosCosmwasm(NEUTRON_CHAIN.to_string());

        // get the old workflow config from registry
        let mut neutron_connector = connectors.get_or_create_connector(&neutron_domain).await?;

        if self.id == 0 {
            return Err(ManagerError::IdIsZero);
        }

        let mut config = neutron_connector.get_workflow_config(self.id).await?;

        let mut instructions: Vec<CosmosMsg> = vec![];
        let mut new_authorizations: Vec<AuthorizationInfo> = vec![];

        for (id, _service_update) in self.services.iter() {
            // Verify that the service id exists in the config and get it
            let _service = config
                .services
                .get(id)
                .context(ManagerError::ServiceIdIsMissing(*id).to_string())?;

            // TODO: Generate service config update instructions
        }

        // Generate authorization update instructions
        for authorization in self.authorizations.clone().into_iter() {
            match authorization {
                AuthorizationInfoUpdate::Add(authorization_info) => {
                    verify_authorization_not_exists(
                        &config.authorizations,
                        authorization_info.label.clone(),
                    )?;

                    // Create instruction for adding authorization
                    new_authorizations.push(authorization_info.clone());

                    // Add new authorizations to our config saved in registry
                    config.authorizations.push(authorization_info);
                }
                AuthorizationInfoUpdate::Modify {
                    label,
                    not_before,
                    expiration,
                    max_concurrent_executions,
                    priority,
                } => {
                    verify_authorization_exists(&config.authorizations, label.clone())?;

                    // Create instruction for modifying authorization
                    instructions.push(WasmMsg::Execute {
                        contract_addr: config.authorization_data.authorization_addr.clone(),
                        msg: to_json_binary(&valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                            valence_authorization_utils::msg::PermissionedMsg::ModifyAuthorization { label: label.clone(), not_before, expiration, max_concurrent_executions, priority: priority.clone() }
                        )).context("Failed binary parsing AuthorizationInfoUpdate::Modify")?,
                        funds: vec![]
                    }.into());

                    // Modify saved config with the new modified authorizations
                    let auth = config
                        .authorizations
                        .iter_mut()
                        .find(|a| a.label == label)
                        .context(format!("Failed to find authorization {}", label))?;

                    if let Some(not_before) = not_before {
                        auth.not_before = not_before;
                    }

                    auth.priority = priority;
                    auth.max_concurrent_executions = max_concurrent_executions;
                }
                AuthorizationInfoUpdate::Disable(label) => {
                    verify_authorization_exists(&config.authorizations, label.clone())?;

                    // Create instruction for disabling authorization
                    instructions.push(WasmMsg::Execute {
                        contract_addr: config.authorization_data.authorization_addr.clone(),
                        msg: to_json_binary(&valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                            valence_authorization_utils::msg::PermissionedMsg::DisableAuthorization { label }
                        )).unwrap(),
                        funds: vec![]
                    }.into());
                }
                AuthorizationInfoUpdate::Enable(label) => {
                    verify_authorization_exists(&config.authorizations, label.clone())?;

                    // Create instruction for enabling authorization
                    instructions.push(WasmMsg::Execute {
                        contract_addr: config.authorization_data.authorization_addr.clone(),
                        msg: to_json_binary(&valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                            valence_authorization_utils::msg::PermissionedMsg::EnableAuthorization { label }
                        )).unwrap(),
                        funds: vec![]
                    }.into());
                }
            }
        }

        // Add all new authorizations
        instructions.push(
            WasmMsg::Execute {
                contract_addr: config.authorization_data.authorization_addr.clone(),
                msg: to_json_binary(
                    &valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                        valence_authorization_utils::msg::PermissionedMsg::CreateAuthorizations {
                            authorizations: new_authorizations,
                        },
                    ),
                )
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // Save the new config to the registry
        neutron_connector.save_workflow_config(config).await?;

        Ok(UpdateResponse { instructions })
    }
}

fn verify_authorization_not_exists(
    authorizations: &[AuthorizationInfo],
    label: String,
) -> ManagerResult<()> {
    if !authorizations.iter().any(|auth| auth.label == label) {
        return Err(ManagerError::AuthorizationLabelNotFound(label));
    }

    Ok(())
}

fn verify_authorization_exists(
    authorizations: &[AuthorizationInfo],
    label: String,
) -> ManagerResult<()> {
    if authorizations.iter().any(|auth| auth.label == label) {
        return Err(ManagerError::AuthorizationLabelExists(label));
    }

    Ok(())
}
