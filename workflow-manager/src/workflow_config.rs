use std::collections::BTreeMap;

use services_utils::Id;
use valence_authorization_utils::authorization::AuthorizationInfo;

use crate::{
    account::{AccountInfo, AccountType},
    context::Context,
    service::ServiceInfo,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    /// List of input accounts by id
    pub input_accounts_id: Vec<Id>,
    /// List of output accounts by id
    pub output_accounts_id: Vec<Id>,
    /// The service id
    pub service_id: Id,
}

#[derive(Clone, Debug, Default)]
pub struct WorkflowConfig {
    /// A list of links between an accounts and services
    pub links: BTreeMap<Id, Link>,
    /// A list of authorizations
    pub authorizations: BTreeMap<Id, AuthorizationInfo>,
    /// The list account data by id
    pub accounts: BTreeMap<Id, AccountInfo>,
    // /// The list service data by id
    pub services: BTreeMap<Id, ServiceInfo>,
}

impl WorkflowConfig {
    /// Instantiate a workflow on all domains.
    pub async fn init(&mut self, ctx: &mut Context) {
        // init accounts
        for (account_id, account) in self.accounts.iter_mut() {
            let domain_info = ctx.get_or_create_domain_info(&account.domain).await;
            let addr = domain_info
                .connector
                .get_account_addr(*account_id, &account.ty)
                .await;
            account.ty = AccountType::Addr { addr }
        }

        return;

        self.links.iter().for_each(|(_, link)| {
            let mut patterns =
                Vec::with_capacity(link.input_accounts_id.len() + link.output_accounts_id.len());
            let mut replace_with =
                Vec::with_capacity(link.input_accounts_id.len() + link.output_accounts_id.len());

            // At this stage we should already have all addresses for all account ids, including bridges
            link.input_accounts_id.iter().for_each(|id| {
                let account = self.accounts.get(id).unwrap();
                let addr = match &account.ty {
                    AccountType::Addr { addr } => addr.to_string(),
                    _ => panic!("Account must be of type Addr"),
                };

                patterns.push(format!("|account_id|\":{id}"));
                replace_with.push(format!("account_addr\":\"{addr}\""))
            });

            link.output_accounts_id.iter().for_each(|id| {
                let account = self.accounts.get(id).unwrap();
                let addr = match &account.ty {
                    AccountType::Addr { addr } => addr.to_string(),
                    _ => panic!("Account must be of type Addr"),
                };
                patterns.push(format!("|account_id|\":{id}"));
                replace_with.push(format!("account_addr\":\"{addr}\""))
            });

            let service = self.services.get_mut(&link.service_id).unwrap();
            service.config.replace_config(patterns, replace_with);
        });

        // init services
        self.services.iter().for_each(|(_id, _service)| {
            // TODO: init the service
        })

        // TODO: init authorizations
    }
}
