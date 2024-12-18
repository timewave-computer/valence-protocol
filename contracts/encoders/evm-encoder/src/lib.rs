use cosmwasm_std::Binary;
use libraries::forwarder::ForwarderFunction;
use strum::EnumString;

pub mod contract;
pub mod error;
pub mod evict_msgs;
pub mod insert_msgs;
pub mod libraries;
pub mod pause;
pub mod resume;
pub mod send_msgs;
pub mod solidity_types;

#[cfg(test)]
mod tests;

#[derive(Debug, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum EVMLibraryFunction {
    Forwarder(ForwarderFunction),
}

impl EVMLibraryFunction {
    /// Returns the appropriate encoder based on the provided library and function strings
    pub fn get_function(lib: &str, func: &str) -> Result<Box<dyn Encode>, String> {
        // Parse the library enum using strum
        let library = lib
            .parse::<EVMLibraryFunction>()
            .map_err(|_| "Invalid library".to_string())?;

        // Get the appropriate function encoder based on the library type
        let encoder: Box<dyn Encode> = match library {
            EVMLibraryFunction::Forwarder(_) => {
                let f = func
                    .parse::<ForwarderFunction>()
                    .map_err(|_| "Invalid forwarder function".to_string())?;
                Box::new(f)
            }
        };

        Ok(encoder)
    }
    /// Validates if the provided library and function strings are valid
    /// `lib` is library name in snake_case (e.g. "forwarder") and `func` is the function name in snake_case (e.g. "forward")
    /// returns true if both library and function exist
    pub fn is_valid(lib: &str, func: &str) -> bool {
        Self::get_function(lib, func).is_ok()
    }

    /// Encodes the provided message using the provided library and function strings
    pub fn encode_message(lib: &str, func: &str, msg: Binary) -> Result<Binary, String> {
        let library = Self::get_function(lib, func)?;
        Ok(library.encode(msg))
    }
}

/// Trait for encoding library function calls
pub trait Encode {
    /// Encodes the function call with the provided message
    /// `msg` is the message to be encoded and returns the encoded binary data
    fn encode(&self, msg: Binary) -> Binary;
}
