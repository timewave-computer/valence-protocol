use std::str::FromStr;

use alloy_primitives::{address, Address, Bytes};
use alloy_sol_types::SolValue;
use cosmwasm_std::{Binary, StdError, StdResult};
use libraries::forwarder::{self};
use strum::EnumString;
use valence_authorization_utils::authorization::Subroutine;

pub mod contract;
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
pub enum EVMLibrary {
    // This one is reserved for when the user sends ABI raw bytes to a contract that is not one of our libraries
    NoLibrary,
    Forwarder,
}

impl EVMLibrary {
    /// Verifies that the library asked for is a valid library and returns it
    pub fn get_library(lib: &str) -> Result<Self, StdError> {
        // Parse the library enum using strum
        let library = lib
            .parse::<EVMLibrary>()
            .map_err(|_| StdError::generic_err("Invalid library".to_string()))?;

        Ok(library)
    }
    /// Validates if the provided library and function strings are valid
    /// `lib` is library name in snake_case (e.g. "forwarder")
    /// returns true if both library and function exist
    pub fn is_valid(lib: &str) -> bool {
        lib.parse::<EVMLibrary>().map_or(false, |function| {
            !matches!(function, EVMLibrary::NoLibrary)
        })
    }

    /// Encodes the provided message using the provided library and function strings
    pub fn encode_message(lib: &str, msg: &Binary) -> StdResult<Vec<u8>> {
        let library = Self::get_library(lib)?;
        match library {
            // When raw bytes are sent we don't do any checks here and just forward the message.
            EVMLibrary::NoLibrary => Ok(msg.to_vec()),
            EVMLibrary::Forwarder => forwarder::encode(msg),
        }
    }
}

/// Helper function that will ABI encode subroutines
fn encode_subroutine(subroutine: Subroutine) -> StdResult<solidity_types::Subroutine> {
    match subroutine {
        Subroutine::Atomic(atomic_subroutine) => {
            let mut functions: Vec<solidity_types::AtomicFunction> = Vec::new();

            // Process functions
            for function in atomic_subroutine.functions {
                functions.push(solidity_types::AtomicFunction {
                    contractAddress: Address::from_str(&function.contract_address.to_string()?)
                        .map_err(|e| StdError::generic_err(e.to_string()))?,
                });
            }

            // Encode the retry logic
            let retry_logic = encode_retry_logic(atomic_subroutine.retry_logic);

            let atomic_subroutine_encoded = solidity_types::AtomicSubroutine {
                functions,
                retryLogic: retry_logic,
            }
            .abi_encode();

            Ok(solidity_types::Subroutine {
                subroutineType: solidity_types::SubroutineType::Atomic,
                subroutine: atomic_subroutine_encoded.into(),
            })
        }
        Subroutine::NonAtomic(non_atomic_subroutine) => {
            let mut functions: Vec<solidity_types::NonAtomicFunction> = Vec::new();

            // Process functions
            for function in non_atomic_subroutine.functions {
                let callback_confirmation = match function.callback_confirmation {
                    None => solidity_types::FunctionCallback {
                        contractAddress: address!(),   // Address 0
                        callbackMessage: Bytes::new(), // Empty bytes
                    },
                    Some(callback) => solidity_types::FunctionCallback {
                        contractAddress: Address::from_str(callback.contract_address.as_ref())
                            .map_err(|e| StdError::generic_err(e.to_string()))?,
                        callbackMessage: Bytes::from(callback.callback_message.to_vec()),
                    },
                };

                functions.push(solidity_types::NonAtomicFunction {
                    contractAddress: Address::from_str(&function.contract_address.to_string()?)
                        .map_err(|e| StdError::generic_err(e.to_string()))?,
                    retryLogic: encode_retry_logic(function.retry_logic),
                    callbackConfirmation: callback_confirmation,
                });
            }

            let non_atomic_subroutine_encoded =
                solidity_types::NonAtomicSubroutine { functions }.abi_encode();

            Ok(solidity_types::Subroutine {
                subroutineType: solidity_types::SubroutineType::NonAtomic,
                subroutine: non_atomic_subroutine_encoded.into(),
            })
        }
    }
}

fn encode_retry_logic(
    retry_logic: Option<valence_authorization_utils::function::RetryLogic>,
) -> solidity_types::RetryLogic {
    if let Some(retry_logic) = retry_logic {
        let times = match retry_logic.times {
            valence_authorization_utils::function::RetryTimes::Indefinitely => {
                solidity_types::RetryTimes {
                    retryType: solidity_types::RetryTimesType::Indefinitely,
                    amount: 0,
                }
            }
            valence_authorization_utils::function::RetryTimes::Amount(amount) => {
                solidity_types::RetryTimes {
                    retryType: solidity_types::RetryTimesType::Amount,
                    amount,
                }
            }
        };

        let interval = match retry_logic.interval {
            cw_utils::Duration::Height(blocks) => solidity_types::Duration {
                durationType: solidity_types::DurationType::Height,
                value: blocks,
            },
            cw_utils::Duration::Time(seconds) => solidity_types::Duration {
                durationType: solidity_types::DurationType::Time,
                value: seconds,
            },
        };

        solidity_types::RetryLogic { times, interval }
    } else {
        // Default retry logic when none is provided
        solidity_types::RetryLogic {
            times: solidity_types::RetryTimes {
                retryType: solidity_types::RetryTimesType::NoRetry,
                amount: 0,
            },
            interval: solidity_types::Duration {
                durationType: solidity_types::DurationType::Time,
                value: 0,
            },
        }
    }
}
