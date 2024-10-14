use crate::msg::{ActionMsgs, Config, QueryMsg, ServiceConfig};
use cosmwasm_std::Addr;
use cw_multi_test::{error::AnyResult, App, AppResponse, ContractWrapper, Executor};
use cw_ownable::Ownership;
use getset::{Getters, Setters};
use valence_service_utils::{
    msg::ExecuteMsg,
    msg::InstantiateMsg,
    testing::{ServiceTestSuite, ServiceTestSuiteBase},
};

#[derive(Getters, Setters)]
struct TemplateTestSuite {
    #[getset(get)]
    inner: ServiceTestSuiteBase,
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
        let mut inner = ServiceTestSuiteBase::new();

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

    pub fn template_init(&mut self, cfg: &ServiceConfig) -> Addr {
        let init_msg = InstantiateMsg {
            owner: self.owner().to_string(),
            processor: self.processor().to_string(),
            config: cfg.clone(),
        };
        self.contract_init(self.template_code_id, "template", &init_msg, &[])
    }

    fn template_config(&self) -> ServiceConfig {
        ServiceConfig {}
    }

    fn execute_noop(&mut self, addr: Addr) -> AnyResult<AppResponse> {
        self.contract_execute(
            addr,
            &ExecuteMsg::<_, ServiceConfig>::ProcessAction(ActionMsgs::NoOp {}),
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

impl ServiceTestSuite for TemplateTestSuite {
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

    let cfg = suite.template_config();

    // Instantiate Template contract
    let svc = suite.template_init(&cfg);

    // Verify owner
    let owner_res: Ownership<Addr> = suite.query_wasm(&svc, &QueryMsg::Ownership {});
    assert_eq!(owner_res.owner, Some(suite.owner().clone()));

    // Verify processor
    let processor_addr: Addr = suite.query_wasm(&svc, &QueryMsg::GetProcessor {});
    assert_eq!(processor_addr, suite.processor().clone());

    // Verify service config
    let svc_cfg: Config = suite.query_wasm(&svc, &QueryMsg::GetServiceConfig {});
    assert_eq!(svc_cfg, Config {});
}

#[test]
fn execute_action() {
    let mut suite = TemplateTestSuite::default();

    let cfg = suite.template_config();

    // Instantiate Template contract
    let svc = suite.template_init(&cfg);

    // Execute action
    suite.execute_noop(svc).unwrap();
}
