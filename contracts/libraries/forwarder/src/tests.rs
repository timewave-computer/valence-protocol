use crate::msg::{ActionMsgs, Config, ForwardingConstraints, LibraryConfig, QueryMsg};
use cosmwasm_std::{coin, Addr, Coin, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{error::AnyResult, App, AppResponse, ContractWrapper, Executor};
use cw_ownable::Ownership;
use cw_utils::Duration;
use getset::{Getters, Setters};
use valence_library_utils::{
    denoms::{CheckedDenom, UncheckedDenom},
    msg::{ExecuteMsg, InstantiateMsg, LibraryConfigValidation},
    testing::{LibraryTestSuite, LibraryTestSuiteBase},
};

const NTRN: &str = "untrn";

#[derive(Getters, Setters)]
struct ForwarderTestSuite {
    #[getset(get)]
    inner: LibraryTestSuiteBase,
    #[getset(get)]
    forwarder_code_id: u64,
    #[getset(get)]
    input_addr: Addr,
    #[getset(get)]
    output_addr: Addr,
    #[getset(get)]
    input_balances: Option<Vec<(u128, String)>>,
}

impl Default for ForwarderTestSuite {
    fn default() -> Self {
        Self::new(None)
    }
}

#[allow(dead_code)]
impl ForwarderTestSuite {
    pub fn new(input_balances: Option<Vec<(u128, String)>>) -> Self {
        let mut inner = LibraryTestSuiteBase::new();

        let input_addr = inner.get_contract_addr(inner.account_code_id(), "input_account");
        let output_addr = inner.api().addr_make("output_account");

        // Forwarder contract
        let forwarder_code = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );

        let forwarder_code_id = inner.app_mut().store_code(Box::new(forwarder_code));

        Self {
            inner,
            forwarder_code_id,
            input_addr,
            output_addr,
            input_balances,
        }
    }

    pub fn forwarder_init(&mut self, cfg: &LibraryConfig) -> Addr {
        let init_msg = InstantiateMsg {
            owner: self.owner().to_string(),
            processor: self.processor().to_string(),
            config: cfg.clone(),
        };
        let addr = self.contract_init(self.forwarder_code_id, "forwarder", &init_msg, &[]);

        let input_addr = self.input_addr().clone();
        if self.app_mut().contract_data(&input_addr).is_err() {
            let account_addr = self.account_init("input_account", vec![addr.to_string()]);
            assert_eq!(account_addr, input_addr);

            if let Some(balances) = self.input_balances.as_ref().cloned() {
                let amounts = balances
                    .iter()
                    .map(|(amount, denom)| coin(*amount, denom.to_string()))
                    .collect::<Vec<Coin>>();
                self.init_balance(&input_addr, amounts);
            }
        }

        addr
    }

    fn forwarder_config(
        &self,
        forwarding_configs: Vec<(UncheckedDenom, u128)>,
        forwarding_constraints: ForwardingConstraints,
    ) -> LibraryConfig {
        LibraryConfig::new(
            self.input_addr(),
            self.output_addr(),
            forwarding_configs.into_iter().map(Into::into).collect(),
            forwarding_constraints,
        )
    }

    fn execute_forward(&mut self, addr: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::<_, LibraryConfig>::ProcessAction(ActionMsgs::Forward {}),
        )
    }

    fn update_config(&mut self, addr: Addr, new_config: LibraryConfig) -> AnyResult<AppResponse> {
        let owner = self.owner().clone();
        self.app_mut().execute_contract(
            owner,
            addr,
            &ExecuteMsg::<ActionMsgs, LibraryConfig>::UpdateConfig { new_config },
            &[],
        )
    }
}

impl LibraryTestSuite<Empty, Empty> for ForwarderTestSuite {
    fn app(&self) -> &App {
        self.inner.app()
    }

    fn app_mut(&mut self) -> &mut App {
        self.inner.app_mut()
    }

    fn owner(&self) -> &Addr {
        self.inner.owner()
    }

    fn processor(&self) -> &Addr {
        self.inner.processor()
    }

    fn account_code_id(&self) -> u64 {
        self.inner.account_code_id()
    }

    fn cw20_code_id(&self) -> u64 {
        self.inner.cw20_code_id()
    }
}

