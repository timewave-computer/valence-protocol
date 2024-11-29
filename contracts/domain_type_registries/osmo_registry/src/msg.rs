use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum QueryResult {
    Gamm { result_type: GammResultTypes },
    Bank { result_type: BankResultTypes },
}

#[cw_serde]
pub enum GammResultTypes {
    Pool,
}

#[cw_serde]
pub enum BankResultTypes {
    AccountDenomBalance,
}
