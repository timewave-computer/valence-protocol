use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct ExternalDomain {
    pub name: String,
    pub connector: Connector,
    pub processor: String,
    pub callback_proxy: CallBackProxy,
}

#[cw_serde]
pub enum Connector {
    PolytoneNote(Addr),
}

#[cw_serde]
pub enum CallBackProxy {
    PolytoneProxy(Addr),
}
