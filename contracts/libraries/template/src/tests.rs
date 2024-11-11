use crate::msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{error::AnyResult, App, AppResponse, ContractWrapper, Executor};
use cw_ownable::Ownership;
use getset::{Getters, Setters};
use valence_library_utils::{
    msg::{ExecuteMsg, InstantiateMsg},
    testing::{LibraryTestSuite, LibraryTestSuiteBase},
    OptionUpdate,
};

#[derive(Getters, Setters)]
struct TemplateTestSuite {
    #[getset(get)]
    inner: LibraryTestSuiteBase,
    #[getset(get)]
    template_code_id: u64,
}

impl Default for TemplateTestSuite {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl TemplateTestSuite {
    pub fn new() -> Self {
        let mut inner = LibraryTestSuiteBase::new();

        // Template contract
        let template_code = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );

        let template_code_id = inner.app_mut().store_code(Box::new(template_code));

        Self {
            inner,
            template_code_id,
        }
    }

    pub fn template_init(&mut self, cfg: &LibraryConfig) -> Addr {
        let init_msg = InstantiateMsg {
            owner: self.owner().to_string(),
            processor: self.processor().to_string(),
            config: cfg.clone(),
        };
        self.contract_init(self.template_code_id, "template", &init_msg, &[])
    }

    fn template_config(&self, admin: String) -> LibraryConfig {
        LibraryConfig {
            skip_update_admin: valence_library_utils::LibraryAccountType::Addr(admin),
            optional: None,
            optional2: "s".to_string(),
        }
    }

    fn execute_noop(&mut self, addr: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::<_, LibraryConfig>::ProcessFunction(FunctionMsgs::NoOp {}),
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

impl LibraryTestSuite<Empty, Empty> for TemplateTestSuite {
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
    let mut suite = TemplateTestSuite::default();

    let admin_addr = suite.owner().clone();
    let cfg = suite.template_config(admin_addr.to_string());

    // Instantiate Template contract
    let lib = suite.template_init(&cfg);

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
        Config {
            admin: admin_addr,
            optional: None
        }
    );

    let raw_lib_cfg: LibraryConfig = suite.query_wasm(&lib, &QueryMsg::GetRawLibraryConfig {});
    assert_eq!(
        raw_lib_cfg,
        LibraryConfig {
            skip_update_admin: valence_library_utils::LibraryAccountType::Addr(
                suite.owner().to_string()
            ),
            optional: None,
            optional2: "s".to_string(),
        }
    );

    // Here we just want to make sure that our ignore_optional actually works
    // Because we ignore the only available field, LibraryConfigUpdate expected to have no fields
    let _ = LibraryConfigUpdate {
        optional: OptionUpdate::Set(None),
        optional2: Some("s".to_string()),
    };
}

#[test]
fn get_diff_update() {
    let suite = TemplateTestSuite::default();

    let admin_addr = suite.owner().clone();
    let old_cfg = suite.template_config(admin_addr.to_string());
    let mut new_cfg = suite.template_config(admin_addr.to_string());

    // We didn't change anything, so if we run get_diff_update, it should return None
    assert!(old_cfg.get_diff_update(new_cfg.clone()).is_none());

    new_cfg.optional = Some("optional".to_string());

    // We changed the optional field, so if we run get_diff_update, it should return Some
    let update = old_cfg.get_diff_update(new_cfg.clone());
    assert_eq!(
        update.unwrap(),
        LibraryConfigUpdate {
            optional: OptionUpdate::Set(Some("optional".to_string())),
            optional2: None
        }
    );
}

#[test]
fn execute_function() {
    let mut suite = TemplateTestSuite::default();

    let cfg = suite.template_config(suite.owner().to_string());

    // Instantiate Template contract
    let lib = suite.template_init(&cfg);

    // Execute function
    suite.execute_noop(lib).unwrap();
}
