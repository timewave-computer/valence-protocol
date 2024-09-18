use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use service_utils::Id;
use valence_authorization_utils::authorization::AuthorizationInfo;

use crate::{
    account::{AccountInfo, AccountType, InstantiateAccountData},
    connectors::Connectors,
    domain::Domain,
    error::{ManagerError, ManagerResult},
    macros::ensure,
    service::ServiceInfo,
    MAIN_CHAIN, MAIN_DOMAIN, NEUTRON_DOMAIN,
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

    pub fn set_processor_addr(&mut self, domain: String, addr: String) {
        self.processor_addrs.insert(domain, addr);
    }

    pub fn set_authorization_bridge_addr(&mut self, domain: String, addr: String) {
        self.authorization_bridge_addrs.insert(domain, addr);
    }

    pub fn set_processor_bridge_addr(&mut self, domain: String, addr: String) {
        self.processor_bridge_addrs.insert(domain, addr);
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
    pub authorizations: BTreeMap<Id, AuthorizationInfo>,
    /// The list account data by id
    pub accounts: BTreeMap<Id, AccountInfo>,
    /// The list service data by id
    pub services: BTreeMap<Id, ServiceInfo>,
    /// This is the info regarding authorization andp rocessor contracts.
    /// Must be empty (Default) when a new workflow is instantiated.
    /// It gets populated when the workflow is instantiated.
    #[serde(default)]
    pub authorization_data: AuthorizationData,
}

impl WorkflowConfig {
    /// Instantiate a workflow on all domains.
    pub async fn init(&mut self, connectors: &Connectors) -> ManagerResult<()> {
        // Verify the whole workflow config
        self.verify_new_config()?;

        // We create the neutron connector specifically because our registry is on neutron.
        let mut neutron_connector = connectors.get_or_create_connector(&NEUTRON_DOMAIN).await?;

        // Get workflow next id from on chain workflow registry
        let workflow_id = neutron_connector.reserve_workflow_id().await?;
        self.id = workflow_id;

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
            .instantiate_authorization(workflow_id, authorization_salt, main_processor_addr.clone())
            .await?;

        main_connector
            .instantiate_processor(
                workflow_id,
                main_processor_salt,
                authorization_addr.clone(),
                None,
            )
            .await?;

        self.authorization_data
            .set_authorization_addr(authorization_addr.clone());
        self.authorization_data
            .set_processor_addr(MAIN_DOMAIN.to_string(), main_processor_addr);

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
                        authorization_bridge_account_addr.clone(),
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
                // Adding external domain to the authorization contract will create the bridge account on that domain
                main_connector
                    .add_external_domain(
                        MAIN_CHAIN,
                        domain.get_chain_name(),
                        authorization_addr.clone(),
                        processor_addr.clone(),
                        processor_bridge_account_addr.clone(),
                    )
                    .await?;

                // Instantiate the authorization bridge account on main connector to external domain
                // in polytone and because its IBC, we basically verify this account was created or retry if it wasn't.
                main_connector
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
                    .set_processor_addr(domain.get_chain_name().to_string(), processor_addr);

                // Add authorization bridge account info by domain
                self.authorization_data.set_authorization_bridge_addr(
                    domain.get_chain_name().to_string(),
                    authorization_bridge_account_addr,
                );

                // Add processor bridge account info by domain
                self.authorization_data.set_processor_bridge_addr(
                    domain.get_chain_name().to_string(),
                    processor_bridge_account_addr,
                );
            };
        }

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

    /// Modify an existing config with a new config
    fn _modify(&mut self) {}

    /// Verify the config is correct and are not missing any data
    fn verify_new_config(&mut self) -> ManagerResult<()> {
        // Verify id is 0, new configs should not have id
        ensure!(self.id == 0, ManagerError::IdNotZero);

        // make sure config authorization data is empty,
        // in new configs, this data should be set to default as it is getting populated
        // by the init function.
        ensure!(
            self.authorization_data == AuthorizationData::default(),
            ManagerError::AuthorizationDataNotDefault
        );

        // TODO: Verify links have all accounts and services include from the list.
        // TODO: Verify all accounts are referenced in service config at least once (or else we have unused account)

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
