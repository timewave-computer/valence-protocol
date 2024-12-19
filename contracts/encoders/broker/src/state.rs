use cosmwasm_std::Addr;
use cw_storage_plus::Map;

// Encoders that we can redirect the encoding petitions to, key is a version and value is the address of the encoder contract
pub const ENCODERS: Map<String, Addr> = Map::new("encoders");
