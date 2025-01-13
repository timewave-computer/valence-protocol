use alloy::sol;

sol!(
    #[sol(rpc)]
    Mailbox,
    "./hyperlane/contracts/solidity/Mailbox.json",
);

sol!(
    #[sol(rpc)]
    MerkleTreeHook,
    "./hyperlane/contracts/solidity/MerkleTreeHook.json",
);

sol!(
    #[sol(rpc)]
    InterchainGasPaymaster,
    "./hyperlane/contracts/solidity/InterchainGasPaymaster.json",
);

sol!(
    #[sol(rpc)]
    PausableIsm,
    "./hyperlane/contracts/solidity/PausableIsm.json",
);

sol!(
    #[sol(rpc)]
    ValidatorAnnounce,
    "./hyperlane/contracts/solidity/ValidatorAnnounce.json",
);

sol!(
    #[sol(rpc)]
    BaseAccount,
    "../solidity/out/BaseAccount.sol/BaseAccount.json",
);
