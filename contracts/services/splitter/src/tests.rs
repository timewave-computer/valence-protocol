use crate::msg::{ActionMsgs, Config, QueryMsg, ServiceConfig, SplitConfig, UncheckedSplitConfig};
use cosmwasm_std::{coin, Addr, Coin};
use cw20::Cw20Coin;
use cw_multi_test::{error::AnyResult, App, AppResponse, ContractWrapper, Executor};
use cw_ownable::Ownership;
use getset::{Getters, Setters};
use valence_service_utils::{
    denoms::{CheckedDenom, UncheckedDenom},
    msg::ExecuteMsg,
    msg::InstantiateMsg,
    testing::{ServiceTestSuite, ServiceTestSuiteBase},
};

const NTRN: &str = "untrn";
const STARS: &str = "ustars";
const MEME: &str = "umeme";
const CATZ: &str = "ucatz";
const ZERO: u128 = 0u128;
const ONE_MILLION: u128 = 1_000_000_000_000_u128;
const TEN_MILLION: u128 = 10_000_000_000_000_u128;

#[derive(Getters, Setters)]
struct SplitterTestSuite {
    #[getset(get)]
    inner: ServiceTestSuiteBase,
    #[getset(get)]
    splitter_code_id: u64,
    #[getset(get)]
    input_addr: Addr,
    #[getset(get)]
    input_balances: Option<Vec<(u128, String)>>,
}

impl Default for SplitterTestSuite {
    fn default() -> Self {
        Self::new(None)
    }
}

#[allow(dead_code)]
impl SplitterTestSuite {
    pub fn new(input_balances: Option<Vec<(u128, String)>>) -> Self {
        let mut inner = ServiceTestSuiteBase::new();

        let input_addr = inner.get_contract_addr(inner.account_code_id(), "input_account");

        // Forwarder contract
        let splitter_code = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );

        let splitter_code_id = inner.app_mut().store_code(Box::new(splitter_code));

        Self {
            inner,
            splitter_code_id,
            input_addr,
            input_balances,
        }
    }

    pub fn splitter_init(&mut self, cfg: &ServiceConfig) -> Addr {
        let init_msg = InstantiateMsg {
            owner: self.owner().to_string(),
            processor: self.processor().to_string(),
            config: cfg.clone(),
        };
        let addr = self.contract_init(self.splitter_code_id, "splitter", &init_msg, &[]);

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

    fn splitter_config(
        &self,
        splits: Vec<UncheckedSplitConfig>,
        base_denom: UncheckedDenom,
    ) -> ServiceConfig {
        ServiceConfig::new(self.input_addr.to_string(), splits, base_denom)
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

    fn execute_split(&mut self, addr: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::<_, ServiceConfig>::ProcessAction(ActionMsgs::Split {}),
        )
    }

    fn update_config(&mut self, addr: Addr, new_config: ServiceConfig) -> AnyResult<AppResponse> {
        let owner = self.owner().clone();
        self.app_mut().execute_contract(
            owner,
            addr,
            &ExecuteMsg::<ActionMsgs, ServiceConfig>::UpdateConfig { new_config },
            &[],
        )
    }
}

