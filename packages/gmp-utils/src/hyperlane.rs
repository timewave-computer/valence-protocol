use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, Binary, CosmosMsg, HexBinary, StdResult, WasmMsg};

#[cw_serde]
pub enum HyperlaneExecuteMsg {
    Dispatch(DispatchMsg),
}

#[cw_serde]
pub struct DispatchMsg {
    pub dest_domain: u32,
    pub recipient_addr: HexBinary,
    pub msg_body: HexBinary,
    pub hook: Option<String>,
    pub metadata: Option<HexBinary>,
}

#[cw_serde]
#[derive(Default)]
pub struct HandleMsg {
    pub origin: u32,
    pub sender: HexBinary,
    pub body: HexBinary,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum IsmSpecifierQueryMsg {
    #[returns(InterchainSecurityModuleResponse)]
    InterchainSecurityModule(),
}

#[cw_serde]
pub struct InterchainSecurityModuleResponse {
    pub ism: Option<Addr>,
}

pub fn create_msg_for_hyperlane(
    mailbox: Addr,
    domain_id: u32,
    processor: String,
    execute_msg: Binary,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: mailbox.to_string(),
        msg: to_json_binary(&HyperlaneExecuteMsg::Dispatch(DispatchMsg {
            dest_domain: domain_id,
            recipient_addr: format_address_for_hyperlane(processor)?,
            msg_body: HexBinary::from(execute_msg.to_vec()),
            hook: None,
            metadata: None,
        }))?,
        funds: vec![],
    }))
}

/// Formats an address for Hyperlane by removing the "0x" prefix and padding it to 32 bytes
pub fn format_address_for_hyperlane(address: String) -> StdResult<HexBinary> {
    // Remove "0x" prefix if present
    let address_hex = address.trim_start_matches("0x").to_string().to_lowercase();
    // Pad to 32 bytes (64 hex characters) because mailboxes expect 32 bytes addresses with leading zeros
    let padded_address = format!("{address_hex:0>64}");
    // Convert to HexBinary which is what Hyperlane expects
    HexBinary::from_hex(&padded_address)
}
