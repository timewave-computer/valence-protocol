use cosmwasm_std::{
    instantiate2_address, testing::MockApi, Addr, Api, CodeInfoResponse, Coin, Uint128,
};
use cw20::Cw20Coin;
use cw_multi_test::{error::AnyResult, next_block, App, AppResponse, ContractWrapper, Executor};
use getset::{Getters, Setters};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt::Debug;

#[derive(Getters, Setters)]
pub struct ServiceTestSuiteBase {
    #[getset(get)]
    app: App,
    #[getset(get)]
    owner: Addr,
    #[getset(get)]
    processor: Addr,
    #[getset(get)]
    account_code_id: u64,
    #[getset(get)]
    cw20_code_id: u64,
}

impl Default for ServiceTestSuiteBase {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl ServiceTestSuiteBase {
    pub fn new() -> Self {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");
        let processor = app.api().addr_make("processor");

        let account_code = ContractWrapper::new(
            valence_base_account::contract::execute,
            valence_base_account::contract::instantiate,
            valence_base_account::contract::query,
        );

        let account_code_id = app.store_code(Box::new(account_code));

        let cw20_code = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );

        let cw20_code_id = app.store_code(Box::new(cw20_code));

        Self {
            app,
            owner,
            processor,
            account_code_id,
            cw20_code_id,
        }
    }
}

pub trait ServiceTestSuite {
    fn app(&mut self) -> &App;
    fn app_mut(&mut self) -> &mut App;
    fn api(&self) -> &MockApi;
    fn owner(&self) -> &Addr;
    fn processor(&self) -> &Addr;
    fn account_code_id(&self) -> u64;
    fn cw20_code_id(&self) -> u64;
    fn account_init(&mut self, salt: &str, approved_services: Vec<String>) -> Addr;
    fn get_contract_addr(&mut self, code_id: u64, salt: &str) -> Addr;

    fn contract_init<T: Serialize>(
        &mut self,
        code_id: u64,
        label: &str,
        init_msg: &T,
        funds: &[Coin],
    ) -> Addr;

    fn contract_init2<T: Serialize>(
        &mut self,
        code_id: u64,
        salt: &str,
        init_msg: &T,
        funds: &[Coin],
    ) -> Addr;

    fn contract_execute<T: Serialize + Debug>(
        &mut self,
        addr: Addr,
        msg: &T,
    ) -> AnyResult<AppResponse>;

    fn next_block(&mut self);

    fn query_balance(&self, addr: &Addr, denom: &str) -> Coin;

    fn query_all_balances(&self, addr: &Addr) -> Vec<Coin>;

    fn assert_balance(&self, addr: &Addr, coin: Coin);

    fn init_balance(&mut self, addr: &Addr, amounts: Vec<Coin>);

    fn cw20_init(
        &mut self,
        name: &str,
        symbol: &str,
        decimals: u8,
        initial_balances: Vec<Cw20Coin>,
    ) -> Addr;

    fn cw20_query_balance(&self, addr: &Addr, cw20_addr: &Addr) -> Uint128;

    fn query_wasm<T, U>(&self, addr: &Addr, query: &T) -> U
    where
        T: Serialize,
        U: serde::de::DeserializeOwned;

    fn send_tokens(&mut self, sender: &Addr, recipient: &Addr, amount: &[Coin]) -> AppResponse;

    fn cw20_send_tokens(
        &mut self,
        cw20_addr: &Addr,
        sender: &Addr,
        recipient: &Addr,
        amount: u128,
    ) -> AppResponse;
}

#[allow(dead_code)]
impl ServiceTestSuite for ServiceTestSuiteBase {
    fn app(&mut self) -> &App {
        &self.app
    }

    fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    fn api(&self) -> &MockApi {
        self.app.api()
    }

    fn owner(&self) -> &Addr {
        &self.owner
    }

    fn processor(&self) -> &Addr {
        &self.processor
    }

    fn account_code_id(&self) -> u64 {
        self.account_code_id
    }

    fn cw20_code_id(&self) -> u64 {
        self.cw20_code_id
    }

    fn account_init(&mut self, salt: &str, approved_services: Vec<String>) -> Addr {
        let init_msg = valence_base_account::msg::InstantiateMsg {
            admin: self.owner().to_string(),
            approved_services,
        };

        self.contract_init2(self.account_code_id, salt, &init_msg, &[])
    }

    fn get_contract_addr(&mut self, code_id: u64, salt: &str) -> Addr {
        let mut hasher = Sha256::new();
        hasher.update(salt);
        let salt = hasher.finalize().to_vec();

        let canonical_creator = self
            .app
            .api()
            .addr_canonicalize(self.owner().as_str())
            .unwrap();

        let CodeInfoResponse { checksum, .. } =
            self.app.wrap().query_wasm_code_info(code_id).unwrap();

        let canonical_addr =
            instantiate2_address(checksum.as_slice(), &canonical_creator, &salt).unwrap();

        self.app.api().addr_humanize(&canonical_addr).unwrap()
    }

    fn contract_init<T: Serialize>(
        &mut self,
        code_id: u64,
        label: &str,
        init_msg: &T,
        funds: &[Coin],
    ) -> Addr {
        self.app
            .instantiate_contract(
                code_id,
                self.owner().clone(),
                &init_msg,
                funds,
                label,
                Some(self.owner().to_string()),
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

        self.app
            .instantiate2_contract(
                code_id,
                self.owner().clone(),
                &init_msg,
                funds,
                salt.to_string(),
                Some(self.owner().to_string()),
                hashed_salt,
            )
            .unwrap()
    }

    fn contract_execute<T: Serialize + Debug>(
        &mut self,
        addr: Addr,
        msg: &T,
    ) -> AnyResult<AppResponse> {
        self.app
            .execute_contract(self.processor().clone(), addr, &msg, &[])
    }

    fn next_block(&mut self) {
        self.app.update_block(next_block);
    }

    fn query_balance(&self, addr: &Addr, denom: &str) -> Coin {
        self.app.wrap().query_balance(addr, denom).unwrap()
    }

    fn query_all_balances(&self, addr: &Addr) -> Vec<Coin> {
        self.app.wrap().query_all_balances(addr).unwrap()
    }

    fn assert_balance(&self, addr: &Addr, coin: Coin) {
        let bal = self.query_balance(addr, &coin.denom);
        assert_eq!(bal, coin);
    }

    fn init_balance(&mut self, addr: &Addr, amounts: Vec<Coin>) {
        self.app.init_modules(|router, _, store| {
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

        let cw20_addr = self
            .app
            .instantiate_contract(
                self.cw20_code_id,
                self.owner.clone(),
                &msg,
                &[],
                format!("CW20 {}", name),
                Some(self.owner().to_string()),
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
        self.app.wrap().query_wasm_smart::<U>(addr, &query).unwrap()
    }

    fn send_tokens(&mut self, sender: &Addr, recipient: &Addr, amount: &[Coin]) -> AppResponse {
        self.app
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
            amount: Uint128::from(amount),
        };
        self.app
            .execute_contract(sender.clone(), cw20_addr.clone(), &msg, &[])
            .unwrap()
    }
}
