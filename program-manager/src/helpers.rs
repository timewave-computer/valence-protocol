use std::collections::HashMap;

use crate::config::{ConfigResult, GLOBAL_CONFIG};

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
