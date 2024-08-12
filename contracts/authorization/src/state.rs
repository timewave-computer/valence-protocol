use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::Map;

/// Top level storage key. Values must not conflict.
/// Each key is only one byte long to ensure we use the smallest possible storage keys.
#[repr(u8)]
pub enum TopKey {
    SubOwners = b'0',
}

impl TopKey {
    const fn as_str(&self) -> &str {
        let array_ref = unsafe { std::mem::transmute::<&TopKey, &[u8; 1]>(self) };
        match core::str::from_utf8(array_ref) {
            Ok(a) => a,
            Err(_) => panic!("Non-utf8 enum value found. Use a-z, A-Z and 0-9"),
        }
    }
}

pub const SUB_OWNERS: Map<Addr, Empty> = Map::new(TopKey::SubOwners.as_str());
