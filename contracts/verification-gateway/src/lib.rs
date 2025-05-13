use cosmwasm_schema::cw_serde;

pub mod contract;
pub mod error;
pub mod msg;

#[cw_serde]
pub enum VerifyingKey {
    SP1VerifyingKeyHash(String),
}
