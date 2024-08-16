pub mod account;
pub mod domain;
pub mod service;

use std::collections::BTreeMap;

use account::AccountInfo;
use service::ServiceInfo;
use services_utils::Id;

pub struct WorkflowConfig {
    /// A list of links between an accounts and services
    pub links: BTreeMap<Id, Link>,
    /// A list of authorizations
    // pub authorizations: BTreeMap<Id, Authorization>,
    /// The list account data by id
    pub accounts: BTreeMap<Id, AccountInfo>,
    // /// The list service data by id
    pub services: BTreeMap<Id, ServiceInfo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    /// List of input accounts
    pub input_accounts_id: Vec<Id>,
    /// List of output accounts
    pub output_accounts_id: Vec<Id>,
    /// A service config
    pub service_id: Id,
}
