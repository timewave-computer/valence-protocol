use std::collections::HashMap;

use anyhow::anyhow;
use bech32::{encode, primitives::decode::CheckedHrpstring, Bech32, Hrp};
use cosmwasm_std::CanonicalAddr;

use crate::config::{ConfigResult, GLOBAL_CONFIG};

fn validate_length(bytes: &[u8]) -> Result<(), anyhow::Error> {
    match bytes.len() {
        1..=255 => Ok(()),
        _ => Err(anyhow!("Invalid canonical address length")),
    }
}

pub fn addr_canonicalize(prefix: &str, input: &str) -> Result<CanonicalAddr, anyhow::Error> {
    let hrp_str =
        CheckedHrpstring::new::<Bech32>(input).map_err(|_| anyhow!("Error decoding bech32"))?;

    if !hrp_str
        .hrp()
        .as_bytes()
        .eq_ignore_ascii_case(prefix.as_bytes())
    {
        return Err(anyhow!("Wrong bech32 prefix"));
    }

    let bytes: Vec<u8> = hrp_str.byte_iter().collect();
    validate_length(&bytes)?;
    Ok(bytes.into())
}

pub fn addr_humanize(prefix: &str, canonical: &CanonicalAddr) -> Result<String, anyhow::Error> {
    validate_length(canonical.as_ref())?;

    let prefix = Hrp::parse(prefix).map_err(|_| anyhow!("Invalid bech32 prefix"))?;
    encode::<Bech32>(prefix, canonical.as_slice()).map_err(|_| anyhow!("Bech32 encoding error"))
}

<<<<<<< HEAD:workflow-manager/src/helpers.rs
// /// Helper for serde default for bool
// pub(crate) fn bool_true_default() -> bool {
//     true
// }
=======
pub async fn get_polytone_info(
    main_chain: &str,
    other_chain: &str,
) -> ConfigResult<HashMap<String, crate::bridge::PolytoneSingleChainInfo>> {
    let gc = GLOBAL_CONFIG.lock().await;
    // get from neutron to current domain bridge info
    Ok(gc
        .get_bridge_info(main_chain, other_chain)?
        .get_polytone_info()
        .clone())
}
>>>>>>> 0ceed756d867ffd33d4763d6734c405886661022:program-manager/src/helpers.rs
