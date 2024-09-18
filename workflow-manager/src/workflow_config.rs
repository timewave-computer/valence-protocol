use std::collections::{BTreeMap, HashMap, HashSet};

use services_utils::Id;
use valence_authorization_utils::authorization::AuthorizationInfo;

use crate::{
    account::{AccountInfo, AccountType, InstantiateAccountData},
    connectors::Connectors,
    domain::Domain,
    error::{ManagerError, ManagerResult},
    service::ServiceInfo,
    MAIN_CHAIN, MAIN_DOMAIN, NEUTRON_DOMAIN,
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
    pub owner: String,
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
    pub async fn init(&mut self, connectors: &Connectors) -> ManagerResult<()> {
        // TODO: We probably want to verify the whole workflow config first, before doing any operations

        // We create the neutron connector specifically because our registry is on neutron.
        let mut neutron_connector = connectors.get_or_create_connector(&NEUTRON_DOMAIN).await?;

        // Get workflow next id from on chain workflow registry
        // TODO: get registry address
        let workflow_id = neutron_connector.reserve_workflow_id().await?;

        // Instantiate the authorization module contracts.
        let all_domains = self.get_all_domains();

        // Instantiate our autorization and processor contracts on the main domain
        let mut main_connector = connectors.get_or_create_connector(&MAIN_DOMAIN).await?;
        let (authorization_addr, authorization_salt) = main_connector
            .get_address(workflow_id, "authorization", "authorization")
            .await?;
        let (main_processor_addr, main_processor_salt) = main_connector
            .get_address(workflow_id, "processor", "processor")
            .await?;

        main_connector
            .instantiate_authorization(workflow_id, authorization_salt, main_processor_addr)
            .await?;

        main_connector
            .instantiate_processor(
                workflow_id,
                main_processor_salt,
                authorization_addr.clone(),
                None,
            )
            .await?;

        // init processors and bridge accounts on all other domains
        // For mainnet we need to instantiate a bridge account for each processor instantiated on other domains
        // For other domains, we need to instantiate a bridge account on the main domain for the authorization contract
        for domain in all_domains.iter() {
            if domain != &MAIN_DOMAIN {
                let mut connector = connectors.get_or_create_connector(domain).await?;

                // get the authorization bridge account address on the other domain (to be the admon of the processor)
                let authorization_bridge_account_addr = connector
                    .get_address_bridge(
                        authorization_addr.as_str(),
                        MAIN_CHAIN,
                        MAIN_CHAIN,
                        domain.get_chain_name(),
                    )
                    .await?;

                // Get the processor address on the other domain
                let (processor_addr, salt) = connector
                    .get_address(workflow_id, "processor", "processor")
                    .await?;

                // Instantiate the processor on the other domain, the admin is the bridge account address of the authorization contract
                connector
                    .instantiate_processor(
                        workflow_id,
                        salt,
                        authorization_bridge_account_addr,
                        None,
                    )
                    .await?;

                // Get the processor bridge account address on main domain
                let processor_bridge_account_addr = main_connector
                    .get_address_bridge(
                        processor_addr.as_str(),
                        MAIN_CHAIN,
                        domain.get_chain_name(),
                        MAIN_CHAIN,
                    )
                    .await?;

                // construct and add the `ExternalDomain` info to the authorization contract
                main_connector
                    .add_external_domain(
                        MAIN_CHAIN,
                        domain.get_chain_name(),
                        authorization_addr.clone(),
                        processor_addr.clone(),
                        processor_bridge_account_addr,
                    )
                    .await?;

                // TODO: Add instantiate authorization bridge account step, to very the bridge was instantiated correctly.
                // Maybe this step should be done after this loop to give at least some time for the contract to create the account
                // because this is async and requires an IBC msg.

                // Adding external domain to the authorization contract will create the bridge account on that domain
                // But we still need to create the processor bridge account on main domain.
                // The processor will create the bridge account on instantiation, but we still need to verify the account was created
                // and if it wasn't, we want to retry couple of times before erroring out.
                connector
                    .instantiate_processor_bridge_account(processor_addr, 5)
                    .await?;
            };
        }

        // Predict account addresses and get the instantiate datas for each account
        let mut account_instantiate_datas: HashMap<u64, InstantiateAccountData> = HashMap::new();

        for (account_id, account) in self.accounts.iter_mut() {
            if let AccountType::Addr { .. } = account.ty {
                // TODO: Probably should error? we are trying to instantiate a new workflow with existing account
                // this is problematic because we don't know who the admin of the account is
                // and how we can update its approved services list.
                // We can also assume the initier knows what he is doing, and will adjust those accounts manually.
                // We can also output what the needed operations to adjust it,
                // similar to what we will do on workflow update
                continue;
            }

            let mut domain_connector = connectors.get_or_create_connector(&account.domain).await?;
            let (addr, salt) = domain_connector
                .get_address(
                    workflow_id,
                    &account.ty.to_string(),
                    format!("account_{}", account_id).as_str(),
                )
                .await?;

            account_instantiate_datas.insert(
                *account_id,
                InstantiateAccountData::new(*account_id, account.clone(), addr.clone(), salt),
            );

            account.ty = AccountType::Addr { addr };
        }

        // We first predict the service addresses
        // Then we update the service configs with the account predicted addresses
        // for all input accounts we add the service address to the approved services list
        // and then instantiate the services
        for (link_id, link) in self.links.clone().iter() {
            let service = self.get_service_mut(link.service_id)?;

            let mut domain_connector = connectors.get_or_create_connector(&service.domain).await?;
            let (service_addr, salt) = domain_connector
                .get_address(
                    workflow_id,
                    &service.config.to_string(),
                    format!("service_{}", link_id).as_str(),
                )
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

        // Instantiate accounts after we added all services addresses to the approved services list for each account
        for (account_id, account_instantiate_data) in account_instantiate_datas.iter() {
            let account = self.get_account(account_id)?;
            let mut domain_connector = connectors.get_or_create_connector(&account.domain).await?;
            domain_connector
                .instantiate_account(account_instantiate_data)
                .await?;
        }

        // Change the admin of the authorization contract to the owner of the workflow
        main_connector
            .change_authorization_owner(authorization_addr, self.owner.clone())
            .await?;

        // TODO: Verify the workflow is complete and everything is instantiatied correctly
        Ok(())
    }
}

impl WorkflowConfig {
    /// Get a unique list of all domains, so it will be easiter to create proccessors
    fn get_all_domains(&self) -> HashSet<Domain> {
        let mut domains = self
            .accounts
            .values()
            .map(|account| account.domain.clone())
            .collect::<Vec<_>>();
        domains.extend(self.services.values().map(|service| service.domain.clone()));
        HashSet::from_iter(domains)
    }

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
