use std::collections::HashMap;

use crate::msg::{
    Config, FunctionMsgs, LibraryConfig, QueryMsg, SplitAmount, SplitConfig, UncheckedSplitConfig,
};
use cosmwasm_std::{Addr, Decimal, Empty};
use cw20::Cw20Coin;
use cw_multi_test::{error::AnyResult, App, AppResponse, ContractWrapper, Executor};
use cw_ownable::Ownership;
use getset::{Getters, Setters};
use valence_dynamic_ratio_query_provider::msg::DenomSplitMap;
use valence_library_utils::{
    denoms::{CheckedDenom, UncheckedDenom},
    msg::ExecuteMsg,
    msg::InstantiateMsg,
    testing::{LibraryTestSuite, LibraryTestSuiteBase},
};

const NTRN: &str = "untrn";
const STARS: &str = "ustars";
const MEME: &str = "umeme";
const CATZ: &str = "ucatz";
const ZERO: u128 = 0u128;
const ONE_MILLION: u128 = 1_000_000_000_000_u128;
const TEN_MILLION: u128 = 10_000_000_000_000_u128;
const HUNDRED_MILLION: u128 = 100_000_000_000_000_u128;
const HALF_MILLION: u128 = ONE_MILLION / 2;
const FIVE_MILLION: u128 = TEN_MILLION / 2;

#[derive(Getters, Setters)]
struct ReverseSplitterTestSuite {
    #[getset(get)]
    inner: LibraryTestSuiteBase,
    #[getset(get)]
    reverse_splitter_code_id: u64,
    #[getset(get)]
    dyn_ratio_code_id: u64,
    #[getset(get)]
    output_addr: Addr,
}

impl Default for ReverseSplitterTestSuite {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl ReverseSplitterTestSuite {
    pub fn new() -> Self {
        let mut inner = LibraryTestSuiteBase::new();

        let output_addr = inner.app().api().addr_make("output_account");

        // Forwarder contract
        let reverse_splitter_code = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );

        let reverse_splitter_code_id = inner.app_mut().store_code(Box::new(reverse_splitter_code));

        let dyn_ratio_code = ContractWrapper::new(
            valence_dynamic_ratio_query_provider::contract::execute,
            valence_dynamic_ratio_query_provider::contract::instantiate,
            valence_dynamic_ratio_query_provider::contract::query,
        );

        let dyn_ratio_code_id = inner.app_mut().store_code(Box::new(dyn_ratio_code));

        Self {
            inner,
            reverse_splitter_code_id,
            dyn_ratio_code_id,
            output_addr,
        }
    }

    pub fn reverse_splitter_init(&mut self, cfg: &LibraryConfig) -> Addr {
        let init_msg = InstantiateMsg {
            owner: self.owner().to_string(),
            processor: self.processor().to_string(),
            config: cfg.clone(),
        };
        let addr = self.contract_init(self.reverse_splitter_code_id, "splitter", &init_msg, &[]);

        cfg.splits.iter().for_each(|split| {
            self.account_approve_library(
                split.account.to_addr(self.api()).unwrap(),
                addr.to_string(),
            )
            .unwrap();
        });

        addr
    }

    pub fn dyn_ratio_contract_init(&mut self, denom: &str, receiver: &str, ratio: Decimal) -> Addr {
        let mut denom_split = HashMap::new();
        denom_split.insert(receiver.to_string(), ratio);

        let mut denom_split_cfg = HashMap::new();
        denom_split_cfg.insert(denom.to_string(), denom_split);

        let init_msg = valence_dynamic_ratio_query_provider::msg::InstantiateMsg {
            admin: self.inner.owner().to_string(),
            split_cfg: DenomSplitMap {
                split_cfg: denom_split_cfg,
            },
        };
        self.contract_init(self.dyn_ratio_code_id, "dynamic_ratio", &init_msg, &[])
    }

