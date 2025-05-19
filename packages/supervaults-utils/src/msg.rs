use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
pub enum ExecuteMsg {
    // deposit funds to use for market making
    Deposit {},
    // withdraw free unutilised funds
    Withdraw { amount: Uint128 },
    // Creates new AMM deposits using contract funds
    DexDeposit {},
    // Cancels and withdraws all active AMM deposits
    DexWithdrawal {},
}

#[cw_serde]
pub enum QueryMsg {
    GetDeposits {},
    GetConfig {},
    GetPrices {},
    GetBalance {},
}
