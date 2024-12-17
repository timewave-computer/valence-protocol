use cosmwasm_std::Binary;
use strum::EnumString;

use crate::Encode;

#[derive(Debug, EnumString, Default)]
#[strum(serialize_all = "snake_case")]
pub enum ForwarderFunction {
    #[default]
    Forward,
}

impl Encode for ForwarderFunction {
    fn encode(&self, _msg: Binary) -> Binary {
        todo!()
    }
}