    fn reverse_splitter_config(
        &self,
        splits: Vec<UncheckedSplitConfig>,
        base_denom: UncheckedDenom,
    ) -> LibraryConfig {
        LibraryConfig::new(self.output_addr(), splits, base_denom)
    }

    fn cw20_token_init(
        &mut self,
        name: &str,
        symbol: &str,
        initial_balances: Vec<(u128, String)>,
    ) -> Addr {
        self.cw20_init(
            name,
            symbol,
            6,
            initial_balances
                .into_iter()
                .map(|(amount, address)| Cw20Coin {
                    address,
                    amount: amount.into(),
                })
                .collect(),
        )
    }

    fn execute_split(&mut self, addr: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::<_, LibraryConfig>::ProcessFunction(FunctionMsgs::Split {}),
        )
    }

    fn update_config(&mut self, addr: Addr, new_config: LibraryConfig) -> AnyResult<AppResponse> {
        let owner = self.owner().clone();
        self.app_mut().execute_contract(
            owner,
            addr,
            &ExecuteMsg::<FunctionMsgs, LibraryConfig>::UpdateConfig { new_config },
            &[],
        )
    }
}

impl LibraryTestSuite<Empty, Empty> for ReverseSplitterTestSuite {
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
    let mut suite = ReverseSplitterTestSuite::default();

    let input_addr = suite.account_init("input_account", vec![]);

    let cfg = suite.reverse_splitter_config(
        vec![UncheckedSplitConfig::with_native_amount(
            ONE_MILLION,
            NTRN,
            &input_addr,
        )],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Verify owner
    let owner_res: Ownership<Addr> = suite.query_wasm(&lib, &QueryMsg::Ownership {});
    assert_eq!(owner_res.owner, Some(suite.owner().clone()));

    // Verify processor
    let processor_addr: Addr = suite.query_wasm(&lib, &QueryMsg::GetProcessor {});
    assert_eq!(processor_addr, suite.processor().clone());

    // Verify library config
    let lib_cfg: Config = suite.query_wasm(&lib, &QueryMsg::GetLibraryConfig {});
    assert_eq!(
        lib_cfg,
        Config::new(
            suite.output_addr().clone(),
            vec![SplitConfig::new(
                CheckedDenom::Native(NTRN.into()),
                input_addr,
                SplitAmount::FixedAmount(ONE_MILLION.into()),
                None,
            )],
            CheckedDenom::Native(NTRN.into())
        )
    );
}

#[test]
#[should_panic(expected = "Configuration error: No split configuration provided.")]
fn instantiate_fails_for_no_split_config() {
    let mut suite = ReverseSplitterTestSuite::default();

    // Configure reverse splitter with no split config
    let cfg = suite.reverse_splitter_config(vec![], UncheckedDenom::Native(NTRN.into()));

    // Instantiate Reverse Splitter contract
    suite.reverse_splitter_init(&cfg);
}

#[test]
#[should_panic(expected = "Configuration error: Invalid split config: amount cannot be zero.")]
fn instantiate_fails_for_zero_amount() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input_addr = suite.account_init("input_account", vec![]);

    // Configure reverse splitter with invalid split config
    let split_cfg = UncheckedSplitConfig::with_native_amount(ZERO, NTRN, &input_addr);
    let cfg = suite.reverse_splitter_config(vec![split_cfg], UncheckedDenom::Native(NTRN.into()));

    // Instantiate Reverse Splitter contract
    suite.reverse_splitter_init(&cfg);
}

#[test]
#[should_panic(expected = "Configuration error: Invalid split config: ratio cannot be zero.")]
fn instantiate_fails_for_zero_ratio() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input_addr = suite.account_init("input_account", vec![]);

    // Configure reverse splitter with invalid split config
    let split_cfg = UncheckedSplitConfig::with_native_ratio(Decimal::zero(), NTRN, &input_addr);
    let cfg = suite.reverse_splitter_config(vec![split_cfg], UncheckedDenom::Native(NTRN.into()));

    // Instantiate Reverse Splitter contract
    suite.reverse_splitter_init(&cfg);
}

