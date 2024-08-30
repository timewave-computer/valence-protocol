use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};
use cw_utils::Duration;

use crate::{authorization_message::MessageDetails, domain::Domain};

#[cw_serde]
pub struct Action {
    // Note: for V1, all actions will be executed in the same domain
    pub domain: Domain,
    pub message_details: MessageDetails,
    // We use String instead of Addr because it can be a contract address in other execution environments
    pub contract_address: String,
    // Only applicable for NonAtomic execution type batches. An action might need to be retried, in that case we will include the retry logic.
    pub retry_logic: Option<RetryLogic>,
    // Only applicable for NonAtomic execution type batches. An action might need to receive a callback to be confirmed, in that case we will include the callback confirmation.
    // If not provided, we assume that correct execution of the message implies confirmation.
    pub callback_confirmation: Option<ActionCallback>,
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
