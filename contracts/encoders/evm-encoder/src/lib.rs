use libraries::forwarder::ForwarderFunction;
use strum::EnumString;

pub mod contract;
pub mod error;
pub mod libraries;

#[cfg(test)]
mod tests;

#[derive(Debug, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum EVMLibraryFunction {
    Forwarder(ForwarderFunction),
}

impl EVMLibraryFunction {
    pub fn is_valid(lib: &str, func: &str) -> bool {
        // First check if the library exists
        match lib.parse::<EVMLibraryFunction>() {
            Ok(EVMLibraryFunction::Forwarder(_)) => {
                // If it's a Forwarder, validate against ForwarderFunction
                func.parse::<ForwarderFunction>().is_ok()
            }
            // Add more library matches here
            _ => false,
        }
    }
}
