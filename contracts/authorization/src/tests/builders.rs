use authorization_utils::{
    action::{Action, ActionCallback, RetryLogic},
    authorization::{
        ActionBatch, AuthorizationDuration, AuthorizationInfo, AuthorizationMode, ExecutionType,
        Priority,
    },
    domain::{CallbackProxy, Connector, Domain, ExternalDomain},
    message::{Message, MessageDetails, MessageType},
};
use cosmwasm_std::{coins, Addr};
use neutron_test_tube::{Account, NeutronTestApp, SigningAccount};

const FEE_DENOM: &str = "untrn";

pub struct NeutronTestAppBuilder {
    external_domain: String,
    fee_denom: String,
    initial_balance: u128,
    num_accounts: u64,
}

impl NeutronTestAppBuilder {
    pub fn new() -> Self {
        NeutronTestAppBuilder {
            external_domain: "osmosis".to_string(),
            fee_denom: FEE_DENOM.to_string(),
            initial_balance: 100_000_000_000,
            num_accounts: 6,
        }
    }

    pub fn with_num_accounts(mut self, num_accounts: u64) -> Self {
        self.num_accounts = num_accounts;
        self
    }

    pub fn build(self) -> Result<NeutronTestAppSetup, &'static str> {
        let app = NeutronTestApp::new();

        if self.num_accounts < 6 {
            return Err("Number of accounts must be at least 6");
        }

        let accounts = app
            .init_accounts(
                &coins(self.initial_balance, &self.fee_denom),
                self.num_accounts,
            )
            .map_err(|_| "Failed to initialize accounts")?;

        let owner = &accounts[0];
        let subowner = &accounts[1];
        let user = &accounts[2];
        let processor = &accounts[3];
        let connector = &accounts[4];
        let callback_proxy = &accounts[5];

        let owner_addr = Addr::unchecked(owner.address());
        let subowner_addr = Addr::unchecked(subowner.address());
        let processor_addr = Addr::unchecked(processor.address());
        let connector_addr = Addr::unchecked(connector.address());
        let callback_proxy_addr = Addr::unchecked(callback_proxy.address());
        let user_addr = Addr::unchecked(user.address());

        let external_domain = ExternalDomain {
            name: self.external_domain,
            connector: Connector::PolytoneNote(connector_addr),
            processor: "processor".to_string(),
            callback_proxy: CallbackProxy::PolytoneProxy(callback_proxy_addr),
        };

        Ok(NeutronTestAppSetup {
            app,
            accounts,
            external_domain,
            owner_addr,
            subowner_addr,
            user_addr,
            processor_addr,
        })
    }
}

pub struct NeutronTestAppSetup {
    pub app: NeutronTestApp,
    pub accounts: Vec<SigningAccount>,
    pub external_domain: ExternalDomain,
    pub owner_addr: Addr,
    pub subowner_addr: Addr,
    pub user_addr: Addr,
    pub processor_addr: Addr,
}

pub struct AuthorizationBuilder {
    label: String,
    mode: AuthorizationMode,
    duration: AuthorizationDuration,
    max_concurrent_executions: Option<u64>,
    action_batch: ActionBatch,
    priority: Option<Priority>,
}

impl AuthorizationBuilder {
    pub fn new() -> Self {
        AuthorizationBuilder {
            label: "authorization".to_string(),
            mode: AuthorizationMode::Permissionless,
            duration: AuthorizationDuration::Forever,
            max_concurrent_executions: None,
            action_batch: ActionBatchBuilder::new().build(),
            priority: None,
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    pub fn with_mode(mut self, mode: AuthorizationMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_duration(mut self, duration: AuthorizationDuration) -> Self {
        self.duration = duration;
        self
    }

    pub fn with_max_concurrent_executions(mut self, max_concurrent_executions: u64) -> Self {
        self.max_concurrent_executions = Some(max_concurrent_executions);
        self
    }

    pub fn with_action_batch(mut self, action_batch: ActionBatch) -> Self {
        self.action_batch = action_batch;
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn build(self) -> AuthorizationInfo {
        AuthorizationInfo {
            label: self.label,
            mode: self.mode,
            duration: self.duration,
            max_concurrent_executions: self.max_concurrent_executions,
            action_batch: self.action_batch,
            priority: self.priority,
        }
    }
}

pub struct ActionBatchBuilder {
    execution_type: ExecutionType,
    actions: Vec<Action>,
}

impl ActionBatchBuilder {
    pub fn new() -> Self {
        ActionBatchBuilder {
            execution_type: ExecutionType::Atomic,
            actions: vec![],
        }
    }

    pub fn with_execution_type(mut self, execution_type: ExecutionType) -> Self {
        self.execution_type = execution_type;
        self
    }

    pub fn with_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    pub fn build(self) -> ActionBatch {
        ActionBatch {
            execution_type: self.execution_type,
            actions: self.actions,
        }
    }
}

pub struct ActionBuilder {
    domain: Domain,
    message_details: MessageDetails,
    contract_address: String,
    retry_logic: Option<RetryLogic>,
    callback_confirmation: Option<ActionCallback>,
}

impl ActionBuilder {
    pub fn new() -> Self {
        ActionBuilder {
            domain: Domain::Main,
            message_details: MessageDetails {
                message_type: MessageType::ExecuteMsg,
                message: Message {
                    name: "method".to_string(),
                    params_restrictions: None,
                },
            },
            contract_address: "address".to_string(),
            retry_logic: None,
            callback_confirmation: None,
        }
    }

    pub fn with_domain(mut self, domain: Domain) -> Self {
        self.domain = domain;
        self
    }

    pub fn with_message_details(mut self, message_details: MessageDetails) -> Self {
        self.message_details = message_details;
        self
    }

    pub fn with_retry_logic(mut self, retry_logic: RetryLogic) -> Self {
        self.retry_logic = Some(retry_logic);
        self
    }

    pub fn with_callback_confirmation(mut self, callback_confirmation: ActionCallback) -> Self {
        self.callback_confirmation = Some(callback_confirmation);
        self
    }

    pub fn build(self) -> Action {
        Action {
            domain: self.domain,
            message_details: self.message_details,
            contract_address: self.contract_address,
            retry_logic: self.retry_logic,
            callback_confirmation: self.callback_confirmation,
        }
    }
}
