use cosmwasm_std::{
    coin, instantiate2_address,
    testing::{MockApi, MockStorage},
    Addr, Api, CodeInfoResponse, Coin, CustomMsg, CustomQuery, Empty, Uint128,
};
use cw20::Cw20Coin;
use cw_multi_test::{
    error::AnyResult, next_block, App, AppResponse, BankKeeper, ContractWrapper, Executor,
    FailingModule, GovFailingModule, IbcFailingModule, Module, StargateFailing, WasmKeeper,
};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Debug;

pub type TestApp<ExecC, QueryC, TCustom = FailingModule<ExecC, QueryC, Empty>> = App<
    BankKeeper,
    MockApi,
    MockStorage,
    TCustom,
    WasmKeeper<ExecC, QueryC>,
    FailingModule<Empty, Empty, Empty>,
    FailingModule<Empty, Empty, Empty>,
    IbcFailingModule,
    GovFailingModule,
    StargateFailing,
>;

pub struct LibraryTestSuiteBase {
    app: App,
    owner: Addr,
    processor: Addr,
    account_code_id: u64,
    cw20_code_id: u64,
}

impl Default for LibraryTestSuiteBase {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl LibraryTestSuiteBase {
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

pub trait LibraryTestSuiteApp<ExecC, QueryC>
where
    ExecC: CustomMsg + Debug + DeserializeOwned + 'static,
    QueryC: CustomQuery + Debug + DeserializeOwned + 'static,
{
}

pub trait LibraryTestSuite<
    ExecC,
    QueryC,
    TCustom: Module<ExecT = ExecC, QueryT = QueryC> = FailingModule<ExecC, QueryC, Empty>,
> where
    ExecC: CustomMsg + Debug + DeserializeOwned + 'static,
    QueryC: CustomQuery + Debug + DeserializeOwned + 'static,
{
    fn app(&self) -> &TestApp<ExecC, QueryC, TCustom>;
    fn app_mut(&mut self) -> &mut TestApp<ExecC, QueryC, TCustom>;
    fn owner(&self) -> &Addr;
    fn processor(&self) -> &Addr;
    fn account_code_id(&self) -> u64;
    fn cw20_code_id(&self) -> u64;

    fn api<'b>(&'b self) -> &'b MockApi
    where
        TCustom: 'b,
    {
        self.app().api()
    }

    fn account_init(&mut self, salt: &str, approved_libraries: Vec<String>) -> Addr {
        let init_msg = valence_account_utils::msg::InstantiateMsg {
            admin: self.owner().to_string(),
            approved_libraries,
        };

        self.contract_init2(self.account_code_id(), salt, &init_msg, &[])
    }

    fn account_init_with_balances(&mut self, salt: &str, balances: Vec<(u128, String)>) -> Addr {
        let addr = self.account_init(salt, vec![]);

        if !balances.is_empty() {
            let balances = balances
                .into_iter()
                .map(|(amount, denom)| coin(amount, denom))
                .collect();
            self.init_balance(&addr, balances);
        }

        addr
    }

    fn account_approve_library(&mut self, addr: Addr, library: String) -> AnyResult<AppResponse> {
        let sender = self.owner().clone();
        self.app_mut().execute_contract(
            sender,
            addr,
            &valence_account_utils::msg::ExecuteMsg::ApproveLibrary { library },
            &[],
        )
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
        let sender = self.processor().clone();
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

    fn assert_balance(&self, addr: &Addr, amount: u128, denom: &str) {
        let bal = self.query_balance(addr, denom);
        assert_eq!(bal, coin(amount, denom));
    }

    fn assert_cw20_balance(&self, addr: &Addr, amount: u128, cw20_addr: &Addr) {
        let bal = self.cw20_query_balance(addr, cw20_addr);
        assert_eq!(bal, Uint128::new(amount));
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

impl LibraryTestSuite<Empty, Empty> for LibraryTestSuiteBase {
    fn app(&self) -> &App {
        &self.app
    }

    fn app_mut(&mut self) -> &mut App {
        &mut self.app
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
}

pub struct CustomLibraryTestSuiteBase<
    ExecC,
    QueryC,
    TCustom: Module<ExecT = ExecC, QueryT = QueryC>,
> where
    ExecC: CustomMsg + Debug + DeserializeOwned,
    QueryC: CustomQuery + Debug + DeserializeOwned,
{
    app: TestApp<ExecC, QueryC, TCustom>,
    owner: Addr,
    processor: Addr,
    account_code_id: u64,
    cw20_code_id: u64,
}

impl<ExecC, QueryC, TCustom: Module<ExecT = ExecC, QueryT = QueryC>>
    CustomLibraryTestSuiteBase<ExecC, QueryC, TCustom>
where
    ExecC: CustomMsg + Debug + DeserializeOwned + 'static,
    QueryC: CustomQuery + Debug + DeserializeOwned + 'static,
{
    pub fn new(mut app: TestApp<ExecC, QueryC, TCustom>) -> Self {
        let owner = app.api().addr_make("owner");
        let processor = app.api().addr_make("processor");

        let account_code = ContractWrapper::new_with_empty(
            valence_base_account::contract::execute,
            valence_base_account::contract::instantiate,
            valence_base_account::contract::query,
        );

        let account_code_id = app.store_code(Box::new(account_code));

        let cw20_code = ContractWrapper::new_with_empty(
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

impl<ExecC, QueryC, TCustom: Module<ExecT = ExecC, QueryT = QueryC>>
    LibraryTestSuite<ExecC, QueryC, TCustom> for CustomLibraryTestSuiteBase<ExecC, QueryC, TCustom>
where
    ExecC: CustomMsg + Debug + DeserializeOwned + 'static,
    QueryC: CustomQuery + Debug + DeserializeOwned + 'static,
{
    fn app(&self) -> &TestApp<ExecC, QueryC, TCustom> {
        &self.app
    }

    fn app_mut(&mut self) -> &mut TestApp<ExecC, QueryC, TCustom> {
        &mut self.app
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
}
