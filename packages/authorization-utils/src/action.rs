use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};
use cw_utils::Duration;
use valence_service_utils::ServiceAccountType;

use crate::{authorization_message::MessageDetails, domain::Domain};

#[cw_serde]
pub struct AtomicAction {
    // Note: for V1, all actions will be executed in the same domain
    pub domain: Domain,
    pub message_details: MessageDetails,
    // We use String instead of Addr because it can be a contract address in other execution environments
    pub contract_address: ServiceAccountType,
}

#[cw_serde]
pub struct NonAtomicAction {
    // Note: for V1, all actions will be executed in the same domain
    pub domain: Domain,
    pub message_details: MessageDetails,
    // We use String instead of Addr because it can be a contract address in other execution environments
    pub contract_address: ServiceAccountType,
    // A non atomic action might need to be retried, in that case we will include the retry logic.
    pub retry_logic: Option<RetryLogic>,
    // An action might need to receive a callback to be confirmed, in that case we will include the callback confirmation.
    // If not provided, we assume that correct execution of the message implies confirmation.
    pub callback_confirmation: Option<ActionCallback>,
}

pub trait Action {
    fn domain(&self) -> &Domain;
    fn message_details(&self) -> &MessageDetails;
    fn get_contract_address(&self) -> String;
}

// Implement this trait for both AtomicAction and NonAtomicAction
impl Action for AtomicAction {
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

impl Action for NonAtomicAction {
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
pub struct ActionCallback {
    // Address of contract we should receive the Callback from
    pub contract_address: Addr,
    // What we should receive from the callback to consider the action completed
    pub callback_message: Binary,
}