#[test]
fn instantiate_with_valid_config() {
    let mut suite = ForwarderTestSuite::default();

    // Set max amount to be forwarded to 1_000_000 NTRN (and no constraints)
    let cfg = suite.forwarder_config(
        vec![(UncheckedDenom::Native(NTRN.into()), 1_000_000_000_000_u128)],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.forwarder_init(&cfg);

    // Verify owner
    let owner_res: Ownership<Addr> = suite.query_wasm(&svc, &QueryMsg::Ownership {});
    assert_eq!(owner_res.owner, Some(suite.owner().clone()));

    // Verify processor
    let processor_addr: Addr = suite.query_wasm(&svc, &QueryMsg::GetProcessor {});
    assert_eq!(processor_addr, suite.processor().clone());

    // Verify library config
    let svc_cfg: Config = suite.query_wasm(&svc, &QueryMsg::GetLibraryConfig {});
    assert_eq!(
        svc_cfg,
        Config::new(
            suite.input_addr().clone(),
            suite.output_addr().clone(),
            vec![(CheckedDenom::Native(NTRN.into()), 1_000_000_000_000_u128).into()],
            cfg.forwarding_constraints
        )
    );
}

#[test]
#[should_panic(
    expected = "Configuration error: Duplicate denom 'Native(\"untrn\")' in forwarding config."
)]
fn instantiate_fails_for_duplicate_denoms() {
    let mut suite = ForwarderTestSuite::default();

    // Set max amount to be forwarded to 1_000_000 NTRN (and no constraints)
    let cfg = suite.forwarder_config(
        vec![
            (UncheckedDenom::Native(NTRN.into()), 1_000_000_000_000_u128),
            (UncheckedDenom::Native(NTRN.into()), 1_000_000_u128),
        ],
        Default::default(),
    );

    // Instantiate Forwarder contract
    suite.forwarder_init(&cfg);
}

#[test]
#[should_panic(
    expected = "Configuration error: invalid cw20 - did not respond to `TokenInfo` query"
)]
fn instantiate_fails_for_unknown_cw20() {
    let mut suite = ForwarderTestSuite::default();
    let fake_cw20 = suite.api().addr_make("umeme");

    // Set max amount to be forwarded to 1_000_000 NTRN (and no constraints)
    let cfg = suite.forwarder_config(
        vec![(
            UncheckedDenom::Cw20(fake_cw20.to_string()),
            1_000_000_000_000_u128,
        )],
        Default::default(),
    );

    // Instantiate Forwarder contract
    suite.forwarder_init(&cfg);
}

#[test]
fn pre_validate_config_works() {
    let suite = ForwarderTestSuite::default();

    // Set max amount to be forwarded to 1_000_000 NTRN (and no constraints)
    let cfg = suite.forwarder_config(
        vec![(
            UncheckedDenom::Native("untrn".to_string()),
            1_000_000_000_000_u128,
        )],
        Default::default(),
    );

    // Pre-validate config
    cfg.pre_validate(suite.api()).unwrap();
}

#[test]
fn forward_native_token_full_amount() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite = ForwarderTestSuite::new(Some(vec![(1_000_000_000_000_u128, NTRN.into())]));

    // Set max amount to be forwarded to 1_000_000 NTRN (and no constraints)
    let cfg = suite.forwarder_config(
        vec![(UncheckedDenom::Native(NTRN.into()), 1_000_000_000_000_u128)],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.forwarder_init(&cfg);

    // Execute forward action
    suite.execute_forward(svc).unwrap();

    // Verify input account's balance: should be zero
    let input_balance = suite.query_balance(&suite.input_addr, NTRN);
    assert_eq!(input_balance, coin(0, NTRN));

    // Verify output account's balance: should be 1_000_000 NTRN
    let output_balance = suite.query_balance(&suite.output_addr, NTRN);
    assert_eq!(output_balance, coin(1_000_000_000_000, NTRN));
}

