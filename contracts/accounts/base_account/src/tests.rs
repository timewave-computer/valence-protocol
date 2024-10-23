use cosmwasm_std::{coin, Addr, BankMsg, Coin, CosmosMsg, StdResult, SubMsg, Uint128};
use cw20::Cw20Coin;
use cw_denom::CheckedDenom;
use cw_multi_test::{error::AnyResult, App, AppResponse, ContractWrapper, Executor};
use cw_ownable::{Ownership, OwnershipError};
use getset::{Getters, Setters};
use itertools::sorted;
use std::string::ToString;
use valence_account_utils::{
    error::{ContractError, UnauthorizedReason},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg, VALENCE_PAYLOAD_KEY},
    testing::{AccountTestSuite, AccountTestSuiteBase},
};

const NTRN: &str = "untrn";
const MEME: &str = "umeme";
const ONE_THOUSAND: u128 = 1_000_000_000_u128;
const ONE_MILLION: u128 = 1_000_000_000_000_u128;

#[derive(Getters, Setters)]
struct BaseAccountTestSuite {
    #[getset(get)]
    inner: AccountTestSuiteBase,
    #[getset(get)]
    input_balances: Option<Vec<(u128, String)>>,
}

impl Default for BaseAccountTestSuite {
    fn default() -> Self {
        Self::new(None)
    }
}

