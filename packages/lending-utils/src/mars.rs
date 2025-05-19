// Since Mars is using an old CosmWasm version, to make it compatible with our packages, we are going to redefine the messages here using Cosmwasm 2.x that we need
// for our library
// The content here is from https://github.com/mars-protocol/core-contracts, which is the stable API for Mars contracts

use cosmwasm_schema::cw_serde;
use cosmwasm_schema::QueryResponses;
use cosmwasm_std::{Coin, Uint128};

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<Account>)]
    Accounts {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct Account {
    pub id: String,
    pub kind: AccountKind,
}

#[cw_serde]
pub enum AccountKind {
    Default,
    HighLeveredStrategy,

    /// A vault that is managed by a fund manager.
    /// Fund manager (wallet) is responsible for managing the vault.
    /// Fund manager can't deposit and withdraw funds from the vault.
    FundManager {
        vault_addr: String,
    },
}

#[cw_serde]
pub enum ExecuteMsg {
    //--------------------------------------------------------------------------------------------------
    // Public messages
    //--------------------------------------------------------------------------------------------------
    /// Mints NFT representing a credit account for user. User can have many.
    CreateCreditAccount(AccountKind),
    /// Update user's position on their credit account
    UpdateCreditAccount {
        account_id: Option<String>,
        account_kind: Option<AccountKind>,
        actions: Vec<Action>,
    },
}

/// The list of actions that users can perform on their positions
#[cw_serde]
pub enum Action {
    /// Deposit coin of specified denom and amount. Verifies if the correct amount is sent with transaction.
    Deposit(Coin),
    /// Withdraw coin of specified denom and amount to a wallet address
    WithdrawToWallet { coin: ActionCoin, recipient: String },
    /// Lend coin to the Red Bank
    Lend(ActionCoin),
    /// Reclaim the coins that were lent to the Red Bank.
    Reclaim(ActionCoin),
}

#[cw_serde]
pub enum ActionAmount {
    Exact(Uint128),
    AccountBalance,
}

#[cw_serde]
pub struct ActionCoin {
    pub denom: String,
    pub amount: ActionAmount,
}
