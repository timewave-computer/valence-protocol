use crate::msg::{ActionMsgs, Config, IbcTransferAmount, LibraryConfig, QueryMsg, RemoteChainInfo};
use cosmwasm_std::{
    coin, to_json_binary, Addr, Api, BlockInfo, CustomMsg, CustomQuery, Empty, Storage, Uint128,
    Uint64,
};
use cw_multi_test::{
    error::AnyResult, no_init, AppBuilder, AppResponse, ContractWrapper, CosmosRouter, Executor,
    Module, SudoMsg,
};
use cw_ownable::Ownership;
use getset::{Getters, Setters};
use neutron_sdk::{bindings::query::NeutronQuery, query::min_ibc_fee::MinIbcFeeResponse};
use serde::de::DeserializeOwned;
use valence_library_utils::{
    denoms::CheckedDenom,
    msg::{ExecuteMsg, InstantiateMsg, LibraryConfigValidation},
    testing::{CustomLibraryTestSuiteBase, LibraryTestSuite, TestApp},
    LibraryAccountType,
};

const NTRN: &str = "untrn";
const ATOM: &str = "uatom";
const ONE_HUNDRED: u128 = 100_000_000_u128;
const ONE_MILLION: u128 = 1_000_000_000_000_u128;

#[derive(Getters, Setters)]
struct IbcTransferTestSuite {
    #[getset(get)]
    inner: CustomLibraryTestSuiteBase<Empty, NeutronQuery, NeutronKeeper>,
    #[getset(get)]
    ibc_transfer_code_id: u64,
    #[getset(get)]
    input_addr: Addr,
    #[getset(get)]
    output_addr: Addr,
    #[getset(get)]
    input_balance: Option<(u128, String)>,
}

impl Default for IbcTransferTestSuite {
    fn default() -> Self {
        Self::new(None)
    }
}

#[allow(dead_code)]
impl IbcTransferTestSuite {
    pub fn new(input_balance: Option<(u128, String)>) -> Self {
        let app = AppBuilder::new_custom()
            .with_custom(NeutronKeeper::new())
            .build(no_init);
        let mut inner = CustomLibraryTestSuiteBase::new(app);

        let input_addr = inner.get_contract_addr(inner.account_code_id(), "input_account");
        let output_addr = inner.api().addr_make("output_account");

        // Template contract
        let ibc_transfer_code = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );

        let ibc_transfer_code_id = inner.app_mut().store_code(Box::new(ibc_transfer_code));

