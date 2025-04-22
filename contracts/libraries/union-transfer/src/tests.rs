use crate::msg::{
    Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg, TransferAmount,
    UncheckedUnionDenomConfig,
};
use cosmwasm_std::{coin, Addr, Empty, Uint128, Uint256};
use cw_multi_test::{error::AnyResult, App, AppResponse, ContractWrapper, Executor};
use cw_ownable::Ownership;
use valence_library_utils::{
    msg::{ExecuteMsg, InstantiateMsg, LibraryConfigValidation},
    testing::{LibraryTestSuite, LibraryTestSuiteBase},
    LibraryAccountType,
};

const NTRN: &str = "untrn";
const ONE_MILLION: u128 = 1_000_000_000_000_u128;

struct UnionTransferTestSuite {
    inner: LibraryTestSuiteBase,
    union_transfer_code_id: u64,
    input_addr: Addr,
    output_addr: String,
    input_balance: Option<(u128, String)>,
}

impl Default for UnionTransferTestSuite {
    fn default() -> Self {
        Self::new(None)
    }
}

#[allow(dead_code)]
impl UnionTransferTestSuite {
    pub fn new(input_balance: Option<(u128, String)>) -> Self {
        let mut inner = LibraryTestSuiteBase::new();

        let input_addr = inner.get_contract_addr(inner.account_code_id(), "input_account");
        let output_addr = inner.api().addr_make("output_account").to_string();

        // Template contract
        let union_transfer_code = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );

        let union_transfer_code_id = inner.app_mut().store_code(Box::new(union_transfer_code));

        Self {
            inner,
            union_transfer_code_id,
            input_addr,
            output_addr,
            input_balance,
        }
    }

    pub fn union_transfer_init(&mut self, cfg: &LibraryConfig) -> Addr {
        let init_msg = InstantiateMsg {
            owner: self.owner().to_string(),
            processor: self.processor().to_string(),
            config: cfg.clone(),
        };
        let addr = self.contract_init(
            self.union_transfer_code_id,
            "union_transfer_library",
            &init_msg,
            &[],
        );

        let input_addr = self.input_addr.clone();
        if self.app_mut().contract_data(&input_addr).is_err() {
            let account_addr = self.account_init("input_account", vec![addr.to_string()]);
            assert_eq!(account_addr, input_addr);

            if let Some((amount, denom)) = self.input_balance.as_ref().cloned() {
                self.init_balance(&input_addr, vec![coin(amount, denom.to_string())]);
            }
        }

        addr
    }

    fn union_transfer_config(&self, denom: String, amount: TransferAmount) -> LibraryConfig {
        LibraryConfig::new(
            valence_library_utils::LibraryAccountType::Addr(self.input_addr.to_string()),
            valence_library_utils::LibraryAccountType::Addr(self.output_addr.to_string()),
            UncheckedUnionDenomConfig::Native(denom),
            amount,
            NTRN.to_string(),
            NTRN.to_string(),
            6,
            Uint256::zero(),
            "0xe53dcec07d16d88e386ae0710e86d9a400f83c31".to_string(),
            Uint256::from_u128(100000000),
            1,
            None,
            self.input_addr.to_string(),
            None,
            None,
        )
    }

    fn execute_union_transfer(&mut self, addr: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::<_, LibraryConfig>::ProcessFunction(FunctionMsgs::Transfer {
                quote_amount: None,
            }),
        )
    }

    fn update_config(&mut self, addr: Addr, new_config: LibraryConfig) -> AnyResult<AppResponse> {
        let owner = self.owner().clone();
        let updated_config = LibraryConfigUpdate {
            input_addr: Some(new_config.input_addr),
            output_addr: Some(new_config.output_addr),
            denom: Some(new_config.denom),
            amount: Some(new_config.amount),
            input_asset_name: Some(new_config.input_asset_name),
            input_asset_symbol: Some(new_config.input_asset_symbol),
            input_asset_decimals: Some(new_config.input_asset_decimals),
            input_asset_token_path: Some(new_config.input_asset_token_path),
            quote_token: Some(new_config.quote_token),
            quote_amount: Some(new_config.quote_amount),
            channel_id: Some(new_config.channel_id),
            transfer_timeout: valence_library_utils::OptionUpdate::Set(new_config.transfer_timeout),
            zkgm_contract: Some(new_config.zkgm_contract),
            batch_instruction_version: valence_library_utils::OptionUpdate::Set(
                new_config.batch_instruction_version,
            ),
            transfer_instruction_version: valence_library_utils::OptionUpdate::Set(
                new_config.transfer_instruction_version,
            ),
        };
        self.app_mut().execute_contract(
            owner,
            addr,
            &ExecuteMsg::<FunctionMsgs, LibraryConfigUpdate>::UpdateConfig {
                new_config: updated_config,
            },
            &[],
        )
    }
}

