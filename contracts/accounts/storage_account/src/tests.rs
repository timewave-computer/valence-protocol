use cosmwasm_std::{coin, coins, Addr, Coin};
use cw_multi_test::{error::AnyResult, App, AppResponse, ContractWrapper, Executor};
use cw_ownable::{Ownership, OwnershipError};
use getset::{Getters, Setters};
use itertools::sorted;
use std::string::ToString;
use valence_account_utils::{
    error::ContractError,
    msg::InstantiateMsg,
    testing::{AccountTestSuite, AccountTestSuiteBase},
};
use valence_middleware_utils::{
    canonical_types::bank::balance::ValenceBankBalance, type_registry::types::ValenceType,
};

use crate::msg::{ExecuteMsg, QueryMsg};

const NTRN: &str = "untrn";
const ONE_MILLION: u128 = 1_000_000_000_000_u128;
const BLOB_KEY: &str = "test_blob";

#[derive(Getters, Setters)]
struct StorageAccountTestSuite {
    #[getset(get)]
    inner: AccountTestSuiteBase,
    #[getset(get)]
    input_balances: Option<Vec<(u128, String)>>,
}

impl Default for StorageAccountTestSuite {
    fn default() -> Self {
        Self::new(None)
    }
}

#[allow(dead_code)]
impl StorageAccountTestSuite {
    pub fn new(input_balances: Option<Vec<(u128, String)>>) -> Self {
        // storage account contract
        let account_code = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );

        let inner = AccountTestSuiteBase::new(Box::new(account_code));

        Self {
            inner,
            input_balances,
        }
    }

    pub fn account_init(&mut self, approved_libraries: Vec<String>) -> Addr {
        let init_msg = InstantiateMsg {
            admin: self.owner().to_string(),
            approved_libraries,
        };
        let acc_addr = self.contract_init(self.account_code_id(), "base_account", &init_msg, &[]);

        if let Some(balances) = self.input_balances.as_ref().cloned() {
            let amounts = balances
                .iter()
                .map(|(amount, denom)| coin(*amount, denom.to_string()))
                .collect::<Vec<Coin>>();
            self.init_balance(&acc_addr, amounts);
        }

        acc_addr
    }

    fn approve_library(&mut self, addr: Addr, library: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::ApproveLibrary {
                library: library.to_string(),
            },
        )
    }

    fn approve_library_non_owner(&mut self, addr: Addr, library: Addr) -> AnyResult<AppResponse> {
        let non_owner = self.api().addr_make("non_owner");
        self.app_mut().execute_contract(
            non_owner,
            addr,
            &ExecuteMsg::ApproveLibrary {
                library: library.to_string(),
            },
            &[],
        )
    }

    fn remove_library(&mut self, addr: Addr, library: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::RemoveLibrary {
                library: library.to_string(),
            },
        )
    }

    fn remove_library_non_owner(&mut self, addr: Addr, library: Addr) -> AnyResult<AppResponse> {
        let non_owner = self.api().addr_make("non_owner");
        self.app_mut().execute_contract(
            non_owner,
            addr,
            &ExecuteMsg::RemoveLibrary {
                library: library.to_string(),
            },
            &[],
        )
    }

    fn transfer_ownership(&mut self, addr: Addr, new_owner: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
                new_owner: new_owner.to_string(),
                expiry: None,
            }),
        )
    }

    fn transfer_ownership_non_owner(
        &mut self,
        addr: Addr,
        new_owner: Addr,
    ) -> AnyResult<AppResponse> {
        self.app_mut().execute_contract(
            new_owner.clone(),
            addr,
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
                new_owner: new_owner.to_string(),
                expiry: None,
            }),
            &[],
        )
    }

    fn accept_ownership(&mut self, addr: Addr, new_owner: Addr) -> AnyResult<AppResponse> {
        self.app_mut().execute_contract(
            new_owner,
            addr,
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::AcceptOwnership {}),
            &[],
        )
    }

    fn renounce_ownership(&mut self, addr: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::RenounceOwnership {}),
        )
    }

    fn renounce_ownership_non_owner(&mut self, addr: Addr, sender: Addr) -> AnyResult<AppResponse> {
        self.app_mut().execute_contract(
            sender,
            addr,
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::RenounceOwnership {}),
            &[],
        )
    }

    fn post_valence_type(
        &mut self,
        addr: Addr,
        key: &str,
        variant: ValenceType,
    ) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::StoreValenceType {
                key: key.to_string(),
                variant,
            },
        )
    }

    fn query_approved_libraries(&mut self, addr: &Addr) -> Vec<Addr> {
        self.query_wasm(addr, &QueryMsg::ListApprovedLibraries {})
    }

    fn query_owership(&mut self, addr: &Addr) -> Ownership<Addr> {
        self.query_wasm(addr, &QueryMsg::Ownership {})
    }

    fn query_blob(&mut self, acc: Addr, key: &str) -> ValenceType {
        self.query_wasm(
            &acc,
            &QueryMsg::QueryValenceType {
                key: key.to_string(),
            },
        )
    }
}

