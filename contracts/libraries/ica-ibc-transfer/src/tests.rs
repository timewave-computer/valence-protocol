use std::collections::BTreeMap;

use cosmwasm_std::{coin, Addr, Empty, Uint128};
use cw_multi_test::{error::AnyResult, App, AppResponse, ContractWrapper, Executor};
use cw_ownable::Ownership;
use valence_library_utils::{
    msg::{ExecuteMsg, InstantiateMsg, LibraryConfigValidation},
    testing::{LibraryTestSuite, LibraryTestSuiteBase},
};

use crate::msg::{Config, FunctionMsgs, LibraryConfig, QueryMsg, RemoteChainInfo};

const UUSDC: &str = "uusdc";
const ONE_THOUSAND: u128 = 1_000_000_000;

struct IcaIbcTransferTestSuite {
    inner: LibraryTestSuiteBase,
    ica_ibc_transfer_code_id: u64,
    input_addr: Addr,
    input_balance: Option<(u128, String)>,
}

impl Default for IcaIbcTransferTestSuite {
    fn default() -> Self {
        Self::new(None)
    }
}

#[allow(dead_code)]
impl IcaIbcTransferTestSuite {
    pub fn new(input_balance: Option<(u128, String)>) -> Self {
        let mut inner = LibraryTestSuiteBase::new();

        let input_addr = inner.get_contract_addr(inner.account_code_id(), "input_account");

        // Template contract
        let ica_ibc_transfer_code = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );

        let ica_ibc_transfer_code_id = inner.app_mut().store_code(Box::new(ica_ibc_transfer_code));

        Self {
            inner,
            ica_ibc_transfer_code_id,
            input_addr,
            input_balance,
        }
    }

    pub fn ica_ibc_transfer_init(&mut self, cfg: &LibraryConfig) -> Addr {
        let init_msg = InstantiateMsg {
            owner: self.owner().to_string(),
            processor: self.processor().to_string(),
            config: cfg.clone(),
        };
        let addr = self.contract_init(
            self.ica_ibc_transfer_code_id,
            "ica_ibc_transfer_library",
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

    fn ica_ibc_transfer_config(
        &self,
        denom: String,
        amount: Uint128,
        receiver: String,
        remote_chain_info: RemoteChainInfo,
    ) -> LibraryConfig {
        LibraryConfig::new(
            valence_library_utils::LibraryAccountType::Addr(self.input_addr.to_string()),
            amount,
            denom,
            receiver,
            "".to_string(),
            remote_chain_info,
            BTreeMap::default(),
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

impl LibraryTestSuite<Empty, Empty> for IcaIbcTransferTestSuite {
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
    let mut suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    // Instantiate IBC transfer contract
    let lib = suite.ica_ibc_transfer_init(&cfg);

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
            suite.input_addr.clone(),
            Uint128::new(ONE_THOUSAND),
            UUSDC.to_string(),
            "receiver".to_string(),
            "".to_string(),
            RemoteChainInfo::new("channel-1".to_string(), Some(600)),
            BTreeMap::default(),
        )
    );
}

#[test]
fn pre_validate_config_works() {
    let suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    // Pre-validate config
    cfg.pre_validate(suite.api()).unwrap();
}

#[test]
#[should_panic(expected = "Invalid ICA IBC transfer config: amount cannot be zero.")]
fn instantiate_fails_for_zero_amount() {
    let mut suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::zero(),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    suite.ica_ibc_transfer_init(&cfg);
}

#[test]
#[should_panic(expected = "Invalid ICA IBC transfer config: denom cannot be empty.")]
fn instantiate_fails_for_empty_denom() {
    let mut suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        "".to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    suite.ica_ibc_transfer_init(&cfg);
}

#[test]
#[should_panic(expected = "Invalid ICA IBC transfer config: channel_id cannot be empty.")]
fn instantiate_fails_for_empty_channel_id() {
    let mut suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("".to_string(), Some(600)),
    );

    suite.ica_ibc_transfer_init(&cfg);
}

#[test]
#[should_panic(expected = "Invalid ICA IBC transfer config: receiver cannot be empty.")]
fn instantiate_fails_for_empty_receiver() {
    let mut suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    suite.ica_ibc_transfer_init(&cfg);
}

#[test]
#[should_panic(expected = "Invalid ICA IBC transfer config: timeout cannot be zero.")]
fn instantiate_fails_for_zero_timeout() {
    let mut suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(0)),
    );

    suite.ica_ibc_transfer_init(&cfg);
}

#[test]
#[should_panic(expected = "Invalid ICA IBC transfer config: amount cannot be zero.")]
fn update_config_validates_amount() {
    let mut suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    let lib = suite.ica_ibc_transfer_init(&cfg);

    let new_cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::zero(),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    suite.update_config(lib, new_cfg).unwrap();
}

#[test]
#[should_panic(expected = "Invalid ICA IBC transfer config: denom cannot be empty.")]
fn update_config_validates_denom() {
    let mut suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    let lib = suite.ica_ibc_transfer_init(&cfg);

    let new_cfg = suite.ica_ibc_transfer_config(
        "".to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    suite.update_config(lib, new_cfg).unwrap();
}

#[test]
#[should_panic(expected = "Invalid ICA IBC transfer config: receiver cannot be empty.")]
fn update_config_validates_receiver() {
    let mut suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    let lib = suite.ica_ibc_transfer_init(&cfg);

    let new_cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    suite.update_config(lib, new_cfg).unwrap();
}

#[test]
#[should_panic(expected = "Invalid ICA IBC transfer config: channel_id cannot be empty.")]
fn update_config_validates_channel_id() {
    let mut suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    let lib = suite.ica_ibc_transfer_init(&cfg);

    let new_cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("".to_string(), Some(600)),
    );

    suite.update_config(lib, new_cfg).unwrap();
}

#[test]
#[should_panic(expected = "Invalid ICA IBC transfer config: timeout cannot be zero.")]
fn update_config_validates_timeout() {
    let mut suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    let lib = suite.ica_ibc_transfer_init(&cfg);

    let new_cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(0)),
    );

    suite.update_config(lib, new_cfg).unwrap();
}

#[test]
fn update_config_works() {
    let mut suite = IcaIbcTransferTestSuite::default();

    let cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-1".to_string(), Some(600)),
    );

    let lib = suite.ica_ibc_transfer_init(&cfg);

    let new_cfg = suite.ica_ibc_transfer_config(
        UUSDC.to_string(),
        Uint128::new(ONE_THOUSAND * 2),
        "receiver".to_string(),
        RemoteChainInfo::new("channel-2".to_string(), Some(1200)),
    );

    suite.update_config(lib.clone(), new_cfg.clone()).unwrap();

    // Verify library config
    let lib_cfg: Config = suite.query_wasm(&lib, &QueryMsg::GetLibraryConfig {});
    assert_eq!(
        lib_cfg,
        Config::new(
            Addr::unchecked(new_cfg.input_addr.to_string().unwrap()),
            new_cfg.amount,
            new_cfg.denom,
            new_cfg.receiver,
            new_cfg.memo,
            new_cfg.remote_chain_info,
            BTreeMap::default()
        )
    );
}