#[test]
fn forward_native_token_partial_amount() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite = ForwarderTestSuite::new(Some(vec![(1_000_000_000_000_u128, NTRN.into())]));

    // Set max amount to be forwarded to 1_000 NTRN (and no constraints)
    let cfg = suite.forwarder_config(
        vec![(UncheckedDenom::Native(NTRN.into()), 1_000_000_000_u128)],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.forwarder_init(&cfg);

    // Execute forward action
    suite.execute_forward(svc).unwrap();

    // Verify input account's balance: should be 999_000 NTR
    let input_balance = suite.query_balance(&suite.input_addr, NTRN);
    assert_eq!(input_balance, coin(999_000_000_000_u128, NTRN));

    // Verify output account's balance: should be 1_000 NTRN
    let output_balance = suite.query_balance(&suite.output_addr, NTRN);
    assert_eq!(output_balance, coin(1_000_000_000, NTRN));
}

#[test]
fn forward_native_token_zero_balance() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite = ForwarderTestSuite::default();

    // Set max amount to be forwarded to 1_000 NTRN (and no constraints)
    let cfg = suite.forwarder_config(
        vec![(UncheckedDenom::Native(NTRN.into()), 1_000_000_000_u128)],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.forwarder_init(&cfg);

    // Execute forward action
    suite.execute_forward(svc).unwrap();

    // Verify input account's balance: should be zero
    let input_balance = suite.query_balance(&suite.input_addr, NTRN);
    assert_eq!(input_balance, coin(0_u128, NTRN));

    // Verify output account's balance: should be zero
    let output_balance = suite.query_balance(&suite.output_addr, NTRN);
    assert_eq!(output_balance, coin(0_u128, NTRN));
}