#[allow(dead_code)]
impl BaseAccountTestSuite {
    pub fn new(input_balances: Option<Vec<(u128, String)>>) -> Self {
        // Base account contract
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

    pub fn account_init(&mut self, approved_services: Vec<String>) -> Addr {
        let init_msg = InstantiateMsg {
            admin: self.owner().to_string(),
            approved_services,
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

    fn cw20_token_init(&mut self, name: &str, symbol: &str, amount: u128, addr: String) -> Addr {
        self.cw20_init(
            name,
            symbol,
            6,
            vec![Cw20Coin {
                address: addr.to_string(),
                amount: amount.into(),
            }],
        )
    }

    fn approve_service(&mut self, addr: Addr, service: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::ApproveService {
                service: service.to_string(),
            },
        )
    }

    fn approve_service_non_owner(&mut self, addr: Addr, service: Addr) -> AnyResult<AppResponse> {
        let non_owner = self.api().addr_make("non_owner");
        self.app_mut().execute_contract(
            non_owner,
            addr,
            &ExecuteMsg::ApproveService {
                service: service.to_string(),
            },
            &[],
        )
    }

    fn remove_service(&mut self, addr: Addr, service: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::RemoveService {
                service: service.to_string(),
            },
        )
    }

    fn remove_service_non_owner(&mut self, addr: Addr, service: Addr) -> AnyResult<AppResponse> {
        let non_owner = self.api().addr_make("non_owner");
        self.app_mut().execute_contract(
            non_owner,
            addr,
            &ExecuteMsg::RemoveService {
                service: service.to_string(),
            },
            &[],
        )
    }

    fn transfer_tokens(
        &mut self,
        account: Addr,
        sender: Addr,
        recipient: Addr,
        amounts: Vec<Coin>,
    ) -> AnyResult<AppResponse> {
        let transfer_messages = amounts
            .into_iter()
            .map(|c| CheckedDenom::Native(c.denom).get_transfer_to_message(&recipient, c.amount))
            .collect::<StdResult<Vec<CosmosMsg>>>()?;

        self.app_mut().execute_contract(
            sender,
            account,
            &ExecuteMsg::ExecuteMsg {
                msgs: transfer_messages,
            },
            &[],
        )
    }

    fn cw20_transfer_tokens(
        &mut self,
        account: Addr,
        cw20_addr: Addr,
        sender: Addr,
        recipient: Addr,
        amount: u128,
    ) -> AnyResult<AppResponse> {
        let cw20_transfer_message =
            CheckedDenom::Cw20(cw20_addr).get_transfer_to_message(&recipient, amount.into())?;
        self.app_mut().execute_contract(
            sender,
            account,
            &ExecuteMsg::ExecuteMsg {
                msgs: vec![cw20_transfer_message],
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

    fn execute_msg(
        &mut self,
        addr: Addr,
        msgs: Vec<cosmwasm_std::CosmosMsg>,
    ) -> AnyResult<AppResponse> {
        self.contract_execute(addr, &ExecuteMsg::ExecuteMsg { msgs })
    }

    fn execute_submsgs(
        &mut self,
        addr: Addr,
        sender: Addr,
        msgs: Vec<SubMsg>,
        payload: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.app_mut().execute_contract(
            sender,
            addr,
            &ExecuteMsg::ExecuteSubmsgs { msgs, payload },
            &[],
        )
    }

    fn query_approved_services(&mut self, addr: &Addr) -> Vec<Addr> {
        self.query_wasm(addr, &QueryMsg::ListApprovedServices {})
    }

    fn query_owership(&mut self, addr: &Addr) -> Ownership<Addr> {
        self.query_wasm(addr, &QueryMsg::Ownership {})
    }
}

impl AccountTestSuite for BaseAccountTestSuite {
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
fn instantiate_with_no_approved_services() {
    let mut suite = BaseAccountTestSuite::default();

    // Instantiate Base account contract
    let acc = suite.account_init(vec![]);

    // Verify owner
    let owner_res: Ownership<Addr> = suite.query_owership(&acc);
    assert_eq!(owner_res.owner, Some(suite.owner().clone()));

    // Verify approved services
    let approved_services: Vec<Addr> = suite.query_approved_services(&acc);
    assert_eq!(approved_services, Vec::<Addr>::new());
}

#[test]
fn instantiate_with_approved_services() {
    let mut suite = BaseAccountTestSuite::default();

    let svc1 = suite.api().addr_make("service_1");
    let svc2 = suite.api().addr_make("service_2");

    // Instantiate Base account contract with approved services
    let acc = suite.account_init(vec![svc1.to_string(), svc2.to_string()]);

    // Verify owner
    let owner_res: Ownership<Addr> = suite.query_owership(&acc);
    assert_eq!(owner_res.owner, Some(suite.owner().clone()));

    // Verify approved services
    let approved_services: Vec<Addr> = suite.query_approved_services(&acc);
    assert_eq!(approved_services, vec![svc1, svc2]);
}

#[test]
fn approve_service_by_owner() {
    let mut suite = BaseAccountTestSuite::default();

    let svc1 = suite.api().addr_make("service_1");
    let svc2 = suite.api().addr_make("service_2");
    let svc3 = suite.api().addr_make("service_3");

    // Instantiate Base account contract with approved services
    let acc = suite.account_init(vec![svc1.to_string(), svc2.to_string()]);

    // Owner approves new service on account
    suite.approve_service(acc.clone(), svc3.clone()).unwrap();

    // Verify approved services
    let approved_services = sorted(suite.query_approved_services(&acc)).collect::<Vec<Addr>>();
    assert_eq!(
        approved_services,
        sorted(vec![svc1, svc2, svc3]).collect::<Vec<Addr>>()
    );
}

#[test]
fn approve_service_by_non_owner() {
    let mut suite = BaseAccountTestSuite::default();

    let svc1 = suite.api().addr_make("service_1");
    let svc2 = suite.api().addr_make("service_2");
    let svc3 = suite.api().addr_make("service_3");

    // Instantiate Base account contract with approved services
    let acc = suite.account_init(vec![svc1.to_string(), svc2.to_string()]);

    // Owner approves new service on account
    let res = suite.approve_service_non_owner(acc.clone(), svc3.clone());
    assert!(res.is_err());

    assert_eq!(
        res.unwrap_err().downcast::<ContractError>().unwrap(),
        ContractError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
    );
}

#[test]
fn remove_service_by_owner() {
    let mut suite = BaseAccountTestSuite::default();

    let svc1 = suite.api().addr_make("service_1");
    let svc2 = suite.api().addr_make("service_2");
    let svc3 = suite.api().addr_make("service_3");

    // Instantiate Base account contract with approved services
    let acc = suite.account_init(vec![svc1.to_string(), svc2.to_string(), svc3.to_string()]);

    // Owner approves new service on account
    suite.remove_service(acc.clone(), svc2.clone()).unwrap();

    // Verify approved services
    let approved_services = sorted(suite.query_approved_services(&acc)).collect::<Vec<Addr>>();
    assert_eq!(
        approved_services,
        sorted(vec![svc1, svc3]).collect::<Vec<Addr>>()
    );
}

#[test]
fn remove_service_by_non_owner() {
    let mut suite = BaseAccountTestSuite::default();

    let svc1 = suite.api().addr_make("service_1");
    let svc2 = suite.api().addr_make("service_2");
    let svc3 = suite.api().addr_make("service_3");

    // Instantiate Base account contract with approved services
    let acc = suite.account_init(vec![svc1.to_string(), svc2.to_string(), svc3.to_string()]);

    // Owner approves new service on account
    let res = suite.remove_service_non_owner(acc.clone(), svc3.clone());
    assert!(res.is_err());

    assert_eq!(
        res.unwrap_err().downcast::<ContractError>().unwrap(),
        ContractError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
    );
}

#[test]
fn transfer_native_tokens_by_owner() {
    let mut suite = BaseAccountTestSuite::new(Some(vec![(ONE_MILLION, NTRN.to_string())]));

    // Instantiate Base account contract
    let acc = suite.account_init(vec![]);

    // Assert account balance
    suite.assert_balance(&acc, coin(ONE_MILLION, NTRN));

    // Owner transfers tokens from account
    let recipient = suite.api().addr_make("recipient");
    suite
        .transfer_tokens(
            acc.clone(),
            suite.owner().clone(),
            recipient.clone(),
            vec![coin(ONE_THOUSAND, NTRN)],
        )
        .unwrap();

    // Verify account & recipient balances
    suite.assert_balance(&acc, coin(999_000_000_000_u128, NTRN));
    suite.assert_balance(&recipient, coin(ONE_THOUSAND, NTRN));
}

#[test]
fn transfer_native_tokens_by_approved_service() {
    let mut suite = BaseAccountTestSuite::new(Some(vec![(ONE_MILLION, NTRN.to_string())]));

    let svc1 = suite.api().addr_make("service_1");

    // Instantiate Base account contract
    let acc = suite.account_init(vec![svc1.to_string()]);

    // Assert account balance
    suite.assert_balance(&acc, coin(ONE_MILLION, NTRN));

    // Owner transfers tokens from account
    let recipient = suite.api().addr_make("recipient");
    suite
        .transfer_tokens(
            acc.clone(),
            svc1,
            recipient.clone(),
            vec![coin(ONE_THOUSAND, NTRN)],
        )
        .unwrap();

    // Verify account & recipient balances
    suite.assert_balance(&acc, coin(ONE_MILLION - ONE_THOUSAND, NTRN));
    suite.assert_balance(&recipient, coin(ONE_THOUSAND, NTRN));
}

#[test]
fn transfer_native_tokens_by_unknown_account() {
    let mut suite = BaseAccountTestSuite::new(Some(vec![(ONE_MILLION, NTRN.to_string())]));

    let svc1 = suite.api().addr_make("service_1");

    // Instantiate Base account contract
    let acc = suite.account_init(vec![svc1.to_string()]);

    // Assert account balance
    suite.assert_balance(&acc, coin(ONE_MILLION, NTRN));

    let non_owner = suite.api().addr_make("non_owner");

    // Owner transfers tokens from account
    let recipient = suite.api().addr_make("recipient");
    let res = suite.transfer_tokens(
        acc.clone(),
        non_owner,
        recipient.clone(),
        vec![coin(ONE_THOUSAND, NTRN)],
    );
    assert!(res.is_err());

    assert_eq!(
        res.unwrap_err().downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized(UnauthorizedReason::NotAdminOrApprovedService)
    );
}

#[test]
fn transfer_cw20_tokens_by_owner() {
    let mut suite = BaseAccountTestSuite::default();

    // Instantiate Base account contract
    let acc = suite.account_init(vec![]);

    // Instantiate CW20 token contract, and initialize input account with 1_000_000 MEME
    let cw20_addr = suite.cw20_token_init(MEME, "MEME", ONE_MILLION, acc.to_string());

    // Assert account balance
    assert_eq!(
        suite.cw20_query_balance(&acc, &cw20_addr),
        Uint128::from(ONE_MILLION)
    );

    // Owner transfers tokens from account
    let recipient = suite.api().addr_make("recipient");
    suite
        .cw20_transfer_tokens(
            acc.clone(),
            cw20_addr.clone(),
            suite.owner().clone(),
            recipient.clone(),
            ONE_THOUSAND,
        )
        .unwrap();

    // Verify account & recipient balances
    assert_eq!(
        suite.cw20_query_balance(&acc, &cw20_addr),
        Uint128::from(ONE_MILLION - ONE_THOUSAND)
    );
    assert_eq!(
        suite.cw20_query_balance(&recipient, &cw20_addr),
        Uint128::from(ONE_THOUSAND)
    );
}

#[test]
fn transfer_cw20_tokens_by_approved_service() {
    let mut suite = BaseAccountTestSuite::default();

    let svc1 = suite.api().addr_make("service_1");

    // Instantiate Base account contract
    let acc = suite.account_init(vec![svc1.to_string()]);

    // Instantiate CW20 token contract, and initialize input account with 1_000_000 MEME
    let cw20_addr = suite.cw20_token_init(MEME, "MEME", ONE_MILLION, acc.to_string());

    // Assert account balance
    assert_eq!(
        suite.cw20_query_balance(&acc, &cw20_addr),
        Uint128::from(ONE_MILLION)
    );

    // Owner transfers tokens from account
    let recipient = suite.api().addr_make("recipient");
    suite
        .cw20_transfer_tokens(
            acc.clone(),
            cw20_addr.clone(),
            svc1,
            recipient.clone(),
            ONE_THOUSAND,
        )
        .unwrap();

    // Verify account & recipient balances
    assert_eq!(
        suite.cw20_query_balance(&acc, &cw20_addr),
        Uint128::from(ONE_MILLION - ONE_THOUSAND)
    );
    assert_eq!(
        suite.cw20_query_balance(&recipient, &cw20_addr),
        Uint128::from(ONE_THOUSAND)
    );
}

#[test]
fn transfer_cw20_tokens_by_unknown_account() {
    let mut suite = BaseAccountTestSuite::default();

    let svc1 = suite.api().addr_make("service_1");

    // Instantiate Base account contract
    let acc = suite.account_init(vec![svc1.to_string()]);

    // Instantiate CW20 token contract, and initialize input account with 1_000_000 MEME
    let cw20_addr = suite.cw20_token_init(MEME, "MEME", ONE_MILLION, acc.to_string());

    // Assert account balance
    assert_eq!(
        suite.cw20_query_balance(&acc, &cw20_addr),
        Uint128::from(ONE_MILLION)
    );

    let non_owner = suite.api().addr_make("non_owner");

    // Owner transfers tokens from account
    let recipient = suite.api().addr_make("recipient");
    let res = suite.cw20_transfer_tokens(
        acc.clone(),
        cw20_addr.clone(),
        non_owner,
        recipient.clone(),
        ONE_THOUSAND,
    );
    assert!(res.is_err());

    assert_eq!(
        res.unwrap_err().downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized(UnauthorizedReason::NotAdminOrApprovedService)
    );
}

#[test]
fn transfer_account_ownership_by_owner() {
    let mut suite = BaseAccountTestSuite::default();

    // Instantiate Base account contract
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
    let mut suite = BaseAccountTestSuite::default();

    // Instantiate Base account contract
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
    let mut suite = BaseAccountTestSuite::default();

    // Instantiate Base account contract
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
    let mut suite = BaseAccountTestSuite::default();

    // Instantiate Base account contract
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
fn execute_submessages_by_approved_service() {
    let mut suite = BaseAccountTestSuite::new(Some(vec![(ONE_MILLION, NTRN.to_string())]));

    let svc1 = suite.api().addr_make("service_1");
    let recipient = suite.api().addr_make("recipient");

    // instantiate base account contract
    let acc = suite.account_init(vec![svc1.to_string()]);
    suite.assert_balance(&acc, coin(ONE_MILLION, NTRN));

    // create a submessage
    let transfer_msg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient.to_string(),
        amount: vec![coin(ONE_THOUSAND, NTRN)],
    }));

    // approved service executes submessage
    suite
        .execute_submsgs(acc.clone(), svc1, vec![transfer_msg], None)
        .unwrap();

    // verify account & recipient balances
    suite.assert_balance(&acc, coin(ONE_MILLION - ONE_THOUSAND, NTRN));
    suite.assert_balance(&recipient, coin(ONE_THOUSAND, NTRN));
}

#[test]
#[should_panic(expected = "Unauthorized: Not an approved service")]
fn execute_submessages_by_owner_unauthorized_panics() {
    let mut suite = BaseAccountTestSuite::new(Some(vec![(ONE_MILLION, NTRN.to_string())]));
    let recipient = suite.api().addr_make("recipient");

    // instantiate base account
    let acc = suite.account_init(vec![]);

    // create a submessage
    let transfer_msg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient.to_string(),
        amount: vec![coin(ONE_THOUSAND, NTRN)],
    }));

