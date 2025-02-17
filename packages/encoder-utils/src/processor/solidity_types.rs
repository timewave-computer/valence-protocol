use alloy_sol_types::sol;

// The types below is what our processor messages will be decoded into on the EVM processor, with all the required information to build the batches
// and apply the logic
sol! {
    struct ProcessorMessage {
        ProcessorMessageType messageType;
        bytes message; // ABI encoded message according to the type
    }

    enum ProcessorMessageType {
        Pause,
        Resume,
        EvictMsgs,
        SendMsgs,
        InsertMsgs
    }

    enum Priority {
        Medium,
        High,
    }

    enum SubroutineType {
        Atomic,
        NonAtomic
    }

    struct Subroutine {
        SubroutineType subroutineType;
        bytes subroutine; // ABI encoded AtomicSubroutine or NonAtomicSubroutine
    }

    struct AtomicSubroutine {
        AtomicFunction[] functions;
        RetryLogic retryLogic;
    }

    struct AtomicFunction {
        address contractAddress;
    }

    enum DurationType {
        Height,
        Time
    }

    struct Duration {
        DurationType durationType;
        uint64 value;
    }

    enum RetryTimesType {
        NoRetry,
        Indefinitely,
        Amount
    }

    struct RetryTimes {
        RetryTimesType retryType;
        uint64 amount;  // Only used when retryType is Amount otherwise will be 0
    }

    struct RetryLogic {
        RetryTimes times;
        Duration interval; // If there's no retry, this field will be mapped to Time(0)
    }

    struct NonAtomicSubroutine {
        NonAtomicFunction[] functions;
    }

    struct NonAtomicFunction {
        address contractAddress;
        RetryLogic retryLogic;
        FunctionCallback callbackConfirmation;
    }

    struct FunctionCallback {
        address contractAddress;  // Set to address(0) if no callback
        bytes callbackMessage;    // Set to empty bytes if no callback
    }

    struct InsertMsgs {
        uint64 executionId;
        uint64 queuePosition;
        Priority priority;
        Subroutine subroutine;
        uint64 expirationTime;
        bytes[] messages; // ABI encoded messages
    }

    struct SendMsgs {
        uint64 executionId;
        Priority priority;
        Subroutine subroutine;
        uint64 expirationTime;
        bytes[] messages; // ABI encoded messages
    }

    struct EvictMsgs {
        uint64 queuePosition;
        Priority priority;
    }

    struct SubroutineResult {
        bool succeeded;
        bool expired;
        uint256 executedCount;
        bytes errorData;
    }

    struct Callback {
        uint64 executionId;
        ExecutionResult executionResult;
        uint256 executedCount;
        bytes data;
    }

    enum ExecutionResult {
        Success,
        Rejected,
        PartiallyExecuted,
        Expired
    }

    struct RejectedResult {
        bytes errorData;
    }

    struct PartiallyExecutedResult {
        uint256 executedCount;
        bytes errorData;
    }

    struct ExpiredResult {
        uint256 executedCount;
    }
}

impl From<valence_authorization_utils::authorization::Priority> for Priority {
    fn from(priority: valence_authorization_utils::authorization::Priority) -> Self {
        match priority {
            valence_authorization_utils::authorization::Priority::Medium => Priority::Medium,
            valence_authorization_utils::authorization::Priority::High => Priority::High,
        }
    }
}

// Convert a solidity callback into a callback for the authorization contract
impl From<Callback> for valence_authorization_utils::msg::InternalAuthorizationMsg {
    fn from(msg: Callback) -> Self {
        valence_authorization_utils::msg::InternalAuthorizationMsg::ProcessorCallback {
            execution_id: msg.executionId,
            execution_result: match msg.executionResult {
                ExecutionResult::Success => {
                    valence_authorization_utils::callback::ExecutionResult::Success
                }
                ExecutionResult::Rejected => {
                    valence_authorization_utils::callback::ExecutionResult::Rejected(
                        msg.data.to_string(),
                    )
                }
                ExecutionResult::PartiallyExecuted => {
                    valence_authorization_utils::callback::ExecutionResult::PartiallyExecuted(
                        msg.executedCount.to(),
                        msg.data.to_string(),
                    )
                }
                ExecutionResult::Expired => {
                    valence_authorization_utils::callback::ExecutionResult::Expired(
                        msg.executedCount.to(),
                    )
                }
                ExecutionResult::__Invalid => unreachable!(),
            },
        }
    }
}
