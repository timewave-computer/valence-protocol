use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use valence_authorization_utils::authorization::AuthorizationInfo;

use valence_library_utils::{GetId, Id};

use crate::{
    account::{AccountInfo, AccountType, InstantiateAccountData},
    connectors::Connectors,
    domain::Domain,
    error::{ManagerError, ManagerResult},
    library::LibraryInfo,
    macros::ensure,
    NEUTRON_CHAIN,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Link {
    /// List of input accounts by id
    pub input_accounts_id: Vec<Id>,
    /// List of output accounts by id
    pub output_accounts_id: Vec<Id>,
    /// The library id
    pub library_id: Id,
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
pub struct ProgramConfig {
    // This is the id of the program
    #[serde(default)]
    pub id: u64,
    pub owner: String,
    /// A list of links between an accounts and libraries
    pub links: BTreeMap<Id, Link>,
    /// The list account data by id
    pub accounts: BTreeMap<Id, AccountInfo>,
    /// The list service data by id
    pub libraries: BTreeMap<Id, LibraryInfo>,
    /// A list of authorizations
    pub authorizations: Vec<AuthorizationInfo>,
    /// This is the info regarding authorization and processor contracts.
    /// Must be empty (Default) when a new program is instantiated.
    /// It gets populated when the program is instantiated.
    #[serde(default)]
    pub authorization_data: AuthorizationData,
}

impl ProgramConfig {
    /// Instantiate a program on all domains.
    pub async fn init(&mut self, connectors: &Connectors) -> ManagerResult<()> {
        let neutron_domain = Domain::CosmosCosmwasm(NEUTRON_CHAIN.to_string());
        // Verify the whole program config
        self.verify_new_config()?;

        // We create the neutron connector specifically because our registry is on neutron.
        let mut neutron_connector = connectors.get_or_create_connector(&neutron_domain).await?;

        // Get program next id from on chain program registry
        let program_id = neutron_connector.reserve_program_id().await?;
        self.id = program_id;

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
        // but if the creation fails, it means we continued the program instantiatoin for no reason.

        // Predict account addresses and get the instantiate datas for each account
        let mut account_instantiate_datas: HashMap<u64, InstantiateAccountData> = HashMap::new();

        for (account_id, account) in self.accounts.iter_mut() {
            if let AccountType::Addr { .. } = account.ty {
                // TODO: Probably should error? we are trying to instantiate a new program with existing account
                // this is problematic because we don't know who the admin of the account is
                // and how we can update its approved libraries list.
                // We can also assume the initier knows what he is doing, and will adjust those accounts manually.
                // We can also output what the needed operations to adjust it,
                // similar to what we will do on program update
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

            // Set active to be true just in case it was given false on init
            account.ty = AccountType::Addr { addr: addr.clone() };
            account.addr = Some(addr);
        }

        // We first predict the library addresses
        // Then we update the library configs with the account predicted addresses
        // for all input accounts we add the library address to the approved libraries list
        // and then instantiate the libraries
        for (_, link) in self.links.clone().iter() {
            let mut library = self.get_library(link.library_id)?;

            let mut domain_connector = connectors.get_or_create_connector(&library.domain).await?;
            let (library_addr, salt) = domain_connector
                .get_address(
                    self.id,
                    &library.config.to_string(),
                    format!("library_{}", link.library_id).as_str(),
                )
                .await?;

            let mut patterns =
                Vec::with_capacity(link.input_accounts_id.len() + link.output_accounts_id.len());
            let mut replace_with =
                Vec::with_capacity(link.input_accounts_id.len() + link.output_accounts_id.len());

            // At this stage we should already have all addresses for all account ids
            for account_id in link.input_accounts_id.iter() {
                let account_data = account_instantiate_datas.get_mut(account_id).ok_or(
                    ManagerError::FailedToRetrieveAccountInitData(*account_id, link.library_id),
                )?;
                // We add the library address to the approved libraries list of the input account
                account_data.add_library(library_addr.to_string());

                patterns.push(format!("|account_id|\":{account_id}"));
                replace_with.push(format!(
                    "library_account_addr\":\"{}\"",
                    account_data.addr.clone()
                ))
            }

            for account_id in link.output_accounts_id.iter() {
                let account_data = account_instantiate_datas.get(account_id).ok_or(
                    ManagerError::FailedToRetrieveAccountInitData(*account_id, link.library_id),
                )?;

                patterns.push(format!("|account_id|\":{account_id}"));
                replace_with.push(format!(
                    "library_account_addr\":\"{}\"",
                    account_data.addr.clone()
                ))
            }

            library.config.replace_config(patterns, replace_with)?;
            library.addr = Some(library_addr);

            self.save_library(link.library_id, &library);

            // Get processor address for this domain
            let processor_addr = self.get_processor_account_on_domain(library.domain.clone())?;

            // init the library
            domain_connector
                .instantiate_library(
                    self.id,
                    authorization_addr.clone(),
                    processor_addr,
                    link.library_id,
                    library.config,
                    salt,
                )
                .await?
        }

        // Instantiate accounts after we added all libraries addresses to the approved libraries list for each account
        for (account_id, account_instantiate_data) in account_instantiate_datas.iter() {
            let account = self.get_account(account_id)?;
            let mut domain_connector = connectors.get_or_create_connector(&account.domain).await?;
            let processor_addr = self
                .authorization_data
                .processor_addrs
                .get(&account.domain.to_string())
                .ok_or(ManagerError::ProcessorAddrNotFound(
                    account.domain.to_string(),
                ))?;

            domain_connector
                .instantiate_account(self.id, processor_addr.clone(), account_instantiate_data)
                .await?;
        }

        // Loop over authorizations, and change ids to their addresses
        for authorization in self.authorizations.iter_mut() {
            match &mut authorization.subroutine {
                valence_authorization_utils::authorization::Subroutine::Atomic(
                    atomic_subroutine,
                ) => {
                    atomic_subroutine.functions.iter_mut().for_each(|function| {
                        let addr = match &function.contract_address {
                            valence_library_utils::LibraryAccountType::Addr(a) => a.to_string(),
                            valence_library_utils::LibraryAccountType::AccountId(account_id) => {
                                account_instantiate_datas
                                    .get(account_id)
                                    .unwrap()
                                    .addr
                                    .clone()
                            }
                            valence_library_utils::LibraryAccountType::LibraryId(library_id) => {
                                self.libraries
                                    .get(library_id)
                                    .unwrap()
                                    .addr
                                    .clone()
                                    .unwrap()
                            }
                        };
                        function.contract_address =
                            valence_library_utils::LibraryAccountType::Addr(addr);
                    });
                }
                valence_authorization_utils::authorization::Subroutine::NonAtomic(
                    non_atomic_subroutine,
                ) => {
                    non_atomic_subroutine
                        .functions
                        .iter_mut()
                        .for_each(|function| {
                            let addr = match &function.contract_address {
                                valence_library_utils::LibraryAccountType::Addr(a) => a.to_string(),
                                valence_library_utils::LibraryAccountType::AccountId(
                                    account_id,
                                ) => account_instantiate_datas
                                    .get(account_id)
                                    .unwrap()
                                    .addr
                                    .clone(),
                                valence_library_utils::LibraryAccountType::LibraryId(
                                    library_id,
                                ) => self
                                    .libraries
                                    .get(library_id)
                                    .unwrap()
                                    .addr
                                    .clone()
                                    .unwrap(),
                            };
                            function.contract_address =
                                valence_library_utils::LibraryAccountType::Addr(addr);
                        });
                }
            }
        }

        // Verify the program was instantiated successfully
        self.verify_init_was_successful(connectors, account_instantiate_datas)
            .await?;

        // Get neutron connector again because we need it to change admin of the authorization contract
        let mut neutron_connector = connectors.get_or_create_connector(&neutron_domain).await?;

        neutron_connector
            .add_authorizations(authorization_addr.clone(), self.authorizations.clone())
            .await?;

        // Change the admin of the authorization contract to the owner of the program
        neutron_connector
            .change_authorization_owner(authorization_addr.clone(), self.owner.clone())
            .await?;

        // Save the program config to registry
        neutron_connector.save_program_config(self.clone()).await?;

        Ok(())
    }

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

        // Get all libraries and accounts ids that exists in links
        let mut libraries: BTreeSet<Id> = BTreeSet::new();
        let mut accounts: BTreeSet<Id> = BTreeSet::new();

        for (_, link) in self.links.iter() {
            for account_id in link.input_accounts_id.iter() {
                accounts.insert(*account_id);
            }

            for account_id in link.output_accounts_id.iter() {
                accounts.insert(*account_id);
            }

            libraries.insert(link.library_id);
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

        // Verify all libraries are referenced in links at least once
        for library_id in self.libraries.keys() {
            if !libraries.remove(library_id) {
                return Err(ManagerError::LibraryIdNotFoundInLinks(*library_id));
            }
        }

        // Verify libraries is empty, if its not, it means we have a link with a library id that doesn't exists
        ensure!(
            libraries.is_empty(),
            ManagerError::LibraryIdNotFoundLink(libraries)
        );

        // Verify all accounts are referenced in library config at least once (or else we have unused account)
        // accounts should be empty here
        for library in self.libraries.values() {
            accounts.extend(library.config.get_account_ids()?);
        }

        // We remove each account if we found
        // if account id was not removed, it means we didn't find it in any library config
        for account_id in self.accounts.keys() {
            if !accounts.remove(account_id) {
                return Err(ManagerError::AccountIdNotFoundInLibraries(*account_id));
            }
        }

        ensure!(
            accounts.is_empty(),
            ManagerError::AccountIdNotFoundLibraryConfig(accounts)
        );

        // Run the soft_validate method on each library config
        for _library in self.libraries.values() {
            // TODO: mock api for the connector
            // library.config.soft_validate_config()?;
        }

        Ok(())
    }

    /// Verify our program was instantiated successfully
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
                .query_program_registry(self.id)
                .await
                .is_err(),
            ManagerError::ProgramIdAlreadyExists(self.id)
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

        // verify libraries have an address and query on-chain contract to make sure its correct
        for (_, library) in self.libraries.iter() {
            let mut connector = connectors.get_or_create_connector(&library.domain).await?;

            connector.verify_library(library.addr.clone()).await?;
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

impl ProgramConfig {
    fn get_processor_account_on_domain(&mut self, domain: Domain) -> ManagerResult<String> {
        // Find either a processor bridge account or
        let processor_addr = self
            .authorization_data
            .processor_addrs
            .get_key_value(&domain.to_string())
            .ok_or(ManagerError::ProcessorAddrNotFound(domain.to_string()))?
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
        domains.extend(
            self.libraries
                .values()
                .map(|library| library.domain.clone()),
        );
        HashSet::from_iter(domains)
    }

    pub fn get_account(&self, account_id: impl GetId) -> ManagerResult<&AccountInfo> {
        self.accounts
            .get(&account_id.get_id())
            .ok_or(ManagerError::generic_err(format!(
                "Account with id {} not found",
                account_id.get_id()
            )))
    }

    pub fn get_library(&self, library_id: u64) -> ManagerResult<LibraryInfo> {
        self.libraries
            .get(&library_id)
            .ok_or(ManagerError::generic_err(format!(
                "Library with id {} not found",
                library_id
            )))
            .cloned()
    }

    pub fn get_processor_addr(&self, domain: &str) -> ManagerResult<String> {
        self.authorization_data
            .processor_addrs
            .get(domain)
            .ok_or(ManagerError::ProcessorAddrNotFound(domain.to_string()))
            .cloned()
    }

    fn save_library(&mut self, library_id: u64, library: &LibraryInfo) {
        self.libraries.insert(library_id, library.clone());
    }
}
