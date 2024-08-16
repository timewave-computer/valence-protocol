#[derive(Debug, PartialEq, Clone)]
pub enum AccountType {
    Addr { addr: String },
    Base { admin: Option<String> },
}

#[derive(Debug, PartialEq, Clone)]
pub struct AccountInfo {
    pub ty: AccountType,
    pub domain: String,
}
