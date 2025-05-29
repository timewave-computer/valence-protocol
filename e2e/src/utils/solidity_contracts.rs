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
pub mod sol_lite_processor {
    alloy::sol!(
        #[sol(rpc)]
        LiteProcessor,
        "../solidity/out/LiteProcessor.sol/LiteProcessor.json",
    );
}

pub mod sol_authorizations {
    alloy::sol!(
        #[sol(rpc)]
        Authorizations,
        "../solidity/out/Authorization.sol/Authorization.json"
    );
}

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

// CCTP Transfer
sol!(
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    CCTPTransfer,
    "../solidity/out/CCTPTransfer.sol/CCTPTransfer.json",
);

// Aave Position Manager
sol!(
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    AavePositionManager,
    "../solidity/out/AavePositionManager.sol/AavePositionManager.json",
);

// Standard Bridge Transfer
sol!(
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    StandardBridgeTransfer,
    "../solidity/out/StandardBridgeTransfer.sol/StandardBridgeTransfer.json",
);

// PancakeSwap V3 Position Manager
sol!(
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    PancakeSwapV3PositionManager,
    "../solidity/out/PancakeSwapV3PositionManager.sol/PancakeSwapV3PositionManager.json",
);

// Valence ERC4626-based vault
sol!(
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    ValenceVault,
    "../solidity/out/ValenceVault.sol/ValenceVault.json",
);

// Valence ERC4626-based one way vault
sol!(
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    OneWayVault,
    "../solidity/out/OneWayVault.sol/OneWayVault.json"
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

// Mock CCTP messenger
sol!(
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    MockTokenMessenger,
    "../solidity/out/MockTokenMessenger.sol/MockTokenMessenger.json",
);

// Eureka transfer
sol!(
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    IBCEurekaTransfer,
    "../solidity/out/IBCEurekaTransfer.sol/IBCEurekaTransfer.json",
);