#[test]
#[should_panic(
    expected = "Configuration error: Duplicate split 'Native(\"untrn\")|Addr(\"cosmwasm1xj6u4ccauyhvylgtj82x2qqc34lk3xuzw4mujevzqyr4gj7gqsjsv4856c\")' in split config."
)]
fn instantiate_fails_for_duplicate_split() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input_addr = suite.account_init("input_account", vec![]);

    // Configure splitter with duplicate split
    let split_cfg = UncheckedSplitConfig::with_native_amount(ONE_MILLION, NTRN, &input_addr);
    let cfg = suite.reverse_splitter_config(
        vec![split_cfg.clone(), split_cfg],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Reverse Splitter contract
    suite.reverse_splitter_init(&cfg);
}

#[test]
#[should_panic(expected = "Configuration error: No split configuration provided.")]
fn update_config_validates_config() {
    let mut suite = ReverseSplitterTestSuite::default();

    // Initialize input account with 1_000_000 NTRN
    let input_addr =
        suite.account_init_with_balances("input_account", vec![(ONE_MILLION, NTRN.into())]);

    let mut cfg = suite.reverse_splitter_config(
        vec![UncheckedSplitConfig::with_native_amount(
            ONE_MILLION,
            NTRN,
            &input_addr,
        )],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Update config to clear all split configs
    cfg.splits.clear();

    // Execute update config action
    suite.update_config(lib.clone(), cfg).unwrap();
}

#[test]
fn update_config_with_valid_config() {
    let mut suite = ReverseSplitterTestSuite::default();

    // Initialize input account with 1_000_000 NTRN
    let input_addr =
        suite.account_init_with_balances("input_account", vec![(ONE_MILLION, NTRN.into())]);

    let mut cfg = suite.reverse_splitter_config(
        vec![UncheckedSplitConfig::with_native_amount(
            ONE_MILLION,
            NTRN,
            &input_addr,
        )],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Update config to a split config for STARS based on ratio,
    // and swap input and output addresses.
    cfg.splits.push(UncheckedSplitConfig::with_native_ratio(
        Decimal::percent(10u64),
        STARS,
        &suite.output_addr,
    ));
    cfg.output_addr = (&input_addr).into();

    // Execute update config action
    suite.update_config(lib.clone(), cfg).unwrap();

    // Verify library config
    let lib_cfg: Config = suite.query_wasm(&lib, &QueryMsg::GetLibraryConfig {});
    assert_eq!(
        lib_cfg,
        Config::new(
            input_addr.clone(),
            vec![
                SplitConfig::new(
                    CheckedDenom::Native(NTRN.into()),
                    input_addr,
                    SplitAmount::FixedAmount(ONE_MILLION.into()),
                    None
                ),
                SplitConfig::new(
                    CheckedDenom::Native(STARS.into()),
                    suite.output_addr().clone(),
                    SplitAmount::FixedRatio(Decimal::percent(10u64)),
                    None
                )
            ],
            CheckedDenom::Native(NTRN.into())
        )
    );
}

// Native & CW20 token amount splits

#[test]
fn split_native_single_token_amount_single_input() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input_addr =
        suite.account_init_with_balances("input_account", vec![(ONE_MILLION, NTRN.into())]);

    let cfg = suite.reverse_splitter_config(
        vec![UncheckedSplitConfig::with_native_amount(
            ONE_MILLION,
            NTRN,
            &input_addr,
        )],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Execute split
    suite.execute_split(lib).unwrap();

    // Verify input account's balance: should be zero
    suite.assert_balance(&input_addr, ZERO, NTRN);

    // Verify output account's balance: should be 1_000_000 NTRN
    suite.assert_balance(suite.output_addr(), ONE_MILLION, NTRN);
}

#[test]
fn split_native_single_token_amount_two_inputs() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input1_addr = suite
        .account_init_with_balances("input_account_1", vec![(600_000_000_000_u128, NTRN.into())]);
    let input2_addr = suite
        .account_init_with_balances("input_account_2", vec![(400_000_000_000_u128, NTRN.into())]);

    let cfg = suite.reverse_splitter_config(
        vec![
            UncheckedSplitConfig::with_native_amount(600_000_000_000_u128, NTRN, &input1_addr),
            UncheckedSplitConfig::with_native_amount(400_000_000_000_u128, NTRN, &input2_addr),
        ],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Execute split
    suite.execute_split(lib).unwrap();

    // Verify input accounts balances: should be zero
    suite.assert_balance(&input1_addr, ZERO, NTRN);
    suite.assert_balance(&input2_addr, ZERO, NTRN);

    // Verify output account's balance: should be 1_000_000 NTRN
    suite.assert_balance(suite.output_addr(), ONE_MILLION, NTRN);
}

#[test]
fn split_native_two_token_amounts_two_inputs() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input1_addr =
        suite.account_init_with_balances("input_account_1", vec![(ONE_MILLION, NTRN.into())]);
    let input2_addr =
        suite.account_init_with_balances("input_account_2", vec![(TEN_MILLION, STARS.into())]);

    let cfg = suite.reverse_splitter_config(
        vec![
            UncheckedSplitConfig::with_native_amount(ONE_MILLION, NTRN, &input1_addr),
            UncheckedSplitConfig::with_native_amount(TEN_MILLION, STARS, &input2_addr),
        ],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Assert initial balances
    suite.assert_balance(&input1_addr, ONE_MILLION, NTRN);
    suite.assert_balance(&input2_addr, TEN_MILLION, STARS);

    // Execute split
    suite.execute_split(lib).unwrap();

    // Verify input account's balance: should be zero
    suite.assert_balance(&input1_addr, ZERO, NTRN);
    suite.assert_balance(&input2_addr, ZERO, STARS);

    // Verify output account's balances: should be 1_000_000 NTRN & 10_000_000 STARS
    suite.assert_balance(suite.output_addr(), ONE_MILLION, NTRN);
    suite.assert_balance(suite.output_addr(), TEN_MILLION, STARS);
}

#[test]
fn split_cw20_single_token_amount_single_input() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input_addr = suite.account_init_with_balances("input_account", vec![]);

    let cw20_addr =
        suite.cw20_token_init(MEME, "MEME", vec![(ONE_MILLION, input_addr.to_string())]);

    let cfg = suite.reverse_splitter_config(
        vec![UncheckedSplitConfig::with_cw20_amount(
            ONE_MILLION,
            &cw20_addr,
            &input_addr,
        )],
        UncheckedDenom::Cw20(cw20_addr.to_string()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Execute split
    suite.execute_split(lib).unwrap();

    // Verify input account's balance: should be zero
    suite.assert_cw20_balance(&input_addr, ZERO, &cw20_addr);

    // Verify output account's balance: should be 1_000_000 MEME
    suite.assert_cw20_balance(suite.output_addr(), ONE_MILLION, &cw20_addr);
}

#[test]
fn split_cw20_single_token_amount_two_inputs() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input1_addr = suite.account_init_with_balances("input_account_1", vec![]);
    let input2_addr = suite.account_init_with_balances("input_account_2", vec![]);

    let cw20_addr = suite.cw20_token_init(
        MEME,
        "MEME",
        vec![
            (600_000_000_000_u128, input1_addr.to_string()),
            (400_000_000_000_u128, input2_addr.to_string()),
        ],
    );

    let cfg = suite.reverse_splitter_config(
        vec![
            UncheckedSplitConfig::with_cw20_amount(600_000_000_000_u128, &cw20_addr, &input1_addr),
            UncheckedSplitConfig::with_cw20_amount(400_000_000_000_u128, &cw20_addr, &input2_addr),
        ],
        UncheckedDenom::Cw20(cw20_addr.to_string()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Execute split
    suite.execute_split(lib).unwrap();

    // Verify input accounts balances: should be zero
    suite.assert_cw20_balance(&input1_addr, ZERO, &cw20_addr);
    suite.assert_cw20_balance(&input2_addr, ZERO, &cw20_addr);

    // Verify output account's balance: should be 1_000_000 MEME
    suite.assert_cw20_balance(suite.output_addr(), ONE_MILLION, &cw20_addr);
}

#[test]
fn split_cw20_two_token_amounts_two_inputs() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input1_addr = suite.account_init_with_balances("input_account_1", vec![]);
    let input2_addr = suite.account_init_with_balances("input_account_2", vec![]);

    let cw20_1_addr =
        suite.cw20_token_init(MEME, "MEME", vec![(ONE_MILLION, input1_addr.to_string())]);
    let cw20_2_addr =
        suite.cw20_token_init(CATZ, "CATZ", vec![(TEN_MILLION, input2_addr.to_string())]);

    let cfg = suite.reverse_splitter_config(
        vec![
            UncheckedSplitConfig::with_cw20_amount(ONE_MILLION, &cw20_1_addr, &input1_addr),
            UncheckedSplitConfig::with_cw20_amount(TEN_MILLION, &cw20_2_addr, &input2_addr),
        ],
        UncheckedDenom::Cw20(cw20_1_addr.to_string()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Assert initial balances
    suite.assert_cw20_balance(&input1_addr, ONE_MILLION, &cw20_1_addr);
    suite.assert_cw20_balance(&input2_addr, TEN_MILLION, &cw20_2_addr);

    // Execute split
    suite.execute_split(lib).unwrap();

    // Verify input accounts balances: should be zero
    suite.assert_cw20_balance(&input1_addr, ZERO, &cw20_1_addr);
    suite.assert_cw20_balance(&input2_addr, ZERO, &cw20_2_addr);

    // Verify output account's balances: should be 1_000_000 MEME & 10_000_000 CATZ
    suite.assert_cw20_balance(suite.output_addr(), ONE_MILLION, &cw20_1_addr);
    suite.assert_cw20_balance(suite.output_addr(), TEN_MILLION, &cw20_2_addr);
}

#[test]
fn split_mix_two_token_amounts_two_inputs() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input1_addr =
        suite.account_init_with_balances("input_account_1", vec![(ONE_MILLION, NTRN.into())]);
    let input2_addr = suite.account_init_with_balances("input_account_2", vec![]);

    let cw20_addr =
        suite.cw20_token_init(CATZ, "CATZ", vec![(TEN_MILLION, input2_addr.to_string())]);

    let cfg = suite.reverse_splitter_config(
        vec![
            UncheckedSplitConfig::with_native_amount(ONE_MILLION, NTRN, &input1_addr),
            UncheckedSplitConfig::with_cw20_amount(TEN_MILLION, &cw20_addr, &input2_addr),
        ],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Assert initial balances
    suite.assert_balance(&input1_addr, ONE_MILLION, NTRN);
    suite.assert_cw20_balance(&input2_addr, TEN_MILLION, &cw20_addr);

    // Execute split
    suite.execute_split(lib).unwrap();

    // Verify input account's balance: should be zero
    suite.assert_balance(&input1_addr, ZERO, NTRN);
    suite.assert_cw20_balance(&input1_addr, ZERO, &cw20_addr);

    // Verify output account's balances: should be 1_000_000 MEME & 10_000_000 CATZ
    suite.assert_balance(suite.output_addr(), ONE_MILLION, NTRN);
    suite.assert_cw20_balance(suite.output_addr(), TEN_MILLION, &cw20_addr);
}

#[test]
fn split_native_two_token_partial_amounts_two_inputs() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input1_addr = suite.account_init_with_balances(
        "input_account_1",
        vec![(FIVE_MILLION, NTRN.into()), (HALF_MILLION, STARS.into())],
    );
    let input2_addr = suite.account_init_with_balances(
        "input_account_2",
        vec![(FIVE_MILLION, NTRN.into()), (HALF_MILLION, STARS.into())],
    );

    let cfg = suite.reverse_splitter_config(
        vec![
            UncheckedSplitConfig::with_native_amount(FIVE_MILLION, NTRN, &input1_addr),
            UncheckedSplitConfig::with_native_amount(HALF_MILLION, STARS, &input1_addr),
            UncheckedSplitConfig::with_native_amount(FIVE_MILLION, NTRN, &input2_addr),
            UncheckedSplitConfig::with_native_amount(HALF_MILLION, STARS, &input2_addr),
        ],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Assert initial balances
    suite.assert_balance(&input1_addr, FIVE_MILLION, NTRN);
    suite.assert_balance(&input1_addr, HALF_MILLION, STARS);
    suite.assert_balance(&input2_addr, FIVE_MILLION, NTRN);
    suite.assert_balance(&input2_addr, HALF_MILLION, STARS);

    // Execute split
    suite.execute_split(lib).unwrap();

    // Verify input account's balance: should be zero
    suite.assert_balance(&input1_addr, ZERO, NTRN);
    suite.assert_balance(&input1_addr, ZERO, STARS);
    suite.assert_balance(&input2_addr, ZERO, NTRN);
    suite.assert_balance(&input2_addr, ZERO, STARS);

    // Verify output account 1's balance: should be 5 million STARS & half a million NTRN
    suite.assert_balance(suite.output_addr(), TEN_MILLION, NTRN);
    suite.assert_balance(suite.output_addr(), ONE_MILLION, STARS);
}

// Insufficient balance tests

#[test]
#[should_panic(
    expected = "Execution error: Insufficient balance on account cosmwasm18ygxc482fgklywq5e2fsnmnkqflwaq5u07f9yw824ajfu2x6920sv28wwu for denom 'Native(\"untrn\")' in split config (required: 10000000000000, available: 1000000000000)."
)]
fn split_native_single_token_amount_fails_for_insufficient_balance() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input1_addr =
        suite.account_init_with_balances("input_account_1", vec![(ONE_MILLION, NTRN.into())]);

    let cfg = suite.reverse_splitter_config(
        vec![UncheckedSplitConfig::with_native_amount(
            TEN_MILLION,
            NTRN,
            &input1_addr,
        )],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Execute split
    suite.execute_split(lib).unwrap();
}

#[test]
#[should_panic(
    expected = "Execution error: Insufficient balance on account cosmwasm18ygxc482fgklywq5e2fsnmnkqflwaq5u07f9yw824ajfu2x6920sv28wwu for denom 'Cw20(Addr(\"cosmwasm1wug8sewp6cedgkmrmvhl3lf3tulagm9hnvy8p0rppz9yjw0g4wtqlrtkzd\"))' in split config (required: 10000000000000, available: 1000000000000)."
)]
fn split_cw20_single_token_amount_fails_for_insufficient_balance() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input1_addr = suite.account_init_with_balances("input_account_1", vec![]);

    let cw20_addr =
        suite.cw20_token_init(MEME, "MEME", vec![(ONE_MILLION, input1_addr.to_string())]);

    let cfg = suite.reverse_splitter_config(
        vec![UncheckedSplitConfig::with_cw20_amount(
            TEN_MILLION,
            &cw20_addr,
            &input1_addr,
        )],
        UncheckedDenom::Cw20(cw20_addr.to_string()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Execute split
    suite.execute_split(lib).unwrap();
}

// Native & CW20 token ratio splits

#[test]
fn split_native_two_token_ratios_two_inputs() {
    let mut suite = ReverseSplitterTestSuite::default();

    let input1_addr =
        suite.account_init_with_balances("input_account_1", vec![(ONE_MILLION, NTRN.into())]);
    let input2_addr =
        suite.account_init_with_balances("input_account_2", vec![(TEN_MILLION, STARS.into())]);

    // Hypothetical ratio for NTRN/STARS is 1:10
    let cfg = suite.reverse_splitter_config(
        vec![
            UncheckedSplitConfig::with_native_ratio(Decimal::one(), NTRN, &input1_addr),
            UncheckedSplitConfig::with_native_ratio(Decimal::percent(10u64), STARS, &input2_addr),
        ],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Execute split
    suite.execute_split(lib).unwrap();

    // Verify input accounts balances: should be zero for both
    suite.assert_balance(&input1_addr, ZERO, NTRN);
    suite.assert_balance(&input2_addr, ZERO, STARS);

    // Verify output account's balances: should be 1_000_000 NTRN & 10_000_000 STARS
    suite.assert_balance(suite.output_addr(), ONE_MILLION, NTRN);
    suite.assert_balance(suite.output_addr(), TEN_MILLION, STARS);
}

#[test]
fn split_mix_three_token_ratios_three_inputs() {
    const NTRN_AMOUNT: u128 = ONE_MILLION / 3;
    const STARS_AMOUNT: u128 = TEN_MILLION;
    const MEME_AMOUNT: u128 = HUNDRED_MILLION / 2;
    const NTRN_STARS_RATIO: u128 = 10;
    const NTRN_MEME_RATIO: u128 = 100;

    let mut suite = ReverseSplitterTestSuite::default();

    let input1_addr =
        suite.account_init_with_balances("input_account_1", vec![(NTRN_AMOUNT, NTRN.into())]);
    let input2_addr =
        suite.account_init_with_balances("input_account_2", vec![(STARS_AMOUNT, STARS.into())]);
    let input3_addr = suite.account_init_with_balances("input_account_3", vec![]);

    let cw20_addr =
        suite.cw20_token_init(MEME, "MEME", vec![(MEME_AMOUNT, input3_addr.to_string())]);

    // Hypothetical ratios:
    // NTRN/STARS is 1:10
    // NTRN/MEME is 1:100
    let cfg = suite.reverse_splitter_config(
        vec![
            UncheckedSplitConfig::with_native_ratio(Decimal::one(), NTRN, &input1_addr),
            UncheckedSplitConfig::with_native_ratio(
                Decimal::percent(100u64 / NTRN_STARS_RATIO as u64),
                STARS,
                &input2_addr,
            ),
            UncheckedSplitConfig::with_cw20_ratio(
                Decimal::percent(100u64 / NTRN_MEME_RATIO as u64),
                &cw20_addr,
                &input3_addr,
            ),
        ],
        UncheckedDenom::Native(NTRN.into()),
    );

    // Instantiate Reverse Splitter contract
    let lib = suite.reverse_splitter_init(&cfg);

    // Execute split
    suite.execute_split(lib).unwrap();

    // The expected splits are based on the fact that the Reverse Splitter
    // will transfer as big an amount as possible in the base denom.
    //
    // Starting balances in input account:
    // NTRN:     333_333
    // STARS: 10_000_000
    // MEME:  50_000_000
    //
    // Expected transferred amounts:
    // NTRN:                     333_333
    // STARS: 333_333 *  10 =  3_333_330
    // MEME:  333_333 * 100 = 33_333_300
    //
    // Expected remaining balances:
    // NTRN:     333_333 -    333_333 =          0
    // STARS: 10_000_000 -  3_333_330 =  6_666_670
    // MEME:  50_000_000 - 33_333_300 = 16_666_700

    // Verify input account balances
    suite.assert_balance(&input1_addr, ZERO, NTRN);
    suite.assert_balance(
        &input2_addr,
        STARS_AMOUNT - (NTRN_AMOUNT * NTRN_STARS_RATIO),
        STARS,
    );
    suite.assert_cw20_balance(
        &input3_addr,
        MEME_AMOUNT - (NTRN_AMOUNT * NTRN_MEME_RATIO),
        &cw20_addr,
    );

    // Verify output account's balances: should be 333_333 NTRN, 3_333_330 STARS, 33_333_300 MEME
    suite.assert_balance(suite.output_addr(), NTRN_AMOUNT, NTRN);
    suite.assert_balance(suite.output_addr(), NTRN_AMOUNT * NTRN_STARS_RATIO, STARS);
    suite.assert_cw20_balance(
        suite.output_addr(),
        NTRN_AMOUNT * NTRN_MEME_RATIO,
        &cw20_addr,
    );
}

// Dynamic ratio tests

// #[test]
// fn split_native_single_token_dyn_ratio_single_input() {
//     let mut suite = ReverseSplitterTestSuite::default();

//     let input1_addr =
//         suite.account_init_with_balances("input_account_1", vec![(ONE_MILLION, NTRN.into())]);
//     let input2_addr =
//         suite.account_init_with_balances("input_account_2", vec![(TEN_MILLION, STARS.into())]);

//     // let dyn_ratio_addr = suite.dyn_ratio_contract_init(STARS,  Decimal::percent(10u64));

//     let cfg = suite.reverse_splitter_config(
//         vec![
//             UncheckedSplitConfig::with_native_ratio(Decimal::one(), NTRN, &input1_addr),
//             UncheckedSplitConfig::with_native_dyn_ratio(&dyn_ratio_addr, "", STARS, &input2_addr),
//         ],
//         UncheckedDenom::Native(NTRN.into()),
//     );

//     // Instantiate Splitter contract
//     let lib = suite.reverse_splitter_init(&cfg);

//     // Execute split
//     suite.execute_split(lib).unwrap();

//     // Verify input account's balance: should be zero
//     suite.assert_balance(&input1_addr, ZERO, NTRN);
//     suite.assert_balance(&input2_addr, ZERO, STARS);

//     // Verify output account's balance: should be 1_000_000 NTRN
//     suite.assert_balance(suite.output_addr(), ONE_MILLION, NTRN);
//     suite.assert_balance(suite.output_addr(), TEN_MILLION, STARS);
// }

// #[test]
// fn split_cw20_single_token_dyn_ratio_single_output() {
//     let mut suite = ReverseSplitterTestSuite::default();

//     let input1_addr =
//         suite.account_init_with_balances("input_account_1", vec![(ONE_MILLION, NTRN.into())]);
//     let input2_addr = suite.account_init_with_balances("input_account_2", vec![]);

//     let cw20_addr =
//         suite.cw20_token_init(MEME, "MEME", vec![(TEN_MILLION, input2_addr.to_string())]);

//     let dyn_ratio_addr = suite.dyn_ratio_contract_init(cw20_addr.as_ref(), Decimal::percent(10u64));

//     let cfg = suite.reverse_splitter_config(
//         vec![
//             UncheckedSplitConfig::with_native_ratio(Decimal::one(), NTRN, &input1_addr),
//             UncheckedSplitConfig::with_cw20_dyn_ratio(
//                 &dyn_ratio_addr,
//                 "",
//                 &cw20_addr,
//                 &input2_addr,
//             ),
//         ],
//         UncheckedDenom::Native(NTRN.into()),
//     );

//     // Instantiate Reverse Splitter contract
//     let lib = suite.reverse_splitter_init(&cfg);

//     // Execute split
//     suite.execute_split(lib).unwrap();

//     // Verify input account's balance: should be zero
//     suite.assert_balance(&input1_addr, ZERO, NTRN);
//     suite.assert_cw20_balance(&input2_addr, ZERO, &cw20_addr);

//     // Verify output account's balance: should be 1_000_000 MEME
//     suite.assert_balance(suite.output_addr(), ONE_MILLION, NTRN);
//     suite.assert_cw20_balance(suite.output_addr(), TEN_MILLION, &cw20_addr);
// }
