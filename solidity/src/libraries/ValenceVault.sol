// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {SafeERC20, ERC20, IERC20, ERC4626} from "@openzeppelin-contracts/token/ERC20/extensions/ERC4626.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {Math} from "@openzeppelin-contracts/utils/math/Math.sol";
import {Ownable} from "@openzeppelin-contracts/access/Ownable.sol";

contract ValenceVault is ERC4626, Ownable {
    using Math for uint256;

    error VaultIsPaused();
    error OnlyOwnerOrStrategistAllowed();
    error OnlyStrategistAllowed();
    error InvalidRate();
    error InvalidWithdrawFee();

    event PausedStateChanged(bool paused);
    event RateUpdated(uint256 newRate);
    event WithdrawFeeUpdated(uint256 newFee);

    struct VaultConfig {
        BaseAccount depositAccount;
        BaseAccount withdrawAccount;
        address strategist;
        uint256 depositCap; // 0 means no cap
        uint256 maxWithdrawFee; // in basis points
    }

    VaultConfig public config;
    bool public paused;

    // Current redemption rate in basis points (1/10000)
    uint256 public redemptionRate;
    // Current position withdraw fee in basis points (1/10000)
    uint256 public positionWithdrawFee;
    // Constant for basis point calculations
    uint256 private constant BASIS_POINTS = 10000;

    modifier onlyStrategist() {
        if (msg.sender != config.strategist) {
            revert OnlyStrategistAllowed();
        }
        _;
    }

    modifier onlyOwnerOrStrategist() {
        if (msg.sender != owner() && msg.sender != config.strategist) {
            revert OnlyOwnerOrStrategistAllowed();
        }
        _;
    }

    modifier whenNotPaused() {
        if (paused) {
            revert VaultIsPaused();
        }
        _;
    }

    constructor(
        address _owner,
        bytes memory _config,
        address underlying,
        string memory vaultTokenName,
        string memory vaultTokenSymbol
    )
        ERC20(vaultTokenName, vaultTokenSymbol)
        ERC4626(IERC20(underlying))
        Ownable(_owner)
    {
        config = abi.decode(_config, (VaultConfig));
        redemptionRate = BASIS_POINTS; // Initialize at 1:1
    }

    function updateConfig(bytes memory _config) public onlyOwner {
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

    function deposit(
        uint256 assets,
        address receiver
    ) public override whenNotPaused returns (uint256) {
        return super.deposit(assets, receiver);
    }

    function mint(
        uint256 shares,
        address receiver
    ) public override whenNotPaused returns (uint256) {
        return super.mint(shares, receiver);
    }

    /**
     * @notice Updates the redemption rate and position withdrawal fee
     * @param rate New redemption rate in basis points (1/10000)
     * @param withdrawFee New position withdrawal fee in basis points (1/10000)
     * @dev Only callable by strategist
     */
    function update(uint256 rate, uint256 withdrawFee) external onlyStrategist {
        // Rate should never be zero as it would make shares worthless
        if (rate == 0) {
            revert InvalidRate();
        }

        // Withdraw fee cannot exceed maximum set in config
        if (withdrawFee > config.maxWithdrawFee) {
            revert InvalidWithdrawFee();
        }

        redemptionRate = rate;
        positionWithdrawFee = withdrawFee;

        emit RateUpdated(rate);
        emit WithdrawFeeUpdated(withdrawFee);
    }

    function pause(bool _pause) external onlyOwnerOrStrategist {
        paused = _pause;
        emit PausedStateChanged(_pause);
    }

    function _deposit(
        address caller,
        address receiver,
        uint256 assets,
        uint256 shares
    ) internal override {
        SafeERC20.safeTransferFrom(
            IERC20(asset()),
            caller,
            address(config.depositAccount),
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
