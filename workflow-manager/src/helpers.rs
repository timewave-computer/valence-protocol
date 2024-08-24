use bech32::{encode, Bech32, Hrp};
use cosmwasm_std::{Addr, CanonicalAddr};

pub fn addr_humanize(prefix: &str, canonical: &CanonicalAddr) -> Addr {
    let hrp = Hrp::parse(prefix).unwrap();
    if let Ok(encoded) = encode::<Bech32>(hrp, canonical.as_slice()) {
        Addr::unchecked(encoded)
    } else {
        panic!("Invalid canonical address")
        // Err(StdError::generic_err("Invalid canonical address"))
    }
}
