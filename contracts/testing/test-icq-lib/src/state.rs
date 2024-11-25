use cosmwasm_schema::cw_serde;
use cw_storage_plus::{Item, Map};

/// contains all transfers mapped by a recipient address observed by the contract.
pub const RECIPIENT_TXS: Map<String, Vec<Transfer>> = Map::new("recipient_txs");
/// contains number of transfers to addresses observed by the contract.
pub const TRANSFERS: Item<u64> = Item::new("transfers");

#[cw_serde]
pub struct Transfer {
    pub recipient: String,
    pub sender: String,
    pub denom: String,
    pub amount: String,
}

#[cw_serde]
pub struct OpenAckVersion {
    pub version: String,
    pub controller_connection_id: String,
    pub host_connection_id: String,
    pub address: String,
    pub encoding: String,
    pub tx_type: String,
}