#[test]
fn forward_cw20_full_amount() {
    // Arrange
    let mut suite = ForwarderTestSuite::default();

    // Instantiate CW20 token contract, and initialize input account with 1_000_000 MEME
    let cw20_addr = suite.cw20_init(
        "umeme",
        "MEME",
        6,
        vec![Cw20Coin {
            address: suite.input_addr.to_string(),
            amount: 1_000_000_000_000_u128.into(),
        }],
    );

    let cfg = suite.forwarder_config(
        vec![(
            UncheckedDenom::Cw20(cw20_addr.to_string()),
            1_000_000_000_000_u128,
        )],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.forwarder_init(&cfg);

    // Execute forward action
    suite.execute_forward(svc).unwrap();

    // Verify input account's balance: should be zero
    let input_balance = suite.cw20_query_balance(&suite.input_addr, &cw20_addr);
    assert_eq!(input_balance, Uint128::zero());

    // Verify output account's balance: should be 1_000_000 MEME
    let output_balance = suite.cw20_query_balance(&suite.output_addr, &cw20_addr);
    assert_eq!(output_balance, Uint128::from(1_000_000_000_000_u128));
}

#[test]
fn forward_cw20_partial_amount() {
    // Arrange
    let mut suite = ForwarderTestSuite::default();

    // Instantiate CW20 token contract, and initialize input account with 1_000_000 MEME
    let cw20_addr = suite.cw20_init(
        "umeme",
        "MEME",
        6,
        vec![Cw20Coin {
            address: suite.input_addr.to_string(),
            amount: 1_000_000_000_000_u128.into(),
        }],
    );

    let cfg = suite.forwarder_config(
        vec![(
            UncheckedDenom::Cw20(cw20_addr.to_string()),
            1_000_000_000_u128,
        )],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.forwarder_init(&cfg);

    // Execute forward action
    suite.execute_forward(svc).unwrap();

    // Verify input account's balance: should be 999_000 MEME
    let input_balance = suite.cw20_query_balance(&suite.input_addr, &cw20_addr);
    assert_eq!(input_balance, Uint128::from(999_000_000_000_u128));

    // Verify output account's balance: should be 1_000 MEME
    let output_balance = suite.cw20_query_balance(&suite.output_addr, &cw20_addr);
    assert_eq!(output_balance, Uint128::from(1_000_000_000_u128));
}

#[test]
fn forward_with_height_interval_constraint() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite = ForwarderTestSuite::new(Some(vec![(1_000_000_000_000_u128, NTRN.into())]));

    // Set max amount to be forwarded to 1_000 NTRN,
    // and constrain forward operation to once every 3 blocks.
    let cfg = suite.forwarder_config(
        vec![(UncheckedDenom::Native(NTRN.into()), 1_000_000_000_u128)],
        ForwardingConstraints::new(Duration::Height(3).into()),
    );

    // Instantiate Forwarder contract
    let svc = suite.forwarder_init(&cfg);

    // BLOCK N
    // Execute forward action shoud succeed
    suite.execute_forward(svc.clone()).unwrap();

    // BLOCK N+1
    suite.next_block();
    // Execute forward action shoud fail
    let mut res = suite.execute_forward(svc.clone());
    assert!(res.is_err());

    assert_eq!(
        res.unwrap_err().root_cause().to_string(),
        "Execution error: Forwarding constraint not met."
    );

    // BLOCK N+2
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.execute_forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+3
    suite.next_block();
    // Execute forward action shoud succeed
    suite.execute_forward(svc.clone()).unwrap();

    // Verify input account's balance: should be 998_000 NTR because of 2 successful forwards
    let input_balance = suite.query_balance(&suite.input_addr, NTRN);
    assert_eq!(input_balance, coin(998_000_000_000_u128, NTRN));

    // Verify output account's balance: should be 2_000 NTRN because of 2 successful forwards
    let output_balance = suite.query_balance(&suite.output_addr, NTRN);
    assert_eq!(output_balance, coin(2_000_000_000, NTRN));
}

#[test]
fn forward_with_time_interval_constraint() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite = ForwarderTestSuite::new(Some(vec![(1_000_000_000_000_u128, NTRN.into())]));

    // Set max amount to be forwarded to 1_000 NTRN,
    // and constrain forward operation to once every 20 seconds.
    let cfg = suite.forwarder_config(
        vec![(UncheckedDenom::Native(NTRN.into()), 1_000_000_000_u128)],
        ForwardingConstraints::new(Duration::Time(20).into()),
    );

    // Instantiate Forwarder contract
    let svc = suite.forwarder_init(&cfg);

    // NOTE: This test verifies the time interval constraint by simulating the passage of time.
    // => cw-multi-test 'next_block' function uses 5 seconds per block.
    // Therefore, 4 blocks are required to simulate 20 seconds.
    // While unrealistic, this is a simple way to test the time interval constraint.

    // BLOCK N
    // Execute forward action shoud succeed
    suite.execute_forward(svc.clone()).unwrap();

    // BLOCK N+1
    suite.next_block();
    // Execute forward action shoud fail
    let mut res = suite.execute_forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+2
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.execute_forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+3
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.execute_forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+4
    suite.next_block();
    // Execute forward action shoud succeed
    suite.execute_forward(svc.clone()).unwrap();

    // Verify input account's balance: should be 998_000 NTR because of 2 successful forwards
    let input_balance = suite.query_balance(&suite.input_addr, NTRN);
    assert_eq!(input_balance, coin(998_000_000_000_u128, NTRN));

    // Verify output account's balance: should be 2_000 NTRN because of 2 successful forwards
    let output_balance = suite.query_balance(&suite.output_addr, NTRN);
    assert_eq!(output_balance, coin(2_000_000_000, NTRN));
}

