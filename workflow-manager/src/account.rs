use crate::domain::Domain;

/// What account type we talking about
#[derive(Debug, PartialEq, Clone, strum::Display)]
pub enum AccountType {
    /// This means the account is already instantiated
    Addr { addr: String },
    /// This our base account implementation
    #[strum(to_string = "base_account")]
    Base { admin: Option<String> },
}

/// The struct given to us by the user.
///
/// We need to know what domain we are talking with
/// and what type of account we should work with.
#[derive(Debug, PartialEq, Clone)]
pub struct AccountInfo {
    pub name: String,
    pub ty: AccountType,
    pub domain: Domain,
}

#[derive(Debug)]
pub struct InstantiateAccountData {
    pub id: u64,
    pub info: AccountInfo,
    pub addr: String,
    pub salt: Vec<u8>,
    pub approved_services: Vec<String>,
}

impl InstantiateAccountData {
    pub fn new(id: u64, info: AccountInfo, addr: String, salt: Vec<u8>) -> Self {
        Self {
            id,
            info,
            addr,
            salt,
            approved_services: vec![],
        }
    }

    pub fn add_service(&mut self, service_addr: String) {
        self.approved_services.push(service_addr);
    }
}