impl ServiceTestSuite for SplitterTestSuite {
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
fn instantiate_with_valid_single_split() {
    let mut suite = SplitterTestSuite::default();

    let output_addr = suite.api().addr_make("output_account");

    let cfg = suite.splitter_config(
        vec![UncheckedSplitConfig::with_native_amount(
            ONE_MILLION,
            NTRN,
            &output_addr,
        )],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    let svc = suite.splitter_init(&cfg);

    // Verify owner
    let owner_res: Ownership<Addr> = suite.query_wasm(&svc, &QueryMsg::Ownership {});
    assert_eq!(owner_res.owner, Some(suite.owner().clone()));

    // Verify processor
    let processor_addr: Addr = suite.query_wasm(&svc, &QueryMsg::GetProcessor {});
    assert_eq!(processor_addr, suite.processor().clone());

    // Verify service config
    let svc_cfg: Config = suite.query_wasm(&svc, &QueryMsg::GetServiceConfig {});
    assert_eq!(
        svc_cfg,
        Config::new(
            suite.input_addr().clone(),
            vec![SplitConfig::new(
                CheckedDenom::Native(NTRN.into()),
                output_addr,
                Some(ONE_MILLION.into()),
                None,
                None
            )],
            CheckedDenom::Native(NTRN.into())
        )
    );
}

#[test]
#[should_panic(
    expected = "Configuration error: Duplicate split 'Native(\"untrn\")|cosmwasm1ea6n0jqm0hj663khx7a5xklsmjgrazjp9vjeewejn84sanr0wgxq2p70xl' in split config."
)]
fn instantiate_fails_for_duplicate_split() {
    let mut suite = SplitterTestSuite::default();

    let output_addr = suite.api().addr_make("output_account");

    // Configure splitter with duplicate split
    let split_cfg = UncheckedSplitConfig::with_native_amount(ONE_MILLION, NTRN, &output_addr);
    let cfg = suite.splitter_config(
        vec![split_cfg.clone(), split_cfg],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    suite.splitter_init(&cfg);
}

// More tests for invalid configurations
// TODO: Add more tests for invalid configurations
//______________________________________________________________________________

// Native & CW20 token amount splits

#[test]
fn split_native_single_token_amount_single_output() {
    let mut suite = SplitterTestSuite::new(Some(vec![(ONE_MILLION, NTRN.into())]));

    let output_addr = suite.api().addr_make("output_account");

    let cfg = suite.splitter_config(
        vec![UncheckedSplitConfig::with_native_amount(
            ONE_MILLION,
            NTRN,
            &output_addr,
        )],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    let svc = suite.splitter_init(&cfg);

    // Execute split
    suite.execute_split(svc).unwrap();

    // Verify input account's balance: should be zero
    suite.assert_balance(suite.input_addr(), ZERO, NTRN);

    // Verify output account's balance: should be 1_000_000 NTRN
    suite.assert_balance(&output_addr, ONE_MILLION, NTRN);
}

#[test]
fn split_native_single_token_amount_two_outputs() {
    let mut suite = SplitterTestSuite::new(Some(vec![(ONE_MILLION, NTRN.into())]));

    let output1_addr = suite.api().addr_make("output_account_1");
    let output2_addr = suite.api().addr_make("output_account_2");

    let cfg = suite.splitter_config(
        vec![
            UncheckedSplitConfig::with_native_amount(600_000_000_000_u128, NTRN, &output1_addr),
            UncheckedSplitConfig::with_native_amount(400_000_000_000_u128, NTRN, &output2_addr),
        ],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    let svc = suite.splitter_init(&cfg);

    // Execute split
    suite.execute_split(svc).unwrap();

    // Verify input account's balance: should be zero
    suite.assert_balance(suite.input_addr(), ZERO, NTRN);

    // Verify output account 1's balance: should be 600_000 NTRN
    suite.assert_balance(&output1_addr, 600_000_000_000_u128, NTRN);

    // Verify output account 2's balance: should be 400_000 NTRN
    suite.assert_balance(&output2_addr, 400_000_000_000_u128, NTRN);
}

#[test]
fn split_native_two_token_amounts_two_outputs() {
    let mut suite = SplitterTestSuite::new(Some(vec![
        (ONE_MILLION, NTRN.into()),
        (TEN_MILLION, STARS.into()),
    ]));

    let output1_addr = suite.api().addr_make("output_account_1");
    let output2_addr = suite.api().addr_make("output_account_2");

    let cfg = suite.splitter_config(
        vec![
            UncheckedSplitConfig::with_native_amount(ONE_MILLION, NTRN, &output1_addr),
            UncheckedSplitConfig::with_native_amount(TEN_MILLION, STARS, &output2_addr),
        ],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    let svc = suite.splitter_init(&cfg);

    // Assert initial balances
    suite.assert_balance(suite.input_addr(), ONE_MILLION, NTRN);
    suite.assert_balance(suite.input_addr(), TEN_MILLION, STARS);

    // Execute split
    suite.execute_split(svc).unwrap();

    // Verify input account's balance: should be zero
    suite.assert_balance(suite.input_addr(), ZERO, NTRN);
    suite.assert_balance(suite.input_addr(), ZERO, STARS);

    // Verify output account 1's balance: should be 1_000_000 NTRN
    suite.assert_balance(&output1_addr, ONE_MILLION, NTRN);

    // Verify output account 2's balance: should be 10_000_000 STARS
    suite.assert_balance(&output2_addr, TEN_MILLION, STARS);
}

#[test]
fn split_cw20_single_token_amount_single_output() {
    let mut suite = SplitterTestSuite::default();

    let cw20_addr =
        suite.cw20_token_init(MEME, "MEME", ONE_MILLION, suite.input_addr().to_string());
    let output_addr = suite.api().addr_make("output_account");

    let cfg = suite.splitter_config(
        vec![UncheckedSplitConfig::with_cw20_amount(
            ONE_MILLION,
            &cw20_addr,
            &output_addr,
        )],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    let svc = suite.splitter_init(&cfg);

    // Execute split
    suite.execute_split(svc).unwrap();

    // Verify input account's balance: should be zero
    suite.assert_cw20_balance(suite.input_addr(), ZERO, &cw20_addr);

    // Verify output account's balance: should be 1_000_000 MEME
    suite.assert_cw20_balance(&output_addr, ONE_MILLION, &cw20_addr);
}

#[test]
fn split_cw20_single_token_amount_two_outputs() {
    let mut suite = SplitterTestSuite::default();

    let cw20_addr =
        suite.cw20_token_init(MEME, "MEME", ONE_MILLION, suite.input_addr().to_string());

    let output1_addr = suite.api().addr_make("output_account_1");
    let output2_addr = suite.api().addr_make("output_account_2");

    let cfg = suite.splitter_config(
        vec![
            UncheckedSplitConfig::with_cw20_amount(600_000_000_000_u128, &cw20_addr, &output1_addr),
            UncheckedSplitConfig::with_cw20_amount(400_000_000_000_u128, &cw20_addr, &output2_addr),
        ],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    let svc = suite.splitter_init(&cfg);

    // Execute split
    suite.execute_split(svc).unwrap();

    // Verify input account's balance: should be zero
    suite.assert_cw20_balance(suite.input_addr(), ZERO, &cw20_addr);

    // Verify output account 1's balance: should be 600_000 MEME
    suite.assert_cw20_balance(&output1_addr, 600_000_000_000_u128, &cw20_addr);

    // Verify output account 2's balance: should be 400_000 MEME
    suite.assert_cw20_balance(&output2_addr, 400_000_000_000_u128, &cw20_addr);
}

#[test]
fn split_cw20_two_token_amounts_two_outputs() {
    let mut suite = SplitterTestSuite::default();

    let cw20_1_addr =
        suite.cw20_token_init(MEME, "MEME", ONE_MILLION, suite.input_addr().to_string());
    let cw20_2_addr =
        suite.cw20_token_init(CATZ, "CATZ", TEN_MILLION, suite.input_addr().to_string());

    let output1_addr = suite.api().addr_make("output_account_1");
    let output2_addr = suite.api().addr_make("output_account_2");

    let cfg = suite.splitter_config(
        vec![
            UncheckedSplitConfig::with_cw20_amount(ONE_MILLION, &cw20_1_addr, &output1_addr),
            UncheckedSplitConfig::with_cw20_amount(TEN_MILLION, &cw20_2_addr, &output2_addr),
        ],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    let svc = suite.splitter_init(&cfg);

    // Assert initial balances
    suite.assert_cw20_balance(suite.input_addr(), ONE_MILLION, &cw20_1_addr);
    suite.assert_cw20_balance(suite.input_addr(), TEN_MILLION, &cw20_2_addr);

    // Execute split
    suite.execute_split(svc).unwrap();

    // Verify input account's balance: should be zero
    suite.assert_cw20_balance(suite.input_addr(), ZERO, &cw20_1_addr);
    suite.assert_cw20_balance(suite.input_addr(), ZERO, &cw20_2_addr);

    // Verify output account 1's balance: should be 1_000_000 MEME
    suite.assert_cw20_balance(&output1_addr, ONE_MILLION, &cw20_1_addr);

    // Verify output account 2's balance: should be 10_000_000 CATZ
    suite.assert_cw20_balance(&output2_addr, TEN_MILLION, &cw20_2_addr);
}

#[test]
fn split_mix_two_token_amounts_two_outputs() {
    let mut suite = SplitterTestSuite::new(Some(vec![(ONE_MILLION, NTRN.into())]));

    let cw20_addr =
        suite.cw20_token_init(CATZ, "CATZ", TEN_MILLION, suite.input_addr().to_string());

    let output1_addr = suite.api().addr_make("output_account_1");
    let output2_addr = suite.api().addr_make("output_account_2");

    let cfg = suite.splitter_config(
        vec![
            UncheckedSplitConfig::with_native_amount(ONE_MILLION, NTRN, &output1_addr),
            UncheckedSplitConfig::with_cw20_amount(TEN_MILLION, &cw20_addr, &output2_addr),
        ],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    let svc = suite.splitter_init(&cfg);

    // Assert initial balances
    suite.assert_balance(suite.input_addr(), ONE_MILLION, NTRN);
    suite.assert_cw20_balance(suite.input_addr(), TEN_MILLION, &cw20_addr);

    // Execute split
    suite.execute_split(svc).unwrap();

    // Verify input account's balance: should be zero
    suite.assert_balance(suite.input_addr(), ZERO, NTRN);
    suite.assert_cw20_balance(suite.input_addr(), ZERO, &cw20_addr);

    // Verify output account 1's balance: should be 1_000_000 MEME
    suite.assert_balance(&output1_addr, ONE_MILLION, NTRN);

    // Verify output account 2's balance: should be 10_000_000 CATZ
    suite.assert_cw20_balance(&output2_addr, TEN_MILLION, &cw20_addr);
}

// Insufficient balance tests

#[test]
#[should_panic(
    expected = "Execution error: Insufficient balance for denom 'Native(\"untrn\")' in split config (required: 10000000000000, available: 1000000000000)."
)]
fn split_native_single_token_amount_fails_for_insufficient_balance() {
    let mut suite = SplitterTestSuite::new(Some(vec![(ONE_MILLION, NTRN.into())]));

    let output_addr = suite.api().addr_make("output_account");

    let cfg = suite.splitter_config(
        vec![UncheckedSplitConfig::with_native_amount(
            TEN_MILLION,
            NTRN,
            &output_addr,
        )],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    let svc = suite.splitter_init(&cfg);

    // Execute split
    suite.execute_split(svc).unwrap();
}

#[test]
#[should_panic(
    expected = "Execution error: Insufficient balance for denom 'Cw20(Addr(\"cosmwasm1uzyszmsnca8euusre35wuqj4el3hyj8jty84kwln7du5stwwxyns2z5hxp\"))' in split config (required: 10000000000000, available: 1000000000000)."
)]
fn split_cw20_single_token_amount_fails_for_insufficient_balance() {
    let mut suite = SplitterTestSuite::default();

    let cw20_addr =
        suite.cw20_token_init(MEME, "MEME", ONE_MILLION, suite.input_addr().to_string());
    let output_addr = suite.api().addr_make("output_account");

    let cfg = suite.splitter_config(
        vec![UncheckedSplitConfig::with_cw20_amount(
            TEN_MILLION,
            &cw20_addr,
            &output_addr,
        )],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    let svc = suite.splitter_init(&cfg);

    // Execute split
    suite.execute_split(svc).unwrap();
}

#[test]
#[should_panic(
    expected = "Execution error: Insufficient balance for denom 'Native(\"untrn\")' in split config (required: 1200000000000, available: 1000000000000)."
)]
fn split_native_single_token_amount_two_outputs_fails_for_insufficient_balance() {
    let mut suite = SplitterTestSuite::new(Some(vec![(ONE_MILLION, NTRN.into())]));

    let output1_addr = suite.api().addr_make("output_account_1");
    let output2_addr = suite.api().addr_make("output_account_2");

    // Amount per individual output does not exceed the input account's balance,
    // but the total amount for that denom does.
    let cfg = suite.splitter_config(
        vec![
            UncheckedSplitConfig::with_native_amount(600_000_000_000_u128, NTRN, &output1_addr),
            UncheckedSplitConfig::with_native_amount(600_000_000_000_u128, NTRN, &output2_addr),
        ],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    let svc = suite.splitter_init(&cfg);

    // Execute split => should fail: combined split amounts exceeds input account's balance
    suite.execute_split(svc).unwrap();
}

// Native & CW20 token ratio splits
