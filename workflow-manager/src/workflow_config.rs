use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use valence_authorization_utils::authorization::AuthorizationInfo;

use valence_service_utils::Id;

use crate::{
    account::{AccountInfo, AccountType, InstantiateAccountData},
    connectors::Connectors,
    domain::Domain,
    error::{ManagerError, ManagerResult},
    macros::ensure,
    service::ServiceInfo,
    NEUTRON_CHAIN,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Link {
    /// List of input accounts by id
    pub input_accounts_id: Vec<Id>,
    /// List of output accounts by id
    pub output_accounts_id: Vec<Id>,
    /// The service id
    pub service_id: Id,
}

/// This struct holds all the data regarding our authorization and processor
/// contracts and bridge accounts
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct AuthorizationData {
    /// authorization contract address on neutron
    pub authorization_addr: String,
    /// List of processor addresses by domain
    /// Key: domain name | Value: processor address
    pub processor_addrs: BTreeMap<String, String>,
    /// List of authorization bridge addresses by domain
    /// The addresses are on the specified domain
    /// Key: domain name | Value: authorization bridge address on that domain
    pub authorization_bridge_addrs: BTreeMap<String, String>,
    /// List of processor bridge addresses by domain
    /// All addresses are on nuetron, mapping to what domain this bridge account is for
    /// Key: domain name | Value: processor bridge address on that domain
    pub processor_bridge_addrs: BTreeMap<String, String>,
}

impl AuthorizationData {
    pub fn set_authorization_addr(&mut self, addr: String) {
        self.authorization_addr = addr;
    }

    pub fn set_processor_addr(&mut self, domain: Domain, addr: String) {
        self.processor_addrs.insert(domain.to_string(), addr);
    }

    pub fn set_authorization_bridge_addr(&mut self, domain: Domain, addr: String) {
        self.authorization_bridge_addrs
            .insert(domain.to_string(), addr);
    }

    pub fn set_processor_bridge_addr(&mut self, domain: Domain, addr: String) {
        self.processor_bridge_addrs.insert(domain.to_string(), addr);
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct WorkflowConfig {
    // This is the id of the workflow
    #[serde(default)]
    pub id: u64,
    pub owner: String,
    /// A list of links between an accounts and services
    pub links: BTreeMap<Id, Link>,
    /// A list of authorizations
    pub authorizations: Vec<AuthorizationInfo>,
    /// The list account data by id
    pub accounts: BTreeMap<Id, AccountInfo>,
    /// The list service data by id
    pub services: BTreeMap<Id, ServiceInfo>,
    /// This is the info regarding authorization and processor contracts.
    /// Must be empty (Default) when a new workflow is instantiated.
    /// It gets populated when the workflow is instantiated.
    #[serde(default)]
    pub authorization_data: AuthorizationData,
}

impl WorkflowConfig {
    /// Instantiate a workflow on all domains.
    pub async fn init(&mut self, connectors: &Connectors) -> ManagerResult<()> {
        let neutron_domain = Domain::CosmosCosmwasm(NEUTRON_CHAIN.to_string());
        // Verify the whole workflow config
        self.verify_new_config()?;

        // We create the neutron connector specifically because our registry is on neutron.
        let mut neutron_connector = connectors.get_or_create_connector(&neutron_domain).await?;

        // Get workflow next id from on chain workflow registry
        let workflow_id = neutron_connector.reserve_workflow_id().await?;
        self.id = workflow_id;

        // Instantiate the authorization module contracts.
        let all_domains = self.get_all_domains();

        let (authorization_addr, authorization_salt) = neutron_connector
            .get_address(self.id, "valence_authorization", "valence_authorization")
            .await?;
        let (main_processor_addr, main_processor_salt) = neutron_connector
            .get_address(self.id, "valence_processor", "valence_processor")
            .await?;

        neutron_connector
            .instantiate_authorization(self.id, authorization_salt, main_processor_addr.clone())
            .await?;

        neutron_connector
            .instantiate_processor(
                self.id,
                main_processor_salt,
                authorization_addr.clone(),
                None,
            )
            .await?;

        self.authorization_data
            .set_authorization_addr(authorization_addr.clone());
        self.authorization_data
            .set_processor_addr(neutron_domain.clone(), main_processor_addr);

        // init processors and bridge accounts on all other domains
        // For mainnet we need to instantiate a bridge account for each processor instantiated on other domains
        // For other domains, we need to instantiate a bridge account on the main domain for the authorization contract
        for domain in all_domains.iter() {
            if domain != &neutron_domain {
                let mut connector = connectors.get_or_create_connector(domain).await?;

                // get the authorization bridge account address on the other domain (to be the admon of the processor)
                let authorization_bridge_account_addr = connector
                    .get_address_bridge(
                        authorization_addr.as_str(),
                        NEUTRON_CHAIN,
                        domain.get_chain_name(),
                        domain.get_chain_name(),
                    )
                    .await?;

                // Get the processor address on the other domain
                let (processor_addr, salt) = connector
                    .get_address(self.id, "valence_processor", "valence_processor")
                    .await?;

                // Instantiate the processor on the other domain, the admin is the bridge account address of the authorization contract
                connector
                    .instantiate_processor(
                        self.id,
                        salt,
                        authorization_bridge_account_addr.clone(),
                        None,
                    )
                    .await?;

                // Get the processor bridge account address on main domain
                let processor_bridge_account_addr = neutron_connector
                    .get_address_bridge(
                        processor_addr.as_str(),
                        NEUTRON_CHAIN,
                        domain.get_chain_name(),
                        NEUTRON_CHAIN,
                    )
                    .await?;

                // construct and add the `ExternalDomain` info to the authorization contract
                // Adding external domain to the authorization contract will create the bridge account on that domain
                neutron_connector
                    .add_external_domain(
                        neutron_domain.get_chain_name(),
                        domain.get_chain_name(),
                        authorization_addr.clone(),
                        processor_addr.clone(),
                        processor_bridge_account_addr.clone(),
                    )
                    .await?;

                // Instantiate the authorization bridge account on main connector to external domain
                // in polytone and because its IBC, we basically verify this account was created or retry if it wasn't.
                neutron_connector
                    .instantiate_authorization_bridge_account(
                        authorization_addr.clone(),
                        domain.get_chain_name().to_string(),
                        3,
                    )
                    .await?;

                // The processor will create the bridge account on instantiation, but we still need to verify the account was created
                // and if it wasn't, we want to retry a couple of times before erroring out.
                connector
                    .instantiate_processor_bridge_account(processor_addr.clone(), 3)
                    .await?;

                // Add processor address to list of processor by domain
                self.authorization_data
                    .set_processor_addr(domain.clone(), processor_addr);

                // Add authorization bridge account info by domain
                self.authorization_data.set_authorization_bridge_addr(
                    domain.clone(),
                    authorization_bridge_account_addr,
                );

                // Add processor bridge account info by domain
                self.authorization_data
                    .set_processor_bridge_addr(domain.clone(), processor_bridge_account_addr);
            };
        }

        // We need to manually drop neutron connector here because we finished with it for now.
        drop(neutron_connector);

        // TODO: Discuss if we want to separate the bridge account instantiation from contract creation.
        // The main benefit of this is that it will give some time for the async operation to complete
        // but if the creation fails, it means we continued the workflow instantiatoin for no reason.

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
                    self.id,
                    &account.ty.to_string(),
                    format!("account_{}", account_id).as_str(),
                )
                .await?;

            account_instantiate_datas.insert(
                *account_id,
                InstantiateAccountData::new(*account_id, account.clone(), addr.clone(), salt),
            );

            account.ty = AccountType::Addr { addr: addr.clone() };
            account.addr = Some(addr);
        }

        // We first predict the service addresses
        // Then we update the service configs with the account predicted addresses
        // for all input accounts we add the service address to the approved services list
        // and then instantiate the services
        for (link_id, link) in self.links.clone().iter() {
            let mut service = self.get_service(link.service_id)?;

            let mut domain_connector = connectors.get_or_create_connector(&service.domain).await?;
            let (service_addr, salt) = domain_connector
                .get_address(
                    self.id,
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
                replace_with.push(format!(
                    "service_account_addr\":\"{}\"",
                    account_data.addr.clone()
                ))
            }

            for account_id in link.output_accounts_id.iter() {
                let account_data = account_instantiate_datas.get(account_id).ok_or(
                    ManagerError::FailedToRetrieveAccountInitData(*account_id, *link_id),
                )?;

                patterns.push(format!("|account_id|\":{account_id}"));
                replace_with.push(format!(
                    "service_account_addr\":\"{}\"",
                    account_data.addr.clone()
                ))
            }

            service.config.replace_config(patterns, replace_with)?;
            service.addr = Some(service_addr);

            self.save_service(link.service_id, &service);

            // Get processor address for this domain
            let processor_addr = self.get_processor_account_on_domain(service.domain.clone())?;

            // init the service
            domain_connector
                .instantiate_service(
                    self.id,
                    authorization_addr.clone(),
                    processor_addr,
                    link.service_id,
                    service.config,
                    salt,
                )
                .await?
        }

        // Instantiate accounts after we added all services addresses to the approved services list for each account
        for (account_id, account_instantiate_data) in account_instantiate_datas.iter() {
            let account = self.get_account(account_id)?;
            let mut domain_connector = connectors.get_or_create_connector(&account.domain).await?;
            let processor_addr = self
                .authorization_data
                .processor_addrs
                .get(&account.domain.to_string())
                .ok_or(ManagerError::ProcessorAddrNotFound(account.domain.clone()))?;

            domain_connector
                .instantiate_account(self.id, processor_addr.clone(), account_instantiate_data)
                .await?;
        }

        // Loop over authorizations, and change ids to their addresses
        for authorization in self.authorizations.iter_mut() {
            match &mut authorization.actions_config {
                valence_authorization_utils::authorization::ActionsConfig::Atomic(
                    atomic_actions_config,
                ) => {
                    atomic_actions_config.actions.iter_mut().for_each(|action| {
                        let addr = match &action.contract_address {
                            valence_service_utils::ServiceAccountType::Addr(a) => a.to_string(),
                            valence_service_utils::ServiceAccountType::AccountId(account_id) => {
                                account_instantiate_datas
                                    .get(account_id)
                                    .unwrap()
                                    .addr
                                    .clone()
                            }
                            valence_service_utils::ServiceAccountType::ServiceId(service_id) => {
                                self.services.get(service_id).unwrap().addr.clone().unwrap()
                            }
                        };
                        action.contract_address =
                            valence_service_utils::ServiceAccountType::Addr(addr);
                    });
                }
                valence_authorization_utils::authorization::ActionsConfig::NonAtomic(
                    non_atomic_actions_config,
                ) => {
                    non_atomic_actions_config
                        .actions
                        .iter_mut()
                        .for_each(|action| {
                            let addr = match &action.contract_address {
                                valence_service_utils::ServiceAccountType::Addr(a) => a.to_string(),
                                valence_service_utils::ServiceAccountType::AccountId(
                                    account_id,
                                ) => account_instantiate_datas
                                    .get(account_id)
                                    .unwrap()
                                    .addr
                                    .clone(),
                                valence_service_utils::ServiceAccountType::ServiceId(
                                    service_id,
                                ) => self.services.get(service_id).unwrap().addr.clone().unwrap(),
                            };
                            action.contract_address =
                                valence_service_utils::ServiceAccountType::Addr(addr);
                        });
                }
            }
        }

        // Verify the workflow was instantiated successfully
        self.verify_init_was_successful(connectors, account_instantiate_datas)
            .await?;

        // Get neutron connector again because we need it to change admin of the authorization contract
        let mut neutron_connector = connectors.get_or_create_connector(&neutron_domain).await?;

        neutron_connector
            .add_authorizations(authorization_addr.clone(), self.authorizations.clone())
            .await?;

        // Change the admin of the authorization contract to the owner of the workflow
        neutron_connector
            .change_authorization_owner(authorization_addr.clone(), self.owner.clone())
            .await?;

        // Save the workflow config to registry
        neutron_connector.save_workflow_config(self.clone()).await?;

        Ok(())
    }

    /// Modify an existing config with a new config
    fn _modify(&mut self) {}

    /// Verify the config is correct and are not missing any data
    pub fn verify_new_config(&mut self) -> ManagerResult<()> {
        // Verify id is 0, new configs should not have an id
        ensure!(self.id == 0, ManagerError::IdNotZero);

        // Verify owner is not empty
        ensure!(!self.owner.is_empty(), ManagerError::OwnerEmpty);

        // Make sure config authorization data is empty,
        // in new configs, this data should be set to default as it is getting populated
        // by the init function.
        ensure!(
            self.authorization_data == AuthorizationData::default(),
            ManagerError::AuthorizationDataNotDefault
        );

        // Verify authorizations is not empty
        ensure!(
            !self.authorizations.is_empty(),
            ManagerError::NoAuthorizations
        );

        // Get all services and accounts ids that exists in links
        let mut services: BTreeSet<Id> = BTreeSet::new();
        let mut accounts: BTreeSet<Id> = BTreeSet::new();

        for (_, link) in self.links.iter() {
            for account_id in link.input_accounts_id.iter() {
                accounts.insert(*account_id);
            }

            for account_id in link.output_accounts_id.iter() {
                accounts.insert(*account_id);
            }

            services.insert(link.service_id);
        }

        // Verify all accounts are referenced in links at least once
        for account_id in self.accounts.keys() {
            if !accounts.remove(account_id) {
                return Err(ManagerError::AccountIdNotFoundInLinks(*account_id));
            }
        }

        // Verify accounts is empty, if its not, it means we have a link with an account id that doesn't exists
        ensure!(
            accounts.is_empty(),
            ManagerError::AccountIdNotFoundLink(accounts)
        );

        // Verify all services are referenced in links at least once
        for service_id in self.services.keys() {
            if !services.remove(service_id) {
                return Err(ManagerError::ServiceIdNotFoundInLinks(*service_id));
            }
        }

        // Verify services is empty, if its not, it means we have a link with a service id that doesn't exists
        ensure!(
            services.is_empty(),
            ManagerError::ServiceIdNotFoundLink(services)
        );

        // Verify all accounts are referenced in service config at least once (or else we have unused account)
        // accounts should be empty here
        for service in self.services.values() {
            accounts.extend(service.config.get_account_ids()?);
        }

        // We remove each account if we found
        // if account id was not removed, it means we didn't find it in any service config
        for account_id in self.accounts.keys() {
            if !accounts.remove(account_id) {
                return Err(ManagerError::AccountIdNotFoundInServices(*account_id));
            }
        }

        ensure!(
            accounts.is_empty(),
            ManagerError::AccountIdNotFoundServiceConfig(accounts)
        );

        // Run the soft_validate method on each service config
        for _service in self.services.values() {
            // TODO: mock api for the connector
            // service.config.soft_validate_config()?;
        }

        Ok(())
    }

    /// Verify our workflow was instantiated successfully
    async fn verify_init_was_successful(
        &mut self,
        connectors: &Connectors,
        account_instantiate_datas: HashMap<u64, InstantiateAccountData>,
    ) -> ManagerResult<()> {
        let neutron_domain = Domain::CosmosCosmwasm(NEUTRON_CHAIN.to_string());
        let mut neutron_connector = connectors.get_or_create_connector(&neutron_domain).await?;
        // verify id is not taken (have no config in registry)
        ensure!(
            neutron_connector
                .query_workflow_registry(self.id)
                .await
                .is_err(),
            ManagerError::WorkflowIdAlreadyExists(self.id)
        );

        // Verify authorization contract is correct on neutron chain
        neutron_connector
            .verify_authorization_addr(self.authorization_data.authorization_addr.clone())
            .await?;

        // Drop the neutron connector because we no longer use it.
        drop(neutron_connector);

        // verify all accounts have addresses and they return the correct code id
        for (_, account_data) in account_instantiate_datas {
            let mut connector = connectors
                .get_or_create_connector(&account_data.info.domain)
                .await?;

            connector.verify_account(account_data.addr.clone()).await?;
        }

        // verify services have an address and query on-chain contract to make sure its correct
        for (_, service) in self.services.iter() {
            let mut connector = connectors.get_or_create_connector(&service.domain).await?;

            connector.verify_service(service.addr.clone()).await?;
        }

        // Veryify each processor was instantiated correctly
        for (domain, processor_addr) in self.authorization_data.processor_addrs.clone().iter() {
            let mut connector = connectors
                .get_or_create_connector(&Domain::from_string(domain.to_string())?)
                .await?;

            connector.verify_processor(processor_addr.clone()).await?;
        }

        // Verify authorization and processor bridge accounts were created correctly
        for (domain, authorization_bridge_addr) in
            self.authorization_data.authorization_bridge_addrs.iter()
        {
            let mut connector = connectors
                .get_or_create_connector(&Domain::from_string(domain.to_string())?)
                .await?;

            connector
                .verify_bridge_account(authorization_bridge_addr.clone())
                .await?;
        }

        Ok(())
    }
}

impl WorkflowConfig {
    fn get_processor_account_on_domain(&mut self, domain: Domain) -> ManagerResult<String> {
        // Find either a processor bridge account or
        let processor_addr = self
            .authorization_data
            .processor_addrs
            .get_key_value(&domain.to_string())
            .ok_or(ManagerError::ProcessorAddrNotFound(domain.clone()))?
            .1;

        Ok(processor_addr.clone())
    }

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

    fn get_service(&mut self, service_id: u64) -> ManagerResult<ServiceInfo> {
        self.services
            .get(&service_id)
            .ok_or(ManagerError::generic_err(format!(
                "Service with id {} not found",
                service_id
            )))
            .cloned()
    }

    fn save_service(&mut self, service_id: u64, service: &ServiceInfo) {
        self.services.insert(service_id, service.clone());
    }
}
