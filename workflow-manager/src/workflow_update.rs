use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use valence_authorization_utils::authorization::AuthorizationInfo;
use valence_service_utils::Id;

use crate::{account::AccountInfo, service::ServiceInfo, workflow_config::{AuthorizationData, Link}};

/// The job of the update, is to output a set of instructions to the user to update the workflow configuration.  
/// We need a separate struct because this is an update and we need to have different fields for the update.
///
/// Here are the main differences:
/// - The id is required for the update
/// - The owner is optional in case we want to change it.
/// -? Accounts must not be removed, accounts that were instantiated part of the workflow might contain funds, 
///   Removing them from here might cause us to "forget" their addresses and we won't be able to recover those funds.
///   To remove account, you can set active to false.
///   Need an update type to allow updaing the admin of the account
/// -? Services must not be removed, to reduce needed calls, we can just set the service to be inactive, and still keep it approved on an account. 
///   Revmoing the authorization to call it, should be enough to "remove" it.
///   Need an update type to allow to change configuration of the service
/// - For authorizations we would only need the "delta" or changes, we can add or remove an authorization.
///   We can Add / Create, modify, disable and enable.
///   Need to create an update type of enum with those actions.

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkflowConfigUpdate {
    /// This is the id of the workflow
    /// Required for update
    pub id: u64,
    /// New owner, if the owner is to be updated
    pub owner: Option<String>,
    /// A list of links between an accounts and services
    pub links: BTreeMap<Id, Link>,
    /// The list account data by id
    pub accounts: BTreeMap<Id, AccountInfo>,
    /// The list service data by id
    pub services: BTreeMap<Id, ServiceInfo>,
    /// A list of authorizations
    pub authorizations: Vec<AuthorizationInfo>,
}

impl WorkflowConfigUpdate {
    /// Modify an existing config with a new config
    fn _update(&mut self) {}
}