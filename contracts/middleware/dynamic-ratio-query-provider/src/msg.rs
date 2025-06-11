use std::collections::HashMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;
use cw_ownable::cw_ownable_execute;

#[cw_serde]
pub struct DenomSplitMap {
    pub split_cfg: HashMap<String, HashMap<String, Decimal>>,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub split_cfg: DenomSplitMap,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateRatios { split_cfg: DenomSplitMap },
}
