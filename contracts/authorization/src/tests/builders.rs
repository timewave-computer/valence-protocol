use cosmwasm_std::{coins, Addr};
use cw_utils::Expiration;
use neutron_test_tube::{Account, NeutronTestApp, SigningAccount};
use serde_json::{json, Map, Value};
use valence_authorization_utils::{
    action::{Action, ActionCallback, RetryLogic},
    authorization::{
        ActionBatch, AuthorizationDuration, AuthorizationInfo, AuthorizationMode, ExecutionType,
        Priority,
    },
    authorization_message::{Message, MessageDetails, MessageType},
    domain::{CallbackProxy, Connector, Domain, ExecutionEnvironment, ExternalDomain},
};

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
            num_accounts: 5,
        }
    }

    pub fn with_num_accounts(mut self, num_accounts: u64) -> Self {
        self.num_accounts = num_accounts;
        self
    }

    pub fn build(self) -> Result<NeutronTestAppSetup, &'static str> {
        let app = NeutronTestApp::new();

        if self.num_accounts < 5 {
            return Err("Number of accounts must be at least 5");
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
        let connector = &accounts[3];
        let callback_proxy = &accounts[4];

        let owner_addr = Addr::unchecked(owner.address());
        let subowner_addr = Addr::unchecked(subowner.address());
        let connector_addr = Addr::unchecked(connector.address());
        let callback_proxy_addr = Addr::unchecked(callback_proxy.address());
        let user_addr = Addr::unchecked(user.address());

        let external_domain = ExternalDomain {
            name: self.external_domain,
            execution_environment: ExecutionEnvironment::CosmWasm,
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
}

pub struct AuthorizationBuilder {
    label: String,
    mode: AuthorizationMode,
    not_before: Expiration,
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
            not_before: Expiration::Never {},
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

    pub fn with_not_before(mut self, not_before: Expiration) -> Self {
        self.not_before = not_before;
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
            not_before: self.not_before,
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
    retry_logic: Option<RetryLogic>,
}

impl ActionBatchBuilder {
    pub fn new() -> Self {
        ActionBatchBuilder {
            execution_type: ExecutionType::Atomic,
            actions: vec![],
            retry_logic: None,
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

    pub fn with_retry_logic(mut self, retry_logic: RetryLogic) -> Self {
        self.retry_logic = Some(retry_logic);
        self
    }

    pub fn build(self) -> ActionBatch {
        ActionBatch {
            execution_type: self.execution_type,
            actions: self.actions,
            retry_logic: self.retry_logic,
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
                message_type: MessageType::CosmwasmExecuteMsg,
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

    pub fn with_contract_address(mut self, contract_address: &str) -> Self {
        self.contract_address = contract_address.to_string();
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

pub struct JsonBuilder {
    main: String,
    data: Value,
}

impl JsonBuilder {
    pub fn new() -> Self {
        JsonBuilder {
            main: String::new(),
            data: Value::Object(Map::new()),
        }
    }

    pub fn main(mut self, main: &str) -> Self {
        self.main = main.to_string();
        self
    }

    pub fn add(mut self, path: &str, value: Value) -> Self {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &mut self.data;

        for (i, &part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                if let Value::Object(map) = current {
                    map.insert(part.to_string(), value.clone());
                }
            } else {
                current = current
                    .as_object_mut()
                    .map(|map| map.entry(part.to_string()).or_insert(json!({})))
                    .expect("Failed to insert or access object");
            }
        }
        self
    }

    pub fn build(self) -> Value {
        if self.main.is_empty() {
            self.data
        } else {
            json!({ self.main: self.data })
        }
    }
}
