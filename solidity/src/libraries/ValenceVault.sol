// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {SafeERC20, ERC20, IERC20, ERC4626} from "@openzeppelin-contracts/token/ERC20/extensions/ERC4626.sol";
import {Library} from "./Library.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {Math} from "@openzeppelin-contracts/utils/math/Math.sol";

contract ValenceVault is Library, ERC4626 {
    using Math for uint256;

    struct VaultConfig {
        BaseAccount DepositAccount;
        BaseAccount WithdrawAccount;
        address Strategist;
        uint256 depositCap; // 0 means no cap
    }

    VaultConfig public config;

    // Current redemption rate in basis points (1/10000)
    uint256 public redemptionRate;
    // Constant for basis point calculations
    uint256 private constant BASIS_POINTS = 10000;

    constructor(
        address _owner,
        address _processor,
        bytes memory _config,
        address underlying,
        string memory vaultTokenName,
        string memory vaultTokenSymbol
    )
        Library(_owner, _processor, _config)
        ERC20(vaultTokenName, vaultTokenSymbol)
        ERC4626(IERC20(underlying))
    {
        redemptionRate = BASIS_POINTS; // Initialize at 1:1
    }

    function updateConfig(bytes memory _config) public override onlyOwner {
        VaultConfig memory decodedConfig = abi.decode(_config, (VaultConfig));

        config = decodedConfig;
    }

    function totalAssets() public view override returns (uint256) {
        return _convertToAssets(totalSupply(), Math.Rounding.Floor);
    }

    function maxDeposit(address) public view override returns (uint256) {
        if (config.depositCap == 0) {
            return type(uint256).max;
        }

        uint256 totalDeposits = totalAssets();
        if (totalDeposits >= config.depositCap) {
            return 0;
        }

        return config.depositCap - totalDeposits;
    }

    function maxMint(address) public view override returns (uint256) {
        if (config.depositCap == 0) {
            return type(uint256).max;
        }

        uint256 totalDeposits = totalAssets();
        if (totalDeposits >= config.depositCap) {
            return 0;
        }

        return
            _convertToShares(
                config.depositCap - totalDeposits,
                Math.Rounding.Floor
            );
    }

    function _deposit(
        address caller,
        address receiver,
        uint256 assets,
        uint256 shares
    ) internal virtual override {
        SafeERC20.safeTransferFrom(
            IERC20(asset()),
            caller,
            address(config.DepositAccount),
            assets
        );
        _mint(receiver, shares);

        emit Deposit(caller, receiver, assets, shares);
    }

    function _convertToAssets(
        uint256 shares,
        Math.Rounding rounding
    ) internal view override returns (uint256) {
        return shares.mulDiv(redemptionRate, BASIS_POINTS, rounding);
    }

    function _convertToShares(
        uint256 assets,
        Math.Rounding rounding
    ) internal view override returns (uint256) {
        return assets.mulDiv(BASIS_POINTS, redemptionRate, rounding);
    }
}
