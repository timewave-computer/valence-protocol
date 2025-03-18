use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdError, StdResult};

use crate::Id;

pub const LIBRARY_ACCOUNT_RAW_PLACEHOLDER: &str = "|lib_acc_placeholder|";

/// An helper type that is used to associate an account or library with an id
/// When a program is not instantiated yet, ids will be used to reference accounts and libraries
/// When a program is instantiated, the ids will be replaced by the instantiated addresses
#[cw_serde]
#[derive(Eq, PartialOrd, Ord)]
pub enum LibraryAccountType {
    #[serde(rename = "|library_account_addr|", alias = "library_account_addr")]
    Addr(String),
    #[serde(rename = "|account_id|", alias = "account_id")]
    AccountId(Id),
    #[serde(rename = "|library_id|", alias = "library_id")]
    LibraryId(Id),
}

impl LibraryAccountType {
    /// Returns the address as string if it is an address, otherwise returns an error
    pub fn to_string(&self) -> StdResult<String> {
        match self {
            LibraryAccountType::Addr(addr) => Ok(addr.to_string()),
            LibraryAccountType::AccountId(_) | LibraryAccountType::LibraryId(_) => Err(
                StdError::generic_err("LibraryAccountType must be an address"),
            ),
        }
    }

    /// Returns the address as Addr if it is an address, otherwise returns an error
    pub fn to_addr(&self, api: &dyn cosmwasm_std::Api) -> StdResult<Addr> {
        match self {
            LibraryAccountType::Addr(addr) => api.addr_validate(addr),
            LibraryAccountType::AccountId(_) | LibraryAccountType::LibraryId(_) => Err(
                StdError::generic_err("LibraryAccountType must be an address"),
            ),
        }
    }

    /// There are cases where a library config expects a string, but we still want to use the
    /// id replacement functionality of the manager.
    /// Using this function will use a placeholder that can be replaced by the manager
    /// to the instantiated address
    pub fn to_raw_placeholder(&self) -> String {
        let value = match self {
            // If its an address, we can use it directly
            LibraryAccountType::Addr(addr) => return addr.to_string(),
            LibraryAccountType::AccountId(id) => id.to_string(),
            LibraryAccountType::LibraryId(_) => {
                panic!("Only accounts can use raw_placeholder functionality")
            }
        };

        format!("{}:{}", LIBRARY_ACCOUNT_RAW_PLACEHOLDER, value)
    }
}

impl From<&Addr> for LibraryAccountType {
    fn from(addr: &Addr) -> Self {
        LibraryAccountType::Addr(addr.to_string())
    }
}

impl From<&str> for LibraryAccountType {
    fn from(input: &str) -> Self {
        if input.starts_with("{\"|account_id|\":") {
            LibraryAccountType::AccountId(
                input
                    .trim_start_matches("{\"|account_id|\":")
                    .trim_end_matches("}")
                    .parse()
                    .unwrap(),
            )
        } else if input.starts_with("{\"|library_id|\":") {
            LibraryAccountType::LibraryId(
                input
                    .trim_start_matches("{\"|library_id|\":")
                    .trim_end_matches("}")
                    .parse()
                    .unwrap(),
            )
        } else if input.starts_with("{\"|library_account_addr|\":\"") {
            LibraryAccountType::Addr(
                input
                    .trim_start_matches("{\"|library_account_addr|\":\"")
                    .trim_end_matches("\"}")
                    .parse()
                    .unwrap(),
            )
        } else {
            LibraryAccountType::Addr(input.to_string())
        }
    }
}

pub trait GetId {
    fn get_account_id(&self) -> Id;
    fn get_library_id(&self) -> Id;
}

impl GetId for LibraryAccountType {
    fn get_account_id(&self) -> Id {
        match self {
            LibraryAccountType::Addr(_) => {
                panic!("LibraryAccountType is an address")
            }
            LibraryAccountType::AccountId(id) => *id,
            LibraryAccountType::LibraryId(_) => panic!("LibraryAccountType is a library id"),
        }
    }

    fn get_library_id(&self) -> Id {
        match self {
            LibraryAccountType::Addr(_) => {
                panic!("LibraryAccountType is an address")
            }
            LibraryAccountType::AccountId(_) => panic!("LibraryAccountType is a account id"),
            LibraryAccountType::LibraryId(id) => *id,
        }
    }
}

impl GetId for u64 {
    fn get_account_id(&self) -> Id {
        *self
    }

    fn get_library_id(&self) -> Id {
        *self
    }
}

impl GetId for &u64 {
    fn get_account_id(&self) -> Id {
        **self
    }

    fn get_library_id(&self) -> Id {
        **self
    }
}

impl GetId for u32 {
    fn get_account_id(&self) -> Id {
        (*self).into()
    }

    fn get_library_id(&self) -> Id {
        (*self).into()
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::to_json_string;

    use super::LibraryAccountType;

    #[test]
    fn serde_serialize() {
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
        struct Helper {
            addr: LibraryAccountType,
        }

        let value = Helper {
            addr: LibraryAccountType::Addr("addr1234".to_string()),
        };

        let json = serde_json::ser::to_string(&value).unwrap();
        let ty: Helper = serde_json::from_str(json.as_str()).unwrap();

        assert_eq!(value, ty);
    }

    #[test]
    fn from_str() {
        let addr = LibraryAccountType::Addr("addr1234".to_string());
        let account = LibraryAccountType::AccountId(1);
        let library = LibraryAccountType::LibraryId(2);

        let addr_json_string = to_json_string(&addr).unwrap();
        let account_json_string = to_json_string(&account).unwrap();
        let library_json_string = to_json_string(&library).unwrap();

        let addr_type: LibraryAccountType = addr_json_string.as_str().into();
        let account_id_type: LibraryAccountType = account_json_string.as_str().into();
        let library_id_type: LibraryAccountType = library_json_string.as_str().into();

        assert_eq!(addr, addr_type);
        assert_eq!(account, account_id_type);
        assert_eq!(library, library_id_type);
    }

    #[test]
    #[should_panic]
    fn raw_placeholder() {
        let raw_addr = "addr1234".to_string();
        let addr = LibraryAccountType::Addr(raw_addr.clone());
        let account = LibraryAccountType::AccountId(1);

        // If type is Addr, we should get back the address directly
        assert_eq!(addr.to_raw_placeholder(), "addr1234");

        // If type account, we should get back the placeholder with the id in question
        let acc_placeholder = account.to_raw_placeholder();
        assert_eq!(
            acc_placeholder,
            format!("{}:1", super::LIBRARY_ACCOUNT_RAW_PLACEHOLDER)
        );
    }

    #[test]
    #[should_panic]
    fn raw_placeholder_library() {
        let library = LibraryAccountType::LibraryId(2);

        // Should panic if we try to get the raw placeholder of a library
        library.to_raw_placeholder();
    }
}
