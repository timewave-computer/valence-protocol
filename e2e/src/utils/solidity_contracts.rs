use alloy::sol;

// Hyperlane
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

// Valence Core
sol!(
    #[sol(rpc)]
    LiteProcessor,
    "../solidity/out/LiteProcessor.sol/LiteProcessor.json",
);

// Valence Base Accounts
sol!(
    #[sol(rpc)]
    BaseAccount,
    "../solidity/out/BaseAccount.sol/BaseAccount.json",
);

// Valence Libraries
sol!(
    #[sol(rpc)]
    Forwarder,
    "../solidity/out/Forwarder.sol/Forwarder.json",
);

// Valence ERC4626-based vault
sol!(
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    ValenceVault,
    "../solidity/out/ValenceVault.sol/ValenceVault.json",
);

// Proxy contract
sol!(
    #[sol(rpc)]
    ERC1967Proxy,
    "../solidity/out/ERC1967Proxy.sol/ERC1967Proxy.json",
);

// Testing utils
sol!(
    #[sol(rpc)]
    MockERC20,
    "../solidity/out/MockERC20.sol/MockERC20.json",
);

sol!(
    #[sol(rpc)]
    ERC20,
    "../solidity/out/ERC20.sol/ERC20.json",
);

// CCTP Transfer
sol!(
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    CCTPTransfer,
    "../solidity/out/CCTPTransfer.sol/CCTPTransfer.json",
);

// Mock CCTP messenger
sol!(
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    MockTokenMessenger,
    "../solidity/out/MockTokenMessenger.sol/MockTokenMessenger.json",
);
