use cosmwasm_std::{
    coin, instantiate2_address, testing::MockApi, Addr, Api, CodeInfoResponse, Coin, StdResult,
    Uint128,
};
use cw20::Cw20Coin;
use cw_denom::{CheckedDenom, UncheckedDenom};
use cw_multi_test::{error::AnyResult, next_block, App, AppResponse, ContractWrapper, Executor};
use cw_ownable::Ownership;
use cw_utils::Duration;
use getset::{Getters, Setters};
use serde::Serialize;
use service_base::msg::{ExecuteMsg, InstantiateMsg};
use sha2::{Digest, Sha256};

use crate::msg::{ActionsMsgs, Config, ForwardingConstraints, QueryMsg, ServiceConfig};

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

            // Always give owner some NTRN
            router
                .bank
                .init_balance(store, &owner, vec![coin(1_000_000_000_000, "untrn")])
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

        let forwarder_addr = self.app.instantiate_contract(
            self.forwarder_code_id,
            self.owner.clone(),
            &msg,
            &[],
            "Forwarder",
            None,
        )?;

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

    fn instantiate_cw20(
        &mut self,
        name: String,
        symbol: String,
        decimals: u8,
        initial_balances: Vec<Cw20Coin>,
    ) -> AnyResult<Addr> {
        let cw20_code = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );

        let cw20_code_id = self.app.store_code(Box::new(cw20_code));

        let msg = cw20_base::msg::InstantiateMsg {
            name: name.clone(),
            symbol,
            decimals,
            initial_balances,
            mint: None,
            marketing: None,
        };

        let cw20_addr = self
            .app
            .instantiate_contract(
                cw20_code_id,
                self.owner.clone(),
                &msg,
                &[],
                format!("CW20 {}", name),
                None,
            )
            .unwrap();

        Ok(cw20_addr)
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

    fn query_cw20_balance(&self, addr: &Addr, cw20_addr: &Addr) -> StdResult<Uint128> {
        let balance: cw20::BalanceResponse = self.query_wasm(
            cw20_addr,
            &cw20::Cw20QueryMsg::Balance {
                address: addr.to_string(),
            },
        )?;
        Ok(balance.balance)
    }

    fn query_wasm<T>(&self, addr: &Addr, query: &impl Serialize) -> StdResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.app.wrap().query_wasm_smart::<T>(addr, &query)
    }

    fn service_config(
        &self,
        forwarding_configs: Vec<(UncheckedDenom, u128)>,
        forwarding_constraints: ForwardingConstraints,
    ) -> ServiceConfig {
        ServiceConfig::new(
            self.input_addr.clone().into(),
            self.output_addr.to_string(),
            forwarding_configs.into_iter().map(Into::into).collect(),
            forwarding_constraints,
        )
    }

    fn send_tokens(
        &mut self,
        sender: &Addr,
        recipient: &Addr,
        amount: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app
            .send_tokens(sender.clone(), recipient.clone(), amount)
    }

    fn send_cw20_tokens(
        &mut self,
        cw20_addr: &Addr,
        sender: &Addr,
        recipient: &Addr,
        amount: u128,
    ) -> AnyResult<AppResponse> {
        let msg = cw20::Cw20ExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount: Uint128::from(amount),
        };
        self.app
            .execute_contract(sender.clone(), cw20_addr.clone(), &msg, &[])
    }
}

#[test]
fn instantiate_with_valid_config() {
    let mut suite = ForwarderTestSuite::default();

    // Set max amount to be forwarded to 1_000_000 NTRN (and no constraints)
    let cfg = suite.service_config(
        vec![(
            UncheckedDenom::Native("untrn".to_string()),
            1_000_000_000_000_u128,
        )],
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
            vec![(
                CheckedDenom::Native("untrn".to_string()),
                1_000_000_000_000_u128
            )
                .into()],
            cfg.forwarding_constraints
        )
    );
}