impl LibraryTestSuite<Empty, Empty> for UnionTransferTestSuite {
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
    let mut suite = UnionTransferTestSuite::default();

    let cfg = suite.union_transfer_config(NTRN.to_string(), TransferAmount::FullAmount);

    // Instantiate Union transfer contract
    let lib = suite.union_transfer_init(&cfg);

    // Verify owner
    let owner_res: Ownership<Addr> = suite.query_wasm(&lib, &QueryMsg::Ownership {});
    assert_eq!(owner_res.owner, Some(suite.owner().clone()));

    // Verify processor
    let processor_addr: Addr = suite.query_wasm(&lib, &QueryMsg::GetProcessor {});
    assert_eq!(processor_addr, suite.processor().clone());

    // Verify library config
    let lib_cfg: Config = suite.query_wasm(&lib, &QueryMsg::GetLibraryConfig {});
    assert_eq!(lib_cfg.input_addr.to_string(), suite.input_addr.to_string());
}

#[test]
fn pre_validate_config_works() {
    let suite = UnionTransferTestSuite::default();

    let cfg = suite.union_transfer_config(NTRN.to_string(), TransferAmount::FullAmount);

    // Pre-validate config
    cfg.pre_validate(suite.api()).unwrap();
}

#[test]
#[should_panic(expected = "Invalid Union transfer config: amount cannot be zero.")]
fn instantiate_fails_for_zero_amount() {
    let mut suite = UnionTransferTestSuite::default();

    let cfg = suite.union_transfer_config(
        NTRN.to_string(),
        TransferAmount::FixedAmount(Uint128::zero()),
    );

    // Instantiate Union transfer contract
    suite.union_transfer_init(&cfg);
}

// Config update tests

#[test]
#[should_panic(expected = "Invalid Union transfer config: amount cannot be zero.")]
fn update_config_validates_amount() {
    let mut suite = UnionTransferTestSuite::default();

    let mut cfg = suite.union_transfer_config(NTRN.to_string(), TransferAmount::FullAmount);

    // Instantiate Union transfer contract
    let lib = suite.union_transfer_init(&cfg);

    // Update config and set amount to zero
    cfg.amount = TransferAmount::FixedAmount(Uint128::zero());

    // Execute update config action
    suite.update_config(lib.clone(), cfg).unwrap();
}

#[test]
#[should_panic(expected = "Invalid Union transfer config: transfer_timeout cannot be zero.")]
fn update_config_validates_union_timeout() {
    let mut suite = UnionTransferTestSuite::default();

    let mut cfg = suite.union_transfer_config(NTRN.to_string(), TransferAmount::FullAmount);

    // Instantiate Union transfer contract
    let lib = suite.union_transfer_init(&cfg);

    // Update config and set Union timeout to zero
    cfg.transfer_timeout = Some(0);

    // Execute update config action
    suite.update_config(lib.clone(), cfg).unwrap();
}

#[test]
fn update_config_with_valid_config() {
    let mut suite = UnionTransferTestSuite::default();

    let mut cfg = suite.union_transfer_config(NTRN.to_string(), TransferAmount::FullAmount);

    // Instantiate Union transfer contract
    let lib = suite.union_transfer_init(&cfg);

    // Update config: swap input and output addresses
    cfg.input_addr = LibraryAccountType::Addr(suite.output_addr.to_string());
    cfg.output_addr = LibraryAccountType::Addr(suite.input_addr.to_string());
    cfg.amount = TransferAmount::FixedAmount(ONE_MILLION.into());

    // Execute update config action
    suite.update_config(lib.clone(), cfg).unwrap();

    // Verify library config
    let lib_cfg: Config = suite.query_wasm(&lib, &QueryMsg::GetLibraryConfig {});
    assert_eq!(
        lib_cfg.input_addr.to_string(),
        suite.output_addr.to_string()
    );
}

// Insufficient balance tests

#[test]
#[should_panic(
    expected = "Execution error: Insufficient balance for denom 'untrn' in config (required: 1000000000000, available: 0)."
)]
fn union_transfer_fails_for_insufficient_balance() {
    let mut suite = UnionTransferTestSuite::default();

    let cfg = suite.union_transfer_config(
        NTRN.to_string(),
        TransferAmount::FixedAmount(ONE_MILLION.into()),
    );

    // Instantiate  contract
    let lib = suite.union_transfer_init(&cfg);

    // Execute Union transfer
    suite.execute_union_transfer(lib).unwrap();
}
