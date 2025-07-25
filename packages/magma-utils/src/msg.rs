use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct DepositMsg {
    pub amount0_min: String,
    pub amount1_min: String,
    pub to: String, // Addr to mint shares to.
}

#[cw_serde]
pub struct WithdrawMsg {
    pub shares: String,
    pub amount0_min: String,
    pub amount1_min: String,
    pub to: String,
}
#[cw_serde]
pub enum ExecuteMsg {
    Deposit(DepositMsg),

    Withdraw(WithdrawMsg),
}

#[cw_serde]
pub enum QueryMsg {
    Balance { address: String },
}

#[cw_serde]
pub struct BalanceResponse {
    pub balance: String,
}
