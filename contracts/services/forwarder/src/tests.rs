use cosmwasm_std::{
    coin, instantiate2_address, testing::MockApi, Addr, Api, CodeInfoResponse, Coin, StdResult,
};
use cw_multi_test::{error::AnyResult, App, AppResponse, ContractWrapper, Executor};
use cw_ownable::Ownership;
use getset::{Getters, Setters};
use service_base::msg::{ExecuteMsg, InstantiateMsg};
use sha2::{Digest, Sha256};

use crate::msg::{
    ActionsMsgs, Config, ForwardingConfig, ForwardingConstraints, QueryMsg, ServiceConfig,
};

#[derive(Getters, Setters)]
struct ForwarderTestSuite {
    #[getset(get)]
    app: App,
    #[getset(get)]
    owner: Addr,
    #[getset(get)]
    processor: Addr,
    #[getset(get)]
    account_code_id: u64,
    #[getset(get)]
    account_salt: Vec<u8>,
    #[getset(get)]
    forwarder_code_id: u64,
    #[getset(get)]
    input_addr: Addr,
    #[getset(get)]
    output_addr: Addr,
}

impl Default for ForwarderTestSuite {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl ForwarderTestSuite {
    fn new() -> Self {
        Self::with_input_balances(None)
    }

    fn with_input_balances(input_balances: Option<&[(u128, &str)]>) -> Self {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");
        let processor = app.api().addr_make("processor");
        let output_addr = app.api().addr_make("output");

        // Input account
        let account_code = ContractWrapper::new(
            base_account::contract::execute,
            base_account::contract::instantiate,
            base_account::contract::query,
        );

        let account_code_id = app.store_code(Box::new(account_code));
        let mut hasher = Sha256::new();
        hasher.update("base_account".as_bytes());
        let salt = hasher.finalize().to_vec();

        let canonical_owner = app
            .api()
            .addr_canonicalize(app.api().addr_make("owner").as_str())
            .unwrap();
        let CodeInfoResponse { checksum, .. } =
            app.wrap().query_wasm_code_info(account_code_id).unwrap();
        let account_canonical_addr =
            instantiate2_address(checksum.as_slice(), &canonical_owner, &salt).unwrap();
        let input_addr = app.api().addr_humanize(&account_canonical_addr).unwrap();

        // Initialize input account with balances
        app.init_modules(|router, _, store| {
            let balances: Vec<Coin> = input_balances
                .unwrap_or(&[])
                .iter()
                .map(|b| coin(b.0, b.1))
                .collect();
            router
                .bank
                .init_balance(store, &input_addr, balances)
                .unwrap();
        });

        // Forwarder contract
        let forwarder_code = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );

        let forwarder_code_id = app.store_code(Box::new(forwarder_code));

        Self {
            app,
            owner,
            processor,
            account_code_id,
            account_salt: salt,
            forwarder_code_id,
            input_addr,
            output_addr,
        }
    }

    fn api(&self) -> &MockApi {
        self.app.api()
    }

    fn instantiate(&mut self, svc_cfg: &ServiceConfig) -> AnyResult<Addr> {
        let msg = InstantiateMsg {
            owner: self.owner.to_string(),
            processor: self.processor.to_string(),
            config: svc_cfg,
        };

        let forwarder_addr = self
            .app
            .instantiate_contract(
                self.forwarder_code_id,
                self.owner.clone(),
                &msg,
                &[],
                "Forwarder",
                None,
            )
            .unwrap();

        let init_msg = base_account::msg::InstantiateMsg {
            admin: self.owner.to_string(),
            approved_services: vec![forwarder_addr.to_string()],
        };
        let input_addr = self.app.instantiate2_contract(
            self.account_code_id,
            self.owner.clone(),
            &init_msg,
            &[],
            "input_account",
            None,
            self.account_salt.clone(),
        )?;
        assert_eq!(
            input_addr, self.input_addr,
            "input account address mismatch"
        );

        Ok(forwarder_addr)
    }

