// Since Nolus is using an older CosmWasm version, to make it compatible with our packages, we are going to redefine the messages here using Cosmwasm that we need
// for our library
// The content here is from https://github.com/nolus-protocol/nolus-money-market, which is the stable API for mars contracts

use cosmwasm_std::{Addr, Uint128};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub enum ExecuteMsg {
    Deposit(),
    // CW20 interface, withdraw from lender deposit
    Burn { amount: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub enum QueryMsg {
    // Deposit
    /// CW20 interface, lender deposit balance
    Balance { address: Addr },
}

// CW20 interface
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BalanceResponse {
    pub balance: Uint128,
}