        Self {
            inner,
            ibc_transfer_code_id,
            input_addr,
            output_addr,
            input_balance,
        }
    }

    pub fn ibc_transfer_init(&mut self, cfg: &LibraryConfig) -> Addr {
        let init_msg = InstantiateMsg {
            owner: self.owner().to_string(),
            processor: self.processor().to_string(),
            config: cfg.clone(),
        };
        let addr = self.contract_init(
            self.ibc_transfer_code_id,
            "ibc_transfer_library",
            &init_msg,
            &[],
        );

        let input_addr = self.input_addr().clone();
        if self.app_mut().contract_data(&input_addr).is_err() {
            let account_addr = self.account_init("input_account", vec![addr.to_string()]);
            assert_eq!(account_addr, input_addr);

            if let Some((amount, denom)) = self.input_balance.as_ref().cloned() {
                self.init_balance(&input_addr, vec![coin(amount, denom.to_string())]);
            }
        }

        addr
    }

    fn ibc_transfer_config(
        &self,
        denom: String,
        amount: IbcTransferAmount,
        memo: String,
        remote_chain_info: RemoteChainInfo,
    ) -> LibraryConfig {
        LibraryConfig::new(
            valence_library_utils::LibraryAccountType::Addr(self.input_addr().to_string()),
            self.output_addr().to_string(),
            valence_library_utils::denoms::UncheckedDenom::Native(denom),
            amount,
            memo,
            remote_chain_info,
        )
    }

    fn execute_ibc_transfer(&mut self, addr: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::<_, LibraryConfig>::ProcessAction(ActionMsgs::IbcTransfer {}),
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

impl LibraryTestSuite<Empty, NeutronQuery, NeutronKeeper> for IbcTransferTestSuite {
    fn app(&self) -> &TestApp<Empty, NeutronQuery, NeutronKeeper> {
        self.inner.app()
    }

    fn app_mut(&mut self) -> &mut TestApp<Empty, NeutronQuery, NeutronKeeper> {
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

#[allow(dead_code)]
pub trait Neutron: Module<ExecT = Empty, QueryT = NeutronQuery, SudoT = SudoMsg> {}

pub struct NeutronKeeper {}

impl Neutron for NeutronKeeper {}

impl NeutronKeeper {
    pub fn new() -> Self {
        Self {}
    }
}

impl Module for NeutronKeeper {
    type ExecT = Empty;
    type QueryT = NeutronQuery;
    type SudoT = SudoMsg;

    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        _msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        unimplemented!()
    }

    fn query(
        &self,
        _api: &dyn cosmwasm_std::Api,
        _storage: &dyn cosmwasm_std::Storage,
        _querier: &dyn cosmwasm_std::Querier,
        _block: &cosmwasm_std::BlockInfo,
        request: Self::QueryT,
    ) -> cw_multi_test::error::AnyResult<cosmwasm_std::Binary> {
        match request {
            NeutronQuery::MinIbcFee {} => Ok(to_json_binary(&MinIbcFeeResponse {
                min_fee: neutron_sdk::bindings::msg::IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![coin(10_000, NTRN)],
                    timeout_fee: vec![coin(10_000, NTRN)],
                },
            })
            .unwrap()),
            _ => {
                unimplemented!()
            }
        }
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn cosmwasm_std::Api,
        _storage: &mut dyn cosmwasm_std::Storage,
        _router: &dyn cw_multi_test::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &cosmwasm_std::BlockInfo,
        _msg: Self::SudoT,
    ) -> cw_multi_test::error::AnyResult<cw_multi_test::AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + cosmwasm_schema::schemars::JsonSchema
            + cosmwasm_schema::serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + cosmwasm_schema::serde::de::DeserializeOwned + 'static,
    {
        unimplemented!()
    }
}

// Note: all tests below are replicated from the Generic IBC transfer service
// Any change in the tests below should be reflected in the Generic IBC transfer service.

#[test]
fn instantiate_with_valid_config() {
    let mut suite = IbcTransferTestSuite::default();

    let cfg = suite.ibc_transfer_config(
        NTRN.to_string(),
        IbcTransferAmount::FullAmount,
        "".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600u64.into())),
    );

    // Instantiate IBC transfer contract
    let lib = suite.ibc_transfer_init(&cfg);

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
            suite.input_addr().clone(),
            suite.output_addr().clone(),
            CheckedDenom::Native(NTRN.into()),
            IbcTransferAmount::FullAmount,
            "".to_string(),
            cfg.remote_chain_info.clone()
        )
    );
}

#[test]
fn pre_validate_config_works() {
    let suite = IbcTransferTestSuite::default();

    let cfg = suite.ibc_transfer_config(
        NTRN.to_string(),
        IbcTransferAmount::FullAmount,
        "".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600u64.into())),
    );

    // Pre-validate config
    cfg.pre_validate(suite.api()).unwrap();
}

#[test]
#[should_panic(expected = "Invalid IBC transfer config: amount cannot be zero.")]
fn instantiate_fails_for_zero_amount() {
    let mut suite = IbcTransferTestSuite::default();

    let cfg = suite.ibc_transfer_config(
        NTRN.to_string(),
        IbcTransferAmount::FixedAmount(Uint128::zero()),
        "".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600u64.into())),
    );

    // Instantiate IBC transfer contract
    suite.ibc_transfer_init(&cfg);
}

#[test]
#[should_panic(
    expected = "Invalid IBC transfer config: remote_chain_info's channel_id cannot be empty."
)]
fn instantiate_fails_for_invalid_channel_id() {
    let mut suite = IbcTransferTestSuite::default();

    let cfg = suite.ibc_transfer_config(
        NTRN.to_string(),
        IbcTransferAmount::FixedAmount(Uint128::one()),
        "".to_string(),
        RemoteChainInfo::new("".to_string(), Some(600u64.into())),
    );

    // Instantiate IBC transfer contract
    suite.ibc_transfer_init(&cfg);
}

