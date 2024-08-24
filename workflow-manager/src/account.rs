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
    pub ty: AccountType,
    pub domain: Domain,
}
