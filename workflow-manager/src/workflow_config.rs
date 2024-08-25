use std::collections::{BTreeMap, HashMap};

use services_utils::Id;
use valence_authorization_utils::authorization::AuthorizationInfo;

use crate::{
    account::{AccountInfo, AccountType, InstantiateAccountData},
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
        let mut account_instantiate_datas: HashMap<u64, InstantiateAccountData> = HashMap::new();
        // init accounts
        for (account_id, account) in self.accounts.iter_mut() {
            if let AccountType::Addr { .. } = account.ty {
                // TODO: Probably should error? we are trying to instantiate a new workflow with existing account
                // this is problematic because we don't know who the admin of the account is
                // and how we can update its approved services list.
                continue;
            }
            let domain_connector = ctx.get_or_create_connector(&account.domain).await;
            let (addr, salt) = domain_connector
                .predict_address(account_id, &account.ty.to_string(), "account")
                .await;

            account_instantiate_datas.insert(
                *account_id,
                InstantiateAccountData::new(*account_id, account.clone(), addr.clone(), salt),
            );
            println!("Account id : {:#?}", addr);
            account.ty = AccountType::Addr { addr };
        }

        for (service_id, link) in self.links.iter() {
            let service = self.services.get_mut(&link.service_id).unwrap();

            let domain_connector = ctx.get_or_create_connector(&service.domain).await;
            let (service_addr, salt) = domain_connector
                .predict_address(service_id, &service.config.to_string(), "service")
                .await;

            let mut patterns =
                Vec::with_capacity(link.input_accounts_id.len() + link.output_accounts_id.len());
            let mut replace_with =
                Vec::with_capacity(link.input_accounts_id.len() + link.output_accounts_id.len());

            // At this stage we should already have all addresses for all account ids, including bridges
            link.input_accounts_id.iter().for_each(|id| {
                let account_data = account_instantiate_datas.get_mut(id).unwrap();
                let account_addr = account_data.addr.clone();
                account_data.add_service(service_addr.to_string());

                patterns.push(format!("|account_id|\":{id}"));
                replace_with.push(format!("account_addr\":\"{account_addr}\""))
            });

            link.output_accounts_id.iter().for_each(|id| {
                let account_data = account_instantiate_datas.get(id).unwrap();
                let account_addr = account_data.addr.clone();

                patterns.push(format!("|account_id|\":{id}"));
                replace_with.push(format!("account_addr\":\"{account_addr}\""))
            });

            service.config.replace_config(patterns, replace_with);

            // TODO: init the service
        }
        
        // TODO: init accounts
        for (account_id, account_instantiate_data) in account_instantiate_datas.iter() {
            let account = self.accounts.get(account_id).unwrap();
            let domain_connector = ctx.get_or_create_connector(&account.domain).await;
            domain_connector.init_account(account_instantiate_data).await;
        }

        println!("{:#?}", account_instantiate_datas);

        // TODO: init authorizations
    }
}