impl AccountTestSuite for StorageAccountTestSuite {
    fn app(&self) -> &App {
        self.inner.app()
    }

    fn app_mut(&mut self) -> &mut App {
        self.inner.app_mut()
    }

    fn owner(&self) -> &Addr {
        self.inner.owner()
    }

    fn account_code_id(&self) -> u64 {
        self.inner.account_code_id()
    }

    fn cw20_code_id(&self) -> u64 {
        self.inner.cw20_code_id()
    }
}

#[test]
fn instantiate_with_no_approved_libraries() {
    let mut suite = StorageAccountTestSuite::default();

    // Instantiate storage account contract
    let acc = suite.account_init(vec![]);

    // Verify owner
    let owner_res: Ownership<Addr> = suite.query_owership(&acc);
    assert_eq!(owner_res.owner, Some(suite.owner().clone()));

    // Verify approved libraries
    let approved_libraries: Vec<Addr> = suite.query_approved_libraries(&acc);
    assert_eq!(approved_libraries, Vec::<Addr>::new());
}

#[test]
fn instantiate_with_approved_libraries() {
    let mut suite = StorageAccountTestSuite::default();

    let lib1 = suite.api().addr_make("library_1");
    let lib2 = suite.api().addr_make("library_2");

    // Instantiate storage account contract with approved libraries
    let acc = suite.account_init(vec![lib1.to_string(), lib2.to_string()]);

    // Verify owner
    let owner_res: Ownership<Addr> = suite.query_owership(&acc);
    assert_eq!(owner_res.owner, Some(suite.owner().clone()));

    // Verify approved libraries
    let approved_libraries: Vec<Addr> = suite.query_approved_libraries(&acc);
    assert_eq!(
        approved_libraries,
        sorted(vec![lib1, lib2]).collect::<Vec<Addr>>()
    );
}

#[test]
fn approve_library_by_owner() {
    let mut suite = StorageAccountTestSuite::default();

    let lib1 = suite.api().addr_make("library_1");
    let lib2 = suite.api().addr_make("library_2");
    let lib3 = suite.api().addr_make("library_3");

    // Instantiate storage account contract with approved libraries
    let acc = suite.account_init(vec![lib1.to_string(), lib2.to_string()]);

    // Owner approves new library on account
    suite.approve_library(acc.clone(), lib3.clone()).unwrap();

    // Verify approved libraries
    let approved_libraries = sorted(suite.query_approved_libraries(&acc)).collect::<Vec<Addr>>();
    assert_eq!(
        approved_libraries,
        sorted(vec![lib1, lib2, lib3]).collect::<Vec<Addr>>()
    );
}

#[test]
fn approve_library_by_non_owner() {
    let mut suite = StorageAccountTestSuite::default();

    let lib1 = suite.api().addr_make("library_1");
    let lib2 = suite.api().addr_make("library_2");
    let lib3 = suite.api().addr_make("library_3");

    // Instantiate storage account contract with approved libraries
    let acc = suite.account_init(vec![lib1.to_string(), lib2.to_string()]);

    // Owner approves new library on account
    let res = suite.approve_library_non_owner(acc.clone(), lib3.clone());
    assert!(res.is_err());

    assert_eq!(
        res.unwrap_err().downcast::<ContractError>().unwrap(),
        ContractError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
    );
}

#[test]
fn remove_library_by_owner() {
    let mut suite = StorageAccountTestSuite::default();

    let lib1 = suite.api().addr_make("library_1");
    let lib2 = suite.api().addr_make("library_2");
    let lib3 = suite.api().addr_make("library_3");

    // Instantiate storage account contract with approved libraries
    let acc = suite.account_init(vec![lib1.to_string(), lib2.to_string(), lib3.to_string()]);

    // Owner approves new library on account
    suite.remove_library(acc.clone(), lib2.clone()).unwrap();

    // Verify approved libraries
    let approved_libraries = sorted(suite.query_approved_libraries(&acc)).collect::<Vec<Addr>>();
    assert_eq!(
        approved_libraries,
        sorted(vec![lib1, lib3]).collect::<Vec<Addr>>()
    );
}

#[test]
fn remove_library_by_non_owner() {
    let mut suite = StorageAccountTestSuite::default();

    let lib1 = suite.api().addr_make("library_1");
    let lib2 = suite.api().addr_make("library_2");
    let lib3 = suite.api().addr_make("library_3");

    // Instantiate storage account contract with approved libraries
    let acc = suite.account_init(vec![lib1.to_string(), lib2.to_string(), lib3.to_string()]);

    // Owner approves new library on account
    let res = suite.remove_library_non_owner(acc.clone(), lib3.clone());
    assert!(res.is_err());

    assert_eq!(
        res.unwrap_err().downcast::<ContractError>().unwrap(),
        ContractError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
    );
}