    fn execute(
        &mut self,
        addr: Addr,
        msg: ExecuteMsg<ActionsMsgs, Config>,
    ) -> AnyResult<AppResponse> {
        self.app
            .execute_contract(self.processor().clone(), addr, &msg, &[])
    }

    fn query_balance(&self, addr: &Addr, denom: &str) -> StdResult<Coin> {
        self.app.wrap().query_balance(addr, denom)
    }

    fn query_wasm<T>(&self, addr: &Addr, query: &QueryMsg) -> StdResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.app.wrap().query_wasm_smart::<T>(addr, &query)
    }

    fn service_config(
        &self,
        forwarding_configs: &[(String, ForwardingConfig)],
        forwarding_constraints: ForwardingConstraints,
    ) -> ServiceConfig {
        ServiceConfig::new(
            self.input_addr.clone().into(),
            self.output_addr.to_string(),
            forwarding_configs.iter().cloned().collect(),
            forwarding_constraints,
        )
    }
}

#[test]
fn instantiate_with_valid_config_succeeds() {
    // Arrange
    let mut suite = ForwarderTestSuite::default();

    let svc_cfg = suite.service_config(
        &[("untrn".to_string(), 1_000_000_000_000_u128.into())],
        Default::default(),
    );

    // Act
    let addr = suite.instantiate(&svc_cfg).unwrap();

    // Assert
    let owner_res = suite
        .query_wasm::<Ownership<Addr>>(&addr, &QueryMsg::GetOwner {})
        .unwrap();
    assert_eq!(owner_res.owner, Some(suite.owner().clone()));

    let processor_addr = suite
        .query_wasm::<Addr>(&addr, &QueryMsg::GetProcessor {})
        .unwrap();
    assert_eq!(processor_addr, suite.processor().clone());

    let cfg = suite
        .query_wasm::<Config>(&addr, &QueryMsg::GetServiceConfig {})
        .unwrap();
    assert_eq!(
        cfg,
        Config::new(
            suite.input_addr().clone(),
            suite.output_addr().clone(),
            svc_cfg.forwarding_configs,
            svc_cfg.forwarding_constraints
        )
    );
}

#[test]
fn forward_native_token_succeeds() {
    // Arrange
    let mut suite =
        ForwarderTestSuite::with_input_balances(Some(&[(1_000_000_000_000_u128, "untrn")]));

    let svc_cfg = suite.service_config(
        &[("untrn".to_string(), 1_000_000_000_000_u128.into())],
        Default::default(),
    );

    let addr = suite.instantiate(&svc_cfg).unwrap();

    // Act
    let _ = suite
        .execute(
            addr,
            ExecuteMsg::ProcessAction(ActionsMsgs::Forward { execution_id: None }),
        )
        .unwrap();

    // Assert

    // Input balance should be zero
    let input_balance = suite.query_balance(&suite.input_addr, "untrn").unwrap();
    assert_eq!(input_balance, coin(0, "untrn"));

    // Output balance should be 1_000_000_000_000
    let output_balance = suite.query_balance(&suite.output_addr, "untrn").unwrap();
    assert_eq!(output_balance, coin(1_000_000_000_000, "untrn"));
}

#[test]
fn forward_cw20_token_succeeds() {
    // Arrange
    let mut suite =
        ForwarderTestSuite::with_input_balances(Some(&[(1_000_000_000_000_u128, "untrn")]));

    let svc_cfg = suite.service_config(
        &[("untrn".to_string(), 1_000_000_000_000_u128.into())],
        Default::default(),
    );

    let addr = suite.instantiate(&svc_cfg).unwrap();

    // Act
    let _ = suite
        .execute(
            addr,
            ExecuteMsg::ProcessAction(ActionsMsgs::Forward { execution_id: None }),
        )
        .unwrap();

    // Assert

    // Input balance should be zero
    let input_balance = suite.query_balance(&suite.input_addr, "untrn").unwrap();
    assert_eq!(input_balance, coin(0, "untrn"));

    // Output balance should be 1_000_000_000_000
    let output_balance = suite.query_balance(&suite.output_addr, "untrn").unwrap();
    assert_eq!(output_balance, coin(1_000_000_000_000, "untrn"));
}