#[test]
fn instantiate_fails_for_duplicate_denoms() {
    let mut suite = ForwarderTestSuite::default();

    // Set max amount to be forwarded to 1_000_000 NTRN (and no constraints)
    let cfg = suite.service_config(
        vec![
            (
                UncheckedDenom::Native("untrn".to_string()),
                1_000_000_000_000_u128,
            ),
            (UncheckedDenom::Native("untrn".to_string()), 1_000_000_u128),
        ],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let res = suite.instantiate(&cfg);
    assert!(res.is_err());

    assert_eq!(
        res.unwrap_err().root_cause().to_string(),
        "Configuration error: Duplicate denom 'untrn' in forwarding config."
    );
}

#[test]
fn instantiate_fails_for_unknown_cw20() {
    let mut suite = ForwarderTestSuite::default();
    let fake_cw20 = suite.api().addr_make("umeme");

    // Set max amount to be forwarded to 1_000_000 NTRN (and no constraints)
    let cfg = suite.service_config(
        vec![(
            UncheckedDenom::Cw20(fake_cw20.to_string()),
            1_000_000_000_000_u128,
        )],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let res = suite.instantiate(&cfg);
    assert!(res.is_err());

    assert!(res
        .unwrap_err()
        .root_cause()
        .to_string()
        .starts_with("Configuration error: invalid cw20 - did not respond to `TokenInfo` query"));
}

#[test]
fn forward_native_token_full_amount() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite =
        ForwarderTestSuite::with_input_balances(Some(&[(1_000_000_000_000_u128, "untrn")]));

    // Set max amount to be forwarded to 1_000_000 NTRN (and no constraints)
    let cfg = suite.service_config(
        vec![(
            UncheckedDenom::Native("untrn".to_string()),
            1_000_000_000_000_u128,
        )],
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
        vec![(
            UncheckedDenom::Native("untrn".to_string()),
            1_000_000_000_u128,
        )],
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

#[test]
fn forward_native_token_zero_balance() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite = ForwarderTestSuite::default();

    // Set max amount to be forwarded to 1_000 NTRN (and no constraints)
    let cfg = suite.service_config(
        vec![(
            UncheckedDenom::Native("untrn".to_string()),
            1_000_000_000_u128,
        )],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.instantiate(&cfg).unwrap();

    // Execute forward action
    let res = suite.forward(svc);
    assert!(res.is_ok());

    // Verify input account's balance: should be zero
    let input_balance = suite.query_balance(&suite.input_addr, "untrn").unwrap();
    assert_eq!(input_balance, coin(0_u128, "untrn"));

    // Verify output account's balance: should be zero
    let output_balance = suite.query_balance(&suite.output_addr, "untrn").unwrap();
    assert_eq!(output_balance, coin(0_u128, "untrn"));
}

#[test]
fn forward_cw20_full_amount() {
    // Arrange
    let mut suite = ForwarderTestSuite::default();

    // Instantiate CW20 token contract, and initialize input account with 1_000_000 MEME
    let cw20_addr = suite
        .instantiate_cw20(
            "umeme".to_string(),
            "MEME".to_string(),
            6,
            vec![Cw20Coin {
                address: suite.input_addr.to_string(),
                amount: 1_000_000_000_000_u128.into(),
            }],
        )
        .unwrap();

    let cfg = suite.service_config(
        vec![(
            UncheckedDenom::Cw20(cw20_addr.to_string()),
            1_000_000_000_000_u128,
        )],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.instantiate(&cfg).unwrap();

    // Execute forward action
    let res = suite.forward(svc);
    assert!(res.is_ok());

    // Verify input account's balance: should be zero
    let input_balance = suite
        .query_cw20_balance(&suite.input_addr, &cw20_addr)
        .unwrap();
    assert_eq!(input_balance, Uint128::zero());

    // Verify output account's balance: should be 1_000_000 MEME
    let output_balance = suite
        .query_cw20_balance(&suite.output_addr, &cw20_addr)
        .unwrap();
    assert_eq!(output_balance, Uint128::from(1_000_000_000_000_u128));
}

#[test]
fn forward_cw20_partial_amount() {
    // Arrange
    let mut suite = ForwarderTestSuite::default();

    // Instantiate CW20 token contract, and initialize input account with 1_000_000 MEME
    let cw20_addr = suite
        .instantiate_cw20(
            "umeme".to_string(),
            "MEME".to_string(),
            6,
            vec![Cw20Coin {
                address: suite.input_addr.to_string(),
                amount: 1_000_000_000_000_u128.into(),
            }],
        )
        .unwrap();

    let cfg = suite.service_config(
        vec![(
            UncheckedDenom::Cw20(cw20_addr.to_string()),
            1_000_000_000_u128,
        )],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.instantiate(&cfg).unwrap();

    // Execute forward action
    let res = suite.forward(svc);
    assert!(res.is_ok());

    // Verify input account's balance: should be 999_000 MEME
    let input_balance = suite
        .query_cw20_balance(&suite.input_addr, &cw20_addr)
        .unwrap();
    assert_eq!(input_balance, Uint128::from(999_000_000_000_u128));

    // Verify output account's balance: should be 1_000 MEME
    let output_balance = suite
        .query_cw20_balance(&suite.output_addr, &cw20_addr)
        .unwrap();
    assert_eq!(output_balance, Uint128::from(1_000_000_000_u128));
}

#[test]
fn forward_with_height_interval_constraint() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite =
        ForwarderTestSuite::with_input_balances(Some(&[(1_000_000_000_000_u128, "untrn")]));

    // Set max amount to be forwarded to 1_000 NTRN,
    // and constrain forward operation to once every 3 blocks.
    let cfg = suite.service_config(
        vec![(
            UncheckedDenom::Native("untrn".to_string()),
            1_000_000_000_u128,
        )],
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
        vec![(
            UncheckedDenom::Native("untrn".to_string()),
            1_000_000_000_u128,
        )],
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

#[test]
fn forward_multiple_tokens_continuously() {
    // Initialize input account with 2000 NTRN
    let mut suite = ForwarderTestSuite::with_input_balances(Some(&[(2_000_000_000_u128, "untrn")]));

    let owner_addr = suite.owner().clone();
    let input_addr = suite.input_addr.clone();

    // Instantiate CW20 token contract, and initialize input account with 30 MEME
    let cw20_addr = suite
        .instantiate_cw20(
            "umeme".to_string(),
            "MEME".to_string(),
            6,
            vec![
                Cw20Coin {
                    address: suite.input_addr.to_string(),
                    amount: 30_000_000_u128.into(),
                },
                // Also send some to owner account
                Cw20Coin {
                    address: suite.owner().to_string(),
                    amount: 1_000_000_000_u128.into(),
                },
            ],
        )
        .unwrap();

    // Set max amount to be forwarded to 1_000 NTRN & 10 MEME
    // and constrain forward operation to once every 2 blocks.
    let cfg = suite.service_config(
        vec![
            (
                UncheckedDenom::Native("untrn".to_string()),
                1_000_000_000_u128,
            ),
            (UncheckedDenom::Cw20(cw20_addr.to_string()), 10_000_000_u128),
        ],
        ForwardingConstraints::from(Duration::Height(2)),
    );

    // Instantiate Forwarder contract
    let svc = suite.instantiate(&cfg).unwrap();

    // BLOCK N
    // Forward successful: 1_000 NTRN & 10 MEME
    let mut res = suite.forward(svc.clone());
    assert!(res.is_ok());

    // BLOCK N+1
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+2
    suite.next_block();
    // Forward successful: 1_000 NTRN & 10 MEME
    res = suite.forward(svc.clone());
    assert!(res.is_ok());

    // BLOCK N+3
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.forward(svc.clone());
    assert!(res.is_err());

    // Transfer 6_000 NTRN to input account (should be zero at this point)
    let _ = suite.send_tokens(
        &owner_addr,
        &input_addr,
        &[coin(6_000_000_000_u128, "untrn")],
    );

    // BLOCK N+4
    suite.next_block();
    // Forward successful: 1_000 NTRN & 10 MEME
    res = suite.forward(svc.clone());
    assert!(res.is_ok());

    // BLOCK N+5
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.forward(svc.clone());
    assert!(res.is_err());

    // Transfer 40 MEME to input account (should be zero at this point)
    let _ = suite.send_cw20_tokens(&cw20_addr, &owner_addr, &input_addr, 40_000_000_u128);

    // BLOCK N+6
    suite.next_block();
    // Forward successful: 1_000 NTRN & 10 MEME
    res = suite.forward(svc.clone());
    assert!(res.is_ok());

    // BLOCK N+7
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+8
    suite.next_block();
    // Forward successful: 1_000 NTRN & 10 MEME
    res = suite.forward(svc.clone());
    assert!(res.is_ok());

    // BLOCK N+9
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+10
    suite.next_block();
    // Forward successful: 1_000 NTRN & 10 MEME
    res = suite.forward(svc.clone());
    assert!(res.is_ok());

    // Verify input account's NTRN balance: should be 2_000 NTRN
    let input_balance = suite.query_balance(&suite.input_addr, "untrn").unwrap();
    assert_eq!(input_balance, coin(2_000_000_000_u128, "untrn"));

    // Verify input account's MEME balance: should be 10 MEME
    let input_balance = suite
        .query_cw20_balance(&suite.input_addr, &cw20_addr)
        .unwrap();
    assert_eq!(input_balance, Uint128::from(10_000_000_u128));

    // Verify output account's balance: should be 6_000 NTRN
    let output_balance = suite.query_balance(&suite.output_addr, "untrn").unwrap();
    assert_eq!(output_balance, coin(6_000_000_000, "untrn"));

    // Verify output account's balance: should be 60 MEME
    let output_balance = suite
        .query_cw20_balance(&suite.output_addr, &cw20_addr)
        .unwrap();
    assert_eq!(output_balance, Uint128::from(60_000_000_u128));
}