#[test]
fn transfer_account_ownership_by_owner() {
    let mut suite = StorageAccountTestSuite::default();

    // Instantiate storage account contract
    let acc = suite.account_init(vec![]);

    // Owner transfer ownership to new owner
    let new_owner = suite.api().addr_make("new_owner");
    suite
        .transfer_ownership(acc.clone(), new_owner.clone())
        .unwrap();

    // Verify new owner is pending
    let owership: Ownership<Addr> = suite.query_owership(&acc);
    assert_eq!(
        owership,
        Ownership {
            owner: Some(suite.owner().clone()),
            pending_owner: Some(new_owner.clone()),
            pending_expiry: None,
        }
    );

    // New owner accepts ownership
    suite
        .accept_ownership(acc.clone(), new_owner.clone())
        .unwrap();

    // Verify ownership has been transferred
    let owership: Ownership<Addr> = suite.query_owership(&acc);
    assert_eq!(
        owership,
        Ownership {
            owner: Some(new_owner),
            pending_owner: None,
            pending_expiry: None,
        }
    );
}

#[test]
fn transfer_account_ownership_by_non_owner() {
    let mut suite = StorageAccountTestSuite::default();

    // Instantiate storage account contract
    let acc = suite.account_init(vec![]);

    // New owner tries to transfer ownership to itself
    let new_owner = suite.api().addr_make("new_owner");
    let res = suite.transfer_ownership_non_owner(acc.clone(), new_owner.clone());
    assert!(res.is_err());

    assert_eq!(
        res.unwrap_err().downcast::<ContractError>().unwrap(),
        ContractError::OwnershipError(OwnershipError::NotOwner)
    );
}

#[test]
fn renounce_account_ownership() {
    let mut suite = StorageAccountTestSuite::default();

    // Instantiate storage account contract
    let acc = suite.account_init(vec![]);

    // Owner renounces ownership
    suite.renounce_ownership(acc.clone()).unwrap();

    // Verify owership has been renounced
    let owership: Ownership<Addr> = suite.query_owership(&acc);
    assert_eq!(
        owership,
        Ownership {
            owner: None,
            pending_owner: None,
            pending_expiry: None,
        }
    );
}

#[test]
fn renounce_account_ownership_by_non_owner() {
    let mut suite = StorageAccountTestSuite::default();

    // Instantiate storage account contract
    let acc = suite.account_init(vec![]);

    // Owner renounces ownership
    let non_owner = suite.api().addr_make("non_owner");
    let res = suite.renounce_ownership_non_owner(acc.clone(), non_owner);
    assert!(res.is_err());

    assert_eq!(
        res.unwrap_err().downcast::<ContractError>().unwrap(),
        ContractError::OwnershipError(OwnershipError::NotOwner)
    );
}

#[test]
fn post_data_blob_admin() {
    let mut suite = StorageAccountTestSuite::new(Some(vec![(ONE_MILLION, NTRN.to_string())]));

    // Instantiate storage account contract
    let acc = suite.account_init(vec![]);

    let variant = ValenceType::BankBalance(ValenceBankBalance {
        assets: coins(ONE_MILLION, NTRN),
    });

    suite
        .post_valence_type(acc.clone(), BLOB_KEY, variant)
        .unwrap();

    // get the posted blob and try to reconstruct it
    let query_result = suite.query_blob(acc, BLOB_KEY);
    let balance_resp: ValenceBankBalance = match query_result {
        ValenceType::BankBalance(blob) => blob,
        _ => panic!("Unexpected variant type"),
    };

    // assert that the underlying data is the same
    assert_eq!(balance_resp.assets[0].denom, NTRN);
    assert_eq!(balance_resp.assets[0].amount.u128(), ONE_MILLION);
}

#[test]
#[should_panic(expected = "Not the admin or an approved library")]
fn post_data_blob_unauthorized() {
    let mut suite = StorageAccountTestSuite::new(Some(vec![(ONE_MILLION, NTRN.to_string())]));

    // Instantiate storage account contract
    let acc = suite.account_init(vec![]);

    let variant = ValenceType::BankBalance(ValenceBankBalance {
        assets: coins(ONE_MILLION, NTRN),
    });

    suite
        .inner
        .app_mut()
        .execute_contract(
            Addr::unchecked("not the real"),
            acc.clone(),
            &ExecuteMsg::StoreValenceType {
                key: BLOB_KEY.to_string(),
                variant,
            },
            &[],
        )
        .unwrap();
}
