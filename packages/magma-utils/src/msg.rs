use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct DepositMsg {
    pub amount0_min: Uint128,
    pub amount1_min: Uint128,
    pub to: String, // Addr to mint shares to.
}

#[cw_serde]
pub struct WithdrawMsg {
    pub shares: Uint128,
    pub amount0_min: Uint128,
    pub amount1_min: Uint128,
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
    pub balance: Uint128,
}