    // owner executes submessage
    suite
        .execute_submsgs(acc.clone(), suite.owner().clone(), vec![transfer_msg], None)
        .unwrap();
}

#[test]
fn execute_submessages_with_payload() {
    let mut suite = BaseAccountTestSuite::new(Some(vec![(ONE_MILLION, NTRN.to_string())]));

    let svc1 = suite.api().addr_make("service_1");
    let recipient = suite.api().addr_make("recipient");

    // instantiate base account
    let acc = suite.account_init(vec![svc1.to_string()]);

    // assert account balance
    suite.assert_balance(&acc, coin(ONE_MILLION, NTRN));

    // create a submessage
    let transfer_msg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient.to_string(),
        amount: vec![coin(ONE_THOUSAND, NTRN)],
    }));

    // create a payload, mocking the osmo-cl-lper Config
    let payload = Some(
        r#"{"input_addr":"neutron1input123456789abcdefghijklmnopqrstuvwxyz","output_addr":"neutron1output987654321zyxwvutsrqponmlkjihgfedcba","lp_config":{"pool_id": "42","pool_asset_1": "untrn","pool_asset_2": "umeme","global_tick_range": {"lower_tick": "-1000","upper_tick": "1000"}}}"#
        .to_string(),
    );

    // approved service executes submessage with payload
    let res = suite
        .execute_submsgs(acc.clone(), svc1, vec![transfer_msg], payload.clone())
        .unwrap();

    // verify the payload is included in the response attributes
    let mut found = false;
    for event in res.events {
        for attr in event.attributes {
            if attr.key == VALENCE_PAYLOAD_KEY {
                assert_eq!(attr.value, payload.clone().unwrap());
                found = true;
            }
        }
    }
    assert!(found);
}

#[test]
fn execute_submessages_without_payload() {
    let mut suite = BaseAccountTestSuite::new(Some(vec![(ONE_MILLION, NTRN.to_string())]));

    let svc1 = suite.api().addr_make("service_1");
    let recipient = suite.api().addr_make("recipient");

    // instantiate base account
    let acc = suite.account_init(vec![svc1.to_string()]);

    // assert account balance
    suite.assert_balance(&acc, coin(ONE_MILLION, NTRN));

    // create a submessage
    let transfer_msg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient.to_string(),
        amount: vec![coin(ONE_THOUSAND, NTRN)],
    }));

    // approved service executes submessage with no payload
    let res = suite
        .execute_submsgs(acc.clone(), svc1, vec![transfer_msg], None)
        .unwrap();

    // verify that no payload is included in the response attributes
    for event in res.events {
        for attr in event.attributes {
            if attr.key == VALENCE_PAYLOAD_KEY {
                panic!();
            }
        }
    }
}
