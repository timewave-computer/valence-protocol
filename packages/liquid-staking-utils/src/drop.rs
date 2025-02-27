// Since drop is using an old CosmWasm version, to make it compatible with our packages, we are going to redefine the messages here using Cosmwasm 2.x that we need
// for our library
// The content here is from https://github.com/hadronlabs-org/drop-contracts/, which is the stable API for drop contracts

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum LiquidStakerExecuteMsg {
    Bond {
        receiver: Option<String>,
        r#ref: Option<String>,
    },
    Unbond {},
}

#[cw_serde]
pub enum WithdrawalManagerExecuteMsg {
    Withdraw {},
}

// NFT hook message
#[cw_serde]
pub enum ReceiveNftMsg {
    Withdraw { receiver: Option<String> },
}
