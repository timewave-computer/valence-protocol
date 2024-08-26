use bech32::{encode, Bech32, Hrp};
use cosmwasm_std::CanonicalAddr;

pub fn addr_humanize(prefix: &str, canonical: &CanonicalAddr) -> String {
    let hrp = Hrp::parse(prefix).unwrap();
    if let Ok(encoded) = encode::<Bech32>(hrp, canonical.as_slice()) {
        encoded
    } else {
        panic!("Invalid canonical address")
        // Err(StdError::generic_err("Invalid canonical address"))
    }
}
