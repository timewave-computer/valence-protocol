use cw_utils::Expiration;
use serde_json::{json, Map, Value};
use valence_service_utils::ServiceAccountType;

use crate::{
    action::{ActionCallback, AtomicAction, NonAtomicAction, RetryLogic},
    authorization::{
        ActionsConfig, AtomicActionsConfig, AuthorizationDuration, AuthorizationInfo,
        AuthorizationModeInfo, NonAtomicActionsConfig, Priority,
    },
    authorization_message::{Message, MessageDetails, MessageType},
    domain::Domain,
};

pub struct AuthorizationBuilder {
    label: String,
    mode: AuthorizationModeInfo,
    not_before: Expiration,
    duration: AuthorizationDuration,
    max_concurrent_executions: Option<u64>,
    actions_config: ActionsConfig,
    priority: Option<Priority>,
}

impl Default for AuthorizationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthorizationBuilder {
    pub fn new() -> Self {
        AuthorizationBuilder {
            label: "authorization".to_string(),
            mode: AuthorizationModeInfo::Permissionless,
            not_before: Expiration::Never {},
            duration: AuthorizationDuration::Forever,
            max_concurrent_executions: None,
            actions_config: ActionsConfig::Atomic(AtomicActionsConfig {
                actions: vec![],
                retry_logic: None,
            }),
            priority: None,
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    pub fn with_mode(mut self, mode: AuthorizationModeInfo) -> Self {
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

    pub fn with_actions_config(mut self, actions_config: ActionsConfig) -> Self {
        self.actions_config = actions_config;
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
            actions_config: self.actions_config,
            priority: self.priority,
        }
    }
}

pub struct AtomicActionsConfigBuilder {
    actions: Vec<AtomicAction>,
    retry_logic: Option<RetryLogic>,
}

impl Default for AtomicActionsConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AtomicActionsConfigBuilder {
    pub fn new() -> Self {
        AtomicActionsConfigBuilder {
            actions: vec![],
            retry_logic: None,
        }
    }

    pub fn with_action(mut self, action: AtomicAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn with_retry_logic(mut self, retry_logic: RetryLogic) -> Self {
        self.retry_logic = Some(retry_logic);
        self
    }

    pub fn build(self) -> ActionsConfig {
        ActionsConfig::Atomic(AtomicActionsConfig {
            actions: self.actions,
            retry_logic: self.retry_logic,
        })
    }
}

pub struct NonAtomicActionsConfigBuilder {
    actions: Vec<NonAtomicAction>,
}

impl Default for NonAtomicActionsConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NonAtomicActionsConfigBuilder {
    pub fn new() -> Self {
        NonAtomicActionsConfigBuilder { actions: vec![] }
    }

    pub fn with_action(mut self, action: NonAtomicAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn build(self) -> ActionsConfig {
        ActionsConfig::NonAtomic(NonAtomicActionsConfig {
            actions: self.actions,
        })
    }
}

pub struct AtomicActionBuilder {
    domain: Domain,
    message_details: MessageDetails,
    contract_address: ServiceAccountType,
}

impl Default for AtomicActionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AtomicActionBuilder {
    pub fn new() -> Self {
        AtomicActionBuilder {
            domain: Domain::Main,
            message_details: MessageDetails {
                message_type: MessageType::CosmwasmExecuteMsg,
                message: Message {
                    name: "method".to_string(),
                    params_restrictions: None,
                },
            },
            contract_address: ServiceAccountType::Addr("address".to_string()),
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

    pub fn with_contract_address(mut self, contract_address: ServiceAccountType) -> Self {
        self.contract_address = contract_address;
        self
    }

    pub fn build(self) -> AtomicAction {
        AtomicAction {
            domain: self.domain,
            message_details: self.message_details,
            contract_address: self.contract_address,
        }
    }
}

pub struct NonAtomicActionBuilder {
    domain: Domain,
    message_details: MessageDetails,
    contract_address: String,
    retry_logic: Option<RetryLogic>,
    callback_confirmation: Option<ActionCallback>,
}

impl Default for NonAtomicActionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NonAtomicActionBuilder {
    pub fn new() -> Self {
        NonAtomicActionBuilder {
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

    pub fn build(self) -> NonAtomicAction {
        NonAtomicAction {
            domain: self.domain,
            message_details: self.message_details,
            contract_address: self.contract_address.as_str().into(),
            retry_logic: self.retry_logic,
            callback_confirmation: self.callback_confirmation,
        }
    }
}

pub struct JsonBuilder {
    main: String,
    data: Value,
}

impl Default for JsonBuilder {
    fn default() -> Self {
        Self::new()
    }
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
