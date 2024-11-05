use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};
use cw_utils::Duration;
use valence_service_utils::ServiceAccountType;

use crate::{authorization_message::MessageDetails, domain::Domain};

#[cw_serde]
pub struct AtomicFunction {
    // Note: for V1, all functions will be executed in the same domain
    pub domain: Domain,
    pub message_details: MessageDetails,
    // We use String instead of Addr because it can be a contract address in other execution environments
    pub contract_address: ServiceAccountType,
}

#[cw_serde]
pub struct NonAtomicFunction {
    // Note: for V1, all functions will be executed in the same domain
    pub domain: Domain,
    pub message_details: MessageDetails,
    // We use String instead of Addr because it can be a contract address in other execution environments
    pub contract_address: ServiceAccountType,
    // A non atomic function might need to be retried, in that case we will include the retry logic.
    pub retry_logic: Option<RetryLogic>,
    // An function might need to receive a callback to be confirmed, in that case we will include the callback confirmation.
    // If not provided, we assume that correct execution of the message implies confirmation.
    pub callback_confirmation: Option<FunctionCallback>,
}

pub trait Function {
    fn domain(&self) -> &Domain;
    fn message_details(&self) -> &MessageDetails;
    fn get_contract_address(&self) -> String;
}

// Implement this trait for both AtomicFunction and NonAtomicFunction
impl Function for AtomicFunction {
    fn domain(&self) -> &Domain {
        &self.domain
    }

    fn message_details(&self) -> &MessageDetails {
        &self.message_details
    }

    fn get_contract_address(&self) -> String {
        self.contract_address.to_string().unwrap()
    }
}

impl Function for NonAtomicFunction {
    fn domain(&self) -> &Domain {
        &self.domain
    }

    fn message_details(&self) -> &MessageDetails {
        &self.message_details
    }

    fn get_contract_address(&self) -> String {
        self.contract_address.to_string().unwrap()
    }
}

#[cw_serde]
pub struct RetryLogic {
    pub times: RetryTimes,
    pub interval: Duration,
}

#[cw_serde]
pub enum RetryTimes {
    Indefinitely,
    Amount(u64),
}

#[cw_serde]
pub struct FunctionCallback {
    // Address of contract we should receive the Callback from
    pub contract_address: Addr,
    // What we should receive from the callback to consider the function completed
    pub callback_message: Binary,
}
