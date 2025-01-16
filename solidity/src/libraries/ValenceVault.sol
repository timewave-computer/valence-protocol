// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {SafeERC20, IERC20Metadata, ERC20, IERC20, IERC4626, ERC4626} from "@openzeppelin-contracts/token/ERC20/extensions/ERC4626.sol";
import {Math} from "@openzeppelin-contracts/utils/math/Math.sol";
import {Library} from "./Library.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";

contract ValenceVault is Library, ERC4626 {
    struct VaultConfig {
        BaseAccount DepositAccount;
        BaseAccount WithdrawAccount;
    }

    VaultConfig public config;
    address public strategist;
    uint256 public assetsInPosition;

    constructor(
        address _owner,
        address _processor,
        address _strategist,
        bytes memory _config,
        address underlying,
        string memory vaultTokenName,
        string memory vaultTokenSymbol
    )
        Library(_owner, _processor, _config)
        ERC20(vaultTokenName, vaultTokenSymbol)
        ERC4626(IERC20(underlying))
    {
        strategist = _strategist;
    }

    function updateConfig(bytes memory _config) public override onlyOwner {
        VaultConfig memory decodedConfig = abi.decode(_config, (VaultConfig));

        config = decodedConfig;
    }

    // totalAssets
    // function totalAssets() public view override returns (uint256) {
    //     return IERC20(asset()).balanceOf(address(config.DepositAccount)) + assetsInPosition;
    // }

    // convertToShares
    // convertToAssets
    // maxWithdraw
    // maxRedeem
    // previewDeposit
    // previewMint
    // previewWithdraw
    // previewRedeem
    // deposit
    // mint
    // withdraw
    // redeem

    // _convertToShares
    // _convertToAssets
    // _deposit
    // _withdraw
    // _decimalsOffset
}
