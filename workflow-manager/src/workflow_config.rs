use std::collections::{BTreeMap, HashMap};

use anyhow::Result as AnyResult;
use services_utils::Id;
use valence_authorization_utils::authorization::AuthorizationInfo;

use crate::{
    account::{AccountInfo, AccountType, InstantiateAccountData},
    context::Context,
    error::{ManagerError, ManagerResult},
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
    pub async fn init(&mut self, ctx: &mut Context) -> AnyResult<()> {
        // TODO: We probably want to verify the whole workflow config first, before doing any operations
        let mut account_instantiate_datas: HashMap<u64, InstantiateAccountData> = HashMap::new();
        // init accounts
        for (account_id, account) in self.accounts.iter_mut() {
            if let AccountType::Addr { .. } = account.ty {
                // TODO: Probably should error? we are trying to instantiate a new workflow with existing account
                // this is problematic because we don't know who the admin of the account is
                // and how we can update its approved services list.
                // We can also assume the initier knows what he is doing, and will adjust those accounts manually.
                // We can also output what the needed operations to adjust it,
                // similar to what we what we will do on workflow update
                continue;
            }
            let domain_connector = ctx.get_or_create_connector(&account.domain).await?;
            let (addr, salt) = domain_connector
                .predict_address(account_id, &account.ty.to_string(), "account")
                .await?;

            account_instantiate_datas.insert(
                *account_id,
                InstantiateAccountData::new(*account_id, account.clone(), addr.clone(), salt),
            );

            account.ty = AccountType::Addr { addr };
        }

        for (link_id, link) in self.links.clone().iter() {
            let service = self.get_service_mut(link.service_id)?;

            let domain_connector = ctx.get_or_create_connector(&service.domain).await?;
            let (service_addr, salt) = domain_connector
                .predict_address(&link.service_id, &service.config.to_string(), "service")
                .await?;

            let mut patterns =
                Vec::with_capacity(link.input_accounts_id.len() + link.output_accounts_id.len());
            let mut replace_with =
                Vec::with_capacity(link.input_accounts_id.len() + link.output_accounts_id.len());

            // At this stage we should already have all addresses for all account ids
            for account_id in link.input_accounts_id.iter() {
                let account_data = account_instantiate_datas.get_mut(account_id).ok_or(
                    ManagerError::FailedToRetrieveAccountInitData(*account_id, *link_id),
                )?;
                // We add the service address to the approved services list of the input account
                account_data.add_service(service_addr.to_string());

                patterns.push(format!("|account_id|\":{account_id}"));
                replace_with.push(format!("account_addr\":\"{}\"", account_data.addr.clone()))
            }

            for account_id in link.output_accounts_id.iter() {
                let account_data = account_instantiate_datas.get(account_id).ok_or(
                    ManagerError::FailedToRetrieveAccountInitData(*account_id, *link_id),
                )?;

                patterns.push(format!("|account_id|\":{account_id}"));
                replace_with.push(format!("account_addr\":\"{}\"", account_data.addr.clone()))
            }

            service.config.replace_config(patterns, replace_with)?;

            // init the service
            domain_connector
                .instantiate_service(link.service_id, &service.config, salt)
                .await?
        }

        // println!("{:#?}", account_instantiate_datas);

        // init accounts
        for (account_id, account_instantiate_data) in account_instantiate_datas.iter() {
            let account = self.get_account(account_id)?;
            let domain_connector = ctx.get_or_create_connector(&account.domain).await?;
            domain_connector
                .instantiate_account(account_instantiate_data)
                .await?;
        }

        // TODO: init authorizations

        Ok(())
    }
}

impl WorkflowConfig {
    fn get_account(&self, account_id: &u64) -> ManagerResult<&AccountInfo> {
        self.accounts
            .get(account_id)
            .ok_or(ManagerError::generic_err(format!(
                "Account with id {} not found",
                account_id
            )))
    }

    fn get_service_mut(&mut self, service_id: u64) -> ManagerResult<&mut ServiceInfo> {
        self.services
            .get_mut(&service_id)
            .ok_or(ManagerError::generic_err(format!(
                "Service with id {} not found",
                service_id
            )))
    }
}
