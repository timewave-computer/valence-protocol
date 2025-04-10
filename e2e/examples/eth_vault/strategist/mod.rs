use std::{error::Error, str::FromStr};

use alloy::primitives::U256;
use cosmwasm_std::{Uint128, Uint256};

mod astroport;
pub mod client;
mod routing;
mod vault;

pub fn u256_to_uint256(u: U256) -> Result<Uint256, Box<dyn Error>> {
    let uint256 = Uint256::from_str(&u.to_string()).unwrap();
    Ok(uint256)
}
