use cosmwasm_std::{
    instantiate2_address, testing::MockApi, Addr, Api, CodeInfoResponse, Coin, Empty, Uint128,
};
use cw20::Cw20Coin;
use cw_multi_test::{
    error::AnyResult, next_block, App, AppResponse, Contract, ContractWrapper, Executor,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt::Debug;

pub struct AccountTestSuiteBase {
    app: App,
    owner: Addr,
    account_code_id: u64,
    cw20_code_id: u64,
}

#[allow(dead_code)]
impl AccountTestSuiteBase {
    pub fn new(account_contract: Box<dyn Contract<Empty>>) -> Self {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");

        let account_code_id = app.store_code(account_contract);

        let cw20_code = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );

        let cw20_code_id = app.store_code(Box::new(cw20_code));

        Self {
            app,
            owner,
            account_code_id,
            cw20_code_id,
        }
    }
}

pub trait AccountTestSuite {
    fn app(&self) -> &App;
    fn app_mut(&mut self) -> &mut App;
    fn owner(&self) -> &Addr;
    fn account_code_id(&self) -> u64;
    fn cw20_code_id(&self) -> u64;

    fn api(&self) -> &MockApi {
        self.app().api()
    }

    fn get_contract_addr(&mut self, code_id: u64, salt: &str) -> Addr {
        let mut hasher = Sha256::new();
        hasher.update(salt);
        let salt = hasher.finalize().to_vec();

        let canonical_creator = self.api().addr_canonicalize(self.owner().as_str()).unwrap();

        let CodeInfoResponse { checksum, .. } =
            self.app().wrap().query_wasm_code_info(code_id).unwrap();

        let canonical_addr =
            instantiate2_address(checksum.as_slice(), &canonical_creator, &salt).unwrap();

        self.api().addr_humanize(&canonical_addr).unwrap()
    }

    fn contract_init<T: Serialize>(
        &mut self,
        code_id: u64,
        label: &str,
        init_msg: &T,
        funds: &[Coin],
    ) -> Addr {
        let owner = self.owner().clone();
        self.app_mut()
            .instantiate_contract(
                code_id,
                owner.clone(),
                &init_msg,
                funds,
                label,
                Some(owner.to_string()),
            )
            .unwrap()
    }

    fn contract_init2<T: Serialize>(
        &mut self,
        code_id: u64,
        salt: &str,
        init_msg: &T,
        funds: &[Coin],
    ) -> Addr {
        let mut hasher = Sha256::new();
        hasher.update(salt);
        let hashed_salt = hasher.finalize().to_vec();

        let owner = self.owner().clone();
        self.app_mut()
            .instantiate2_contract(
                code_id,
                owner.clone(),
                &init_msg,
                funds,
                salt.to_string(),
                Some(owner.to_string()),
                hashed_salt,
            )
            .unwrap()
    }

    fn contract_execute<T: Serialize + Debug>(
        &mut self,
        addr: Addr,
        msg: &T,
    ) -> AnyResult<AppResponse> {
        let sender = self.owner().clone();
        self.app_mut().execute_contract(sender, addr, &msg, &[])
    }

    fn next_block(&mut self) {
        self.app_mut().update_block(next_block);
    }

    fn query_balance(&self, addr: &Addr, denom: &str) -> Coin {
        self.app().wrap().query_balance(addr, denom).unwrap()
    }

    fn query_all_balances(&self, addr: &Addr) -> Vec<Coin> {
        #[allow(deprecated)]
        self.app().wrap().query_all_balances(addr).unwrap()
    }

    fn assert_balance(&self, addr: &Addr, coin: Coin) {
        let bal = self.query_balance(addr, &coin.denom);
        assert_eq!(bal, coin);
    }

    fn init_balance(&mut self, addr: &Addr, amounts: Vec<Coin>) {
        self.app_mut().init_modules(|router, _, store| {
            router.bank.init_balance(store, addr, amounts).unwrap();
        });
    }

    fn cw20_init(
        &mut self,
        name: &str,
        symbol: &str,
        decimals: u8,
        initial_balances: Vec<Cw20Coin>,
    ) -> Addr {
        let msg = cw20_base::msg::InstantiateMsg {
            name: name.to_string(),
            symbol: symbol.to_string(),
            decimals,
            initial_balances,
            mint: None,
            marketing: None,
        };

        let owner = self.owner().clone();
        let cw20_code_id = self.cw20_code_id();
        let cw20_addr = self
            .app_mut()
            .instantiate_contract(
                cw20_code_id,
                owner.clone(),
                &msg,
                &[],
                format!("CW20 {name}"),
                Some(owner.to_string()),
            )
            .unwrap();

        cw20_addr
    }

    fn cw20_query_balance(&self, addr: &Addr, cw20_addr: &Addr) -> Uint128 {
        let res = self.query_wasm::<_, cw20::BalanceResponse>(
            cw20_addr,
            &cw20::Cw20QueryMsg::Balance {
                address: addr.to_string(),
            },
        );
        res.balance
    }

    fn query_wasm<T, U>(&self, addr: &Addr, query: &T) -> U
    where
        T: Serialize,
        U: serde::de::DeserializeOwned,
    {
        self.app()
            .wrap()
            .query_wasm_smart::<U>(addr, &query)
            .unwrap()
    }

    fn send_tokens(&mut self, sender: &Addr, recipient: &Addr, amount: &[Coin]) -> AppResponse {
        self.app_mut()
            .send_tokens(sender.clone(), recipient.clone(), amount)
            .unwrap()
    }

    fn cw20_send_tokens(
        &mut self,
        cw20_addr: &Addr,
        sender: &Addr,
        recipient: &Addr,
        amount: u128,
    ) -> AppResponse {
        let msg = cw20::Cw20ExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount: amount.into(),
        };
        self.app_mut()
            .execute_contract(sender.clone(), cw20_addr.clone(), &msg, &[])
            .unwrap()
    }
}

#[allow(dead_code)]
impl AccountTestSuite for AccountTestSuiteBase {
    fn app(&self) -> &App {
        &self.app
    }

    fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    fn owner(&self) -> &Addr {
        &self.owner
    }

    fn account_code_id(&self) -> u64 {
        self.account_code_id
    }

    fn cw20_code_id(&self) -> u64 {
        self.cw20_code_id
    }
}
