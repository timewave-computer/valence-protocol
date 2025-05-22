use cosmwasm_std::Uint128;

pub fn get_mmvault_withdraw_msg(amount: Uint128) -> mmvault::msg::ExecuteMsg {
    mmvault::msg::ExecuteMsg::Withdraw { amount }
}

pub fn get_mmvault_deposit_msg() -> mmvault::msg::ExecuteMsg {
    mmvault::msg::ExecuteMsg::Deposit {}
}
