use alloy::sol;
use serde::{Deserialize, Serialize};

sol!(
    #[sol(rpc)]
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    Forwarder,
    "../../solidity/out/Forwarder.sol/Forwarder.json",
);

sol!(
    #[sol(rpc)]
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    BaseAccount,
    "../../solidity/out/BaseAccount.sol/BaseAccount.json",
);

sol!(
    #[sol(rpc)]
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    LibraryProxy,
    "../../solidity/out/LibraryProxy.sol/LibraryProxy.json",
); 