#[test]
#[should_panic(
    expected = "Invalid IBC transfer config: remote_chain_info's ibc_transfer_timeout cannot be zero."
)]
fn instantiate_fails_for_invalid_ibc_transfer_timeout() {
    let mut suite = IbcTransferTestSuite::default();

    let cfg = suite.ibc_transfer_config(
        NTRN.to_string(),
        IbcTransferAmount::FixedAmount(Uint128::one()),
        "".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(Uint64::zero())),
    );

    // Instantiate IBC transfer contract
    suite.ibc_transfer_init(&cfg);
}

// Config update tests

#[test]
#[should_panic(expected = "Invalid IBC transfer config: amount cannot be zero.")]
fn update_config_validates_config() {
    let mut suite = IbcTransferTestSuite::default();

    let mut cfg = suite.ibc_transfer_config(
        NTRN.to_string(),
        IbcTransferAmount::FullAmount,
        "".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600u64.into())),
    );

    // Instantiate IBC transfer contract
    let lib = suite.ibc_transfer_init(&cfg);

    // Update config and set amount to zero
    cfg.amount = IbcTransferAmount::FixedAmount(Uint128::zero());

    // Execute update config action
    suite.update_config(lib.clone(), cfg).unwrap();
}

#[test]
fn update_config_with_valid_config() {
    let mut suite = IbcTransferTestSuite::default();

    let mut cfg = suite.ibc_transfer_config(
        NTRN.to_string(),
        IbcTransferAmount::FullAmount,
        "".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600u64.into())),
    );

    // Instantiate IBC transfer contract
    let lib = suite.ibc_transfer_init(&cfg);

    // Update config: swap input and output addresses
    cfg.input_addr = LibraryAccountType::Addr(suite.output_addr().to_string());
    cfg.output_addr = suite.input_addr().to_string();
    cfg.amount = IbcTransferAmount::FixedAmount(ONE_MILLION.into());
    cfg.memo = "Chancellor on brink of second bailout for banks.".to_string();

    // Execute update config action
    suite.update_config(lib.clone(), cfg).unwrap();

    // Verify library config
    let lib_cfg: Config = suite.query_wasm(&lib, &QueryMsg::GetLibraryConfig {});
    assert_eq!(
        lib_cfg,
        Config::new(
            suite.output_addr().clone(),
            suite.input_addr().clone(),
            CheckedDenom::Native(NTRN.into()),
            IbcTransferAmount::FixedAmount(ONE_MILLION.into()),
            "Chancellor on brink of second bailout for banks.".to_string(),
            RemoteChainInfo::new("channel-1".to_string(), Some(600u64.into()))
        )
    );
}

// Insufficient balance tests

#[test]
#[should_panic(
    expected = "Execution error: Insufficient balance for denom 'untrn' in config (required: 1000000000000, available: 0)."
)]
fn ibc_transfer_fails_for_insufficient_balance() {
    let mut suite = IbcTransferTestSuite::default();

    let cfg = suite.ibc_transfer_config(
        NTRN.to_string(),
        IbcTransferAmount::FixedAmount(ONE_MILLION.into()),
        "".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600u64.into())),
    );

    // Instantiate  contract
    let lib = suite.ibc_transfer_init(&cfg);

    // Execute IBC transfer
    suite.execute_ibc_transfer(lib).unwrap();
}

#[test]
#[should_panic(
    expected = "Execution error: Insufficient balance to cover for IBC fees 'untrn' in sender account (required: 20000, available: 0)."
)]
fn ibc_transfer_fails_for_insufficient_fee_balance() {
    let mut suite = IbcTransferTestSuite::default();

    suite.init_balance(
        &suite.input_addr().clone(),
        vec![coin(ONE_HUNDRED, ATOM.to_string())],
    );

    let cfg: LibraryConfig = suite.ibc_transfer_config(
        ATOM.to_string(),
        IbcTransferAmount::FixedAmount(ONE_HUNDRED.into()),
        "".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600u64.into())),
    );

    // Instantiate  contract
    let lib = suite.ibc_transfer_init(&cfg);

    // Execute IBC transfer
    suite.execute_ibc_transfer(lib).unwrap();
}
