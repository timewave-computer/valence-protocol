use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

use crate::xyk::ValenceXykPool;

#[cw_serde]
pub struct InstantiateMsg {}
#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// serialize a message to binary
    #[returns(Binary)]
    Serialize { obj: ValenceXykPool },
    /// deserialize a message from binary/bytes
    #[returns(ValenceXykPool)]
    Deserialize { type_url: String, binary: Binary },
    // TODO: transform an outdated type to a new version
}