#[test]
fn forward_multiple_tokens_continuously() {
    // Initialize input account with 2000 NTRN
    let mut suite = ForwarderTestSuite::new(Some(vec![(2_000_000_000_u128, NTRN.into())]));

    let owner_addr = suite.owner().clone();
    let input_addr = suite.input_addr.clone();

    // Instantiate CW20 token contract, and initialize input account with 30 MEME
    let cw20_addr = suite.cw20_init(
        "umeme",
        "MEME",
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
    );

    // Set max amount to be forwarded to 1_000 NTRN & 10 MEME
    // and constrain forward operation to once every 2 blocks.
    let cfg = suite.forwarder_config(
        vec![
            (UncheckedDenom::Native(NTRN.into()), 1_000_000_000_u128),
            (UncheckedDenom::Cw20(cw20_addr.to_string()), 10_000_000_u128),
        ],
        ForwardingConstraints::new(Duration::Height(2).into()),
    );

    // Instantiate Forwarder contract
    let svc = suite.forwarder_init(&cfg);

    // Initialize owner account with 1_000_000 NTRN
    suite.init_balance(&owner_addr, vec![coin(1_000_000_000_000_u128, NTRN)]);

    // BLOCK N
    // Forward successful: 1_000 NTRN & 10 MEME
    suite.execute_forward(svc.clone()).unwrap();

    // BLOCK N+1
    suite.next_block();
    // Execute forward action shoud fail
    let mut res = suite.execute_forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+2
    suite.next_block();
    // Forward successful: 1_000 NTRN & 10 MEME
    suite.execute_forward(svc.clone()).unwrap();

    // BLOCK N+3
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.execute_forward(svc.clone());
    assert!(res.is_err());

    // Transfer 6_000 NTRN to input account (should be zero at this point)
    let _ = suite.send_tokens(&owner_addr, &input_addr, &[coin(6_000_000_000_u128, NTRN)]);

    // BLOCK N+4
    suite.next_block();
    // Forward successful: 1_000 NTRN & 10 MEME
    suite.execute_forward(svc.clone()).unwrap();

    // BLOCK N+5
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.execute_forward(svc.clone());
    assert!(res.is_err());

    // Transfer 40 MEME to input account (should be zero at this point)
    let _ = suite.cw20_send_tokens(&cw20_addr, &owner_addr, &input_addr, 40_000_000_u128);

    // BLOCK N+6
    suite.next_block();
    // Forward successful: 1_000 NTRN & 10 MEME
    suite.execute_forward(svc.clone()).unwrap();

    // BLOCK N+7
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.execute_forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+8
    suite.next_block();
    // Forward successful: 1_000 NTRN & 10 MEME
    suite.execute_forward(svc.clone()).unwrap();

    // BLOCK N+9
    suite.next_block();
    // Execute forward action shoud fail
    res = suite.execute_forward(svc.clone());
    assert!(res.is_err());

    // BLOCK N+10
    suite.next_block();
    // Forward successful: 1_000 NTRN & 10 MEME
    suite.execute_forward(svc.clone()).unwrap();

    // Verify input account's NTRN balance: should be 2_000 NTRN
    let input_balance = suite.query_balance(&suite.input_addr, NTRN);
    assert_eq!(input_balance, coin(2_000_000_000_u128, NTRN));

    // Verify input account's MEME balance: should be 10 MEME
    let input_balance = suite.cw20_query_balance(&suite.input_addr, &cw20_addr);
    assert_eq!(input_balance, Uint128::from(10_000_000_u128));

    // Verify output account's balance: should be 6_000 NTRN
    let output_balance = suite.query_balance(&suite.output_addr, NTRN);
    assert_eq!(output_balance, coin(6_000_000_000, NTRN));

    // Verify output account's balance: should be 60 MEME
    let output_balance = suite.cw20_query_balance(&suite.output_addr, &cw20_addr);
    assert_eq!(output_balance, Uint128::from(60_000_000_u128));
}

#[test]
fn update_config() {
    // Initialize input account with 1_000_000 NTRN
    let mut suite = ForwarderTestSuite::new(Some(vec![(1_000_000_000_000_u128, NTRN.into())]));

    // Set max amount to be forwarded to 1_000 NTRN (and no constraints)
    let cfg = suite.forwarder_config(
        vec![(UncheckedDenom::Native(NTRN.into()), 1_000_000_000_u128)],
        Default::default(),
    );

    // Instantiate Forwarder contract
    let svc = suite.forwarder_init(&cfg);

    // Update config to forward 2_000 NTRN, add constraint,
    // and swap input and output addresses.
    let mut new_config = suite.forwarder_config(
        vec![(UncheckedDenom::Native(NTRN.into()), 2_000_000_000_u128)],
        ForwardingConstraints::new(Duration::Height(3).into()),
    );
    new_config.input_addr = suite.output_addr().into();
    new_config.output_addr = suite.input_addr().into();

    // Execute update config action
    suite.update_config(svc.clone(), new_config).unwrap();

    // Verify library config
    let cfg = suite.query_wasm::<_, Config>(&svc, &QueryMsg::GetLibraryConfig {});
    assert_eq!(
        cfg,
        Config::new(
            suite.output_addr,
            suite.input_addr,
            vec![(CheckedDenom::Native(NTRN.into()), 2_000_000_000_u128).into()],
            ForwardingConstraints::new(Duration::Height(3).into())
        )
    );
}
