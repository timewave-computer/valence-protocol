use cosmwasm_std::{
    coin, instantiate2_address, testing::MockApi, Addr, Api, CodeInfoResponse, Coin, StdResult,
};
use cw_multi_test::{error::AnyResult, next_block, App, AppResponse, ContractWrapper, Executor};
use cw_ownable::Ownership;
use cw_utils::Duration;
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

    fn next_block(&mut self) {
        self.app.update_block(next_block);
    }

    fn forward(&mut self, addr: Addr) -> AnyResult<AppResponse> {
        self.execute(
            addr,
            ExecuteMsg::ProcessAction(ActionsMsgs::Forward { execution_id: None }),
        )
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
fn instantiate_with_valid_config() {
    let mut suite = ForwarderTestSuite::default();

    // Set max amount to be forwarded to 1_000_000 NTRN (and no constraints)
    let cfg = suite.service_config(
        &[("untrn".to_string(), 1_000_000_000_000_u128.into())],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.instantiate(&cfg).unwrap();

    // Verify owner
    let owner_res = suite
        .query_wasm::<Ownership<Addr>>(&svc, &QueryMsg::GetOwner {})
        .unwrap();
    assert_eq!(owner_res.owner, Some(suite.owner().clone()));

    // Verify processor
    let processor_addr = suite
        .query_wasm::<Addr>(&svc, &QueryMsg::GetProcessor {})
        .unwrap();
    assert_eq!(processor_addr, suite.processor().clone());

    // Verify service config
    let svc_cfg = suite
        .query_wasm::<Config>(&svc, &QueryMsg::GetServiceConfig {})
        .unwrap();
    assert_eq!(
        svc_cfg,
        Config::new(
            suite.input_addr().clone(),
            suite.output_addr().clone(),
            cfg.forwarding_configs,
            cfg.forwarding_constraints
        )
    );
}

#[test]
fn forward_native_token_full_amount() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite =
        ForwarderTestSuite::with_input_balances(Some(&[(1_000_000_000_000_u128, "untrn")]));

    // Set max amount to be forwarded to 1_000_000 NTRN (and no constraints)
    let cfg = suite.service_config(
        &[("untrn".to_string(), 1_000_000_000_000_u128.into())],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.instantiate(&cfg).unwrap();

    // Execute forward action
    let res = suite.forward(svc);
    assert!(res.is_ok());

    // Verify input account's balance: should be zero
    let input_balance = suite.query_balance(&suite.input_addr, "untrn").unwrap();
    assert_eq!(input_balance, coin(0, "untrn"));

    // Verify output account's balance: should be 1_000_000 NTRN
    let output_balance = suite.query_balance(&suite.output_addr, "untrn").unwrap();
    assert_eq!(output_balance, coin(1_000_000_000_000, "untrn"));
}

#[test]
fn forward_native_token_partial_amount() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite =
        ForwarderTestSuite::with_input_balances(Some(&[(1_000_000_000_000_u128, "untrn")]));

    // Set max amount to be forwarded to 1_000 NTRN (and no constraints)
    let cfg = suite.service_config(
        &[("untrn".to_string(), 1_000_000_000.into())],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.instantiate(&cfg).unwrap();

    // Execute forward action
    let res = suite.forward(svc);
    assert!(res.is_ok());

    // Verify input account's balance: should be 999_000 NTR
    let input_balance = suite.query_balance(&suite.input_addr, "untrn").unwrap();
    assert_eq!(input_balance, coin(999_000_000_000_u128, "untrn"));

    // Verify output account's balance: should be 1_000 NTRN
    let output_balance = suite.query_balance(&suite.output_addr, "untrn").unwrap();
    assert_eq!(output_balance, coin(1_000_000_000, "untrn"));
}

// #[test]
// fn forward_cw20_token_succeeds() {
//     // Arrange
//     let mut suite =
//         ForwarderTestSuite::with_input_balances(Some(&[(1_000_000_000_000_u128, "untrn")]));

//     let svc_cfg = suite.service_config(
//         &[("untrn".to_string(), 1_000_000_000_000_u128.into())],
//         Default::default(),
//     );

//     let addr = suite.instantiate(&svc_cfg).unwrap();

//     // Act
//     let _ = suite
//         .execute(
//             addr,
//             ExecuteMsg::ProcessAction(ActionsMsgs::Forward { execution_id: None }),
//         )
//         .unwrap();

//     // Assert

//     // Input balance should be zero
//     let input_balance = suite.query_balance(&suite.input_addr, "untrn").unwrap();
//     assert_eq!(input_balance, coin(0, "untrn"));

//     // Output balance should be 1_000_000_000_000
//     let output_balance = suite.query_balance(&suite.output_addr, "untrn").unwrap();
//     assert_eq!(output_balance, coin(1_000_000_000_000, "untrn"));
// }

#[test]
fn forward_with_height_interval_constraint() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite =
        ForwarderTestSuite::with_input_balances(Some(&[(1_000_000_000_000_u128, "untrn")]));

    // Set max amount to be forwarded to 1_000 NTRN,
    // and constrain forward operation to once every 3 blocks.
    let cfg = suite.service_config(
        &[("untrn".to_string(), 1_000_000_000.into())],
        ForwardingConstraints::from(Duration::Height(3)),
    );

    // Instantiate Forwarder contract
    let svc = suite.instantiate(&cfg).unwrap();

    // BLOCK N
    // Execute forward action shoud succeed
    let mut res = suite.forward(svc.clone());
    assert!(res.is_ok());

    // BLOCK N+1
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+2
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+3
    suite.next_block();
    // Execute forward action shoud succeed
    res = suite.forward(svc.clone());
    assert!(res.is_ok());

    // Verify input account's balance: should be 998_000 NTR because of 2 successful forwards
    let input_balance = suite.query_balance(&suite.input_addr, "untrn").unwrap();
    assert_eq!(input_balance, coin(998_000_000_000_u128, "untrn"));

    // Verify output account's balance: should be 2_000 NTRN because of 2 successful forwards
    let output_balance = suite.query_balance(&suite.output_addr, "untrn").unwrap();
    assert_eq!(output_balance, coin(2_000_000_000, "untrn"));
}

#[test]
fn forward_with_time_interval_constraint() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite =
        ForwarderTestSuite::with_input_balances(Some(&[(1_000_000_000_000_u128, "untrn")]));

    // Set max amount to be forwarded to 1_000 NTRN,
    // and constrain forward operation to once every 20 seconds.
    let cfg = suite.service_config(
        &[("untrn".to_string(), 1_000_000_000.into())],
        ForwardingConstraints::from(Duration::Time(20)),
    );

    // Instantiate Forwarder contract
    let svc = suite.instantiate(&cfg).unwrap();

    // NOTE: This test verifies the time interval constraint by simulating the passage of time.
    // => cw-multi-test 'next_block' function uses 5 seconds per block.
    // Therefore, 4 blocks are required to simulate 20 seconds.
    // While unrealistic, this is a simple way to test the time interval constraint.

    // BLOCK N
    // Execute forward action shoud succeed
    let mut res = suite.forward(svc.clone());
    assert!(res.is_ok());

    // BLOCK N+1
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+2
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+3
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+4
    suite.next_block();
    // Execute forward action shoud succeed
    res = suite.forward(svc.clone());
    assert!(res.is_ok());

    // Verify input account's balance: should be 998_000 NTR because of 2 successful forwards
    let input_balance = suite.query_balance(&suite.input_addr, "untrn").unwrap();
    assert_eq!(input_balance, coin(998_000_000_000_u128, "untrn"));

    // Verify output account's balance: should be 2_000 NTRN because of 2 successful forwards
    let output_balance = suite.query_balance(&suite.output_addr, "untrn").unwrap();
    assert_eq!(output_balance, coin(2_000_000_000, "untrn"));
}
