use cw_utils::Expiration;
use serde_json::{json, Map, Value};
use valence_service_utils::ServiceAccountType;

use crate::{
    authorization::{
        AtomicSubroutine, AuthorizationDuration, AuthorizationInfo, AuthorizationModeInfo,
        NonAtomicSubroutine, Priority, Subroutine,
    },
    authorization_message::{Message, MessageDetails, MessageType},
    domain::Domain,
    function::{AtomicFunction, FunctionCallback, NonAtomicFunction, RetryLogic},
};

pub struct AuthorizationBuilder {
    label: String,
    mode: AuthorizationModeInfo,
    not_before: Expiration,
    duration: AuthorizationDuration,
    max_concurrent_executions: Option<u64>,
    subroutine: Subroutine,
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
            subroutine: Subroutine::Atomic(AtomicSubroutine {
                functions: vec![],
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

    pub fn with_subroutine(mut self, subroutine: Subroutine) -> Self {
        self.subroutine = subroutine;
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
            subroutine: self.subroutine,
            priority: self.priority,
        }
    }
}

pub struct AtomicSubroutineBuilder {
    functions: Vec<AtomicFunction>,
    retry_logic: Option<RetryLogic>,
}

impl Default for AtomicSubroutineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AtomicSubroutineBuilder {
    pub fn new() -> Self {
        AtomicSubroutineBuilder {
            functions: vec![],
            retry_logic: None,
        }
    }

    pub fn with_function(mut self, function: AtomicFunction) -> Self {
        self.functions.push(function);
        self
    }

    pub fn with_retry_logic(mut self, retry_logic: RetryLogic) -> Self {
        self.retry_logic = Some(retry_logic);
        self
    }

    pub fn build(self) -> Subroutine {
        Subroutine::Atomic(AtomicSubroutine {
            functions: self.functions,
            retry_logic: self.retry_logic,
        })
    }
}

pub struct NonAtomicSubroutineBuilder {
    functions: Vec<NonAtomicFunction>,
}

impl Default for NonAtomicSubroutineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NonAtomicSubroutineBuilder {
    pub fn new() -> Self {
        NonAtomicSubroutineBuilder { functions: vec![] }
    }

    pub fn with_function(mut self, function: NonAtomicFunction) -> Self {
        self.functions.push(function);
        self
    }

    pub fn build(self) -> Subroutine {
        Subroutine::NonAtomic(NonAtomicSubroutine {
            functions: self.functions,
        })
    }
}

pub struct AtomicFunctionBuilder {
    domain: Domain,
    message_details: MessageDetails,
    contract_address: ServiceAccountType,
}

impl Default for AtomicFunctionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AtomicFunctionBuilder {
    pub fn new() -> Self {
        AtomicFunctionBuilder {
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

    pub fn build(self) -> AtomicFunction {
        AtomicFunction {
            domain: self.domain,
            message_details: self.message_details,
            contract_address: self.contract_address,
        }
    }
}

pub struct NonAtomicFunctionBuilder {
    domain: Domain,
    message_details: MessageDetails,
    contract_address: String,
    retry_logic: Option<RetryLogic>,
    callback_confirmation: Option<FunctionCallback>,
}

impl Default for NonAtomicFunctionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NonAtomicFunctionBuilder {
    pub fn new() -> Self {
        NonAtomicFunctionBuilder {
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

    pub fn with_callback_confirmation(mut self, callback_confirmation: FunctionCallback) -> Self {
        self.callback_confirmation = Some(callback_confirmation);
        self
    }

    pub fn build(self) -> NonAtomicFunction {
        NonAtomicFunction {
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
