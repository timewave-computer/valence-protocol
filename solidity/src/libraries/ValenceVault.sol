// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {SafeERC20, ERC20, IERC20, ERC4626} from "@openzeppelin-contracts/token/ERC20/extensions/ERC4626.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {Math} from "@openzeppelin-contracts/utils/math/Math.sol";
import {Ownable} from "@openzeppelin-contracts/access/Ownable.sol";
// import {console} from "forge-std/src/console.sol";

contract ValenceVault is ERC4626, Ownable {
    using Math for uint256;

    error VaultIsPaused();
    error OnlyOwnerOrStrategistAllowed();
    error OnlyStrategistAllowed();
    error InvalidRate();
    error InvalidWithdrawFee();
    error ZeroShares();
    error ZeroAssets();

    event PausedStateChanged(bool paused);
    event RateUpdated(uint256 newRate);
    event WithdrawFeeUpdated(uint256 newFee);
    event FeesUpdated(uint256 platformFee, uint256 performanceFee);
    event MaxHistoricalRateUpdated(uint256 newRate);
    event FeesDistributed(
        address indexed strategistAccount,
        address indexed platformAccount,
        uint256 strategistShares,
        uint256 platformShares
    );

    struct FeeConfig {
        uint32 depositFeeBps; // Deposit fee in basis points
        uint32 platformFeeBps; // Yearly platform fee in basis points
        uint32 performanceFeeBps; // Performance fee in basis points
    }

    struct FeeDistributionConfig {
        address strategistAccount; // Account to receive strategist's portion of fees
        address platformAccount; // Account to receive platform's portion of fees
        uint32 strategistRatioBps; // Strategist's share of total fees in basis points
    }

    struct VaultConfig {
        BaseAccount depositAccount;
        BaseAccount withdrawAccount;
        address strategist;
        uint256 depositCap; // 0 means no cap
        uint32 maxWithdrawFee; // in basis points
        FeeConfig fees;
        FeeDistributionConfig feeDistribution;
    }

    VaultConfig public config;
    bool public paused;

    // Current redemption rate in basis points (1/10000)
    uint256 public redemptionRate;
    // Maximum historical redemption rate for performance fee calculation
    uint256 public maxHistoricalRate;
    // Current position withdraw fee in basis points (1/10000)
    uint32 public positionWithdrawFee;
    // Total shares at last update
    uint256 public lastUpdateTotalShares;
    // Last update timestamp for fee calculation
    uint256 public lastUpdateTimestamp;
    // Fees to be collected in asset
    uint256 public feesOwedInAsset;

    // Constant for basis point calculations
    uint32 private constant BASIS_POINTS = 10000;
    // 1 day = 86400 seconds
    uint64 private constant SECONDS_PER_YEAR = 365 days;

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
        maxHistoricalRate = BASIS_POINTS;
        lastUpdateTimestamp = block.timestamp;
        lastUpdateTotalShares = 0;
    }

    function updateConfig(bytes memory _config) public onlyOwner {
        VaultConfig memory decodedConfig = abi.decode(_config, (VaultConfig));

        // TODO: Do checks for config updates
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

    /** @dev Override deposit to handle fees before calling _deposit */
    function deposit(
        uint256 assets,
        address receiver
    ) public override whenNotPaused returns (uint256) {
        uint256 maxAssets = maxDeposit(receiver);
        if (assets > maxAssets) {
            revert ERC4626ExceededMaxDeposit(receiver, assets, maxAssets);
        }

        uint256 depositFee = calculateDepositFee(assets);
        uint256 assetsAfterFee = assets - depositFee;

        if (depositFee > 0) {
            feesOwedInAsset += depositFee;
        }

        uint256 shares = previewDeposit(assetsAfterFee);
        _deposit(_msgSender(), receiver, assets, shares);

        return shares;
    }

    /** @dev Override mint to handle fees before calling _deposit */
    function mint(
        uint256 shares,
        address receiver
    ) public override whenNotPaused returns (uint256) {
        uint256 maxShares = maxMint(receiver);
        if (shares > maxShares) {
            revert ERC4626ExceededMaxMint(receiver, shares, maxShares);
        }

        (uint256 grossAssets, uint256 fee) = calculateMintFee(shares);

        if (fee > 0) {
            feesOwedInAsset += fee;
        }

        _deposit(_msgSender(), receiver, grossAssets, shares);

        return grossAssets;
    }

    /**
     * @notice Updates the redemption rate and calculates/collects fees
     * @param newRate New redemption rate in basis points (1/10000)
     * @param newWithdrawFee New position withdrawal fee in basis points (1/10000)
     */
    function update(
        uint256 newRate,
        uint32 newWithdrawFee
    ) external onlyStrategist {
        // Input validation
        if (newRate == 0) {
            revert InvalidRate();
        }
        VaultConfig memory _config = config;
        if (newWithdrawFee > _config.maxWithdrawFee) {
            revert InvalidWithdrawFee();
        }

        uint256 currentAssets = totalAssets();
        uint256 currentShares = totalSupply();

        if (currentShares == 0) revert ZeroShares();
        if (currentAssets == 0) revert ZeroAssets();

        // Calculate fees

        uint256 platformFees = calculatePlatformFees(
            newRate,
            currentAssets,
            currentShares,
            _config.fees.platformFeeBps
        );

        uint256 performanceFees = calculatePerformanceFees(
            newRate,
            currentAssets,
            _config.fees.performanceFeeBps
        );

        // Update fees owed
        if (platformFees > 0 || performanceFees > 0) {
            feesOwedInAsset += platformFees + performanceFees;
            emit FeesUpdated(platformFees, performanceFees);
        }

        // Distribute accumulated fees
        _distributeFees();

        // Update state
        redemptionRate = newRate;
        positionWithdrawFee = newWithdrawFee;
        lastUpdateTotalShares = currentShares;
        lastUpdateTimestamp = block.timestamp;

        // Update max historical rate if new rate is higher
        if (newRate > maxHistoricalRate) {
            maxHistoricalRate = newRate;
            emit MaxHistoricalRateUpdated(newRate);
        }

        emit RateUpdated(newRate);
        emit WithdrawFeeUpdated(newWithdrawFee);
    }

    function pause(bool _pause) external onlyOwnerOrStrategist {
        paused = _pause;
        emit PausedStateChanged(_pause);
    }

    /**
     * @notice Calculates the gross assets needed and fee for minting shares
     * @param shares Amount of shares to mint
     * @return grossAssets Total assets needed including fee
     * @return fee Fee amount in assets
     */
    function calculateMintFee(
        uint256 shares
    ) public view returns (uint256 grossAssets, uint256 fee) {
        // Calculate base assets needed for shares
        uint256 baseAssets = previewMint(shares);

        // Calculate gross assets required including fee
        uint256 feeBps = config.fees.depositFeeBps;
        if (feeBps == 0) {
            return (baseAssets, 0);
        }

        // grossAssets = baseAssets / (1 - feeRate)
        grossAssets = baseAssets.mulDiv(
            BASIS_POINTS,
            BASIS_POINTS - feeBps,
            Math.Rounding.Ceil
        );

        fee = grossAssets - baseAssets;
        return (grossAssets, fee);
    }

    /**
     * @notice Calculates the fee amount to be charged for a deposit
     * @dev Uses basis points (BPS) for fee calculation where 1 BPS = 0.01%
     *      The fee is rounded up to ensure the protocol doesn't lose dust amounts
     *      If the deposit fee BPS is set to 0, returns 0 to optimize gas
     * @param assets The amount of assets being deposited
     * @return The fee amount in the same decimals as the asset
     */
    function calculateDepositFee(uint256 assets) public view returns (uint256) {
        uint256 feeBps = config.fees.depositFeeBps;
        if (feeBps == 0) return 0;
        return assets.mulDiv(feeBps, BASIS_POINTS, Math.Rounding.Ceil);
    }

    /**
     * @dev Calculates platform fees based on minimum values to prevent manipulation
     * @param newRate New redemption rate being set
     * @param currentAssets Current total assets in vault
     * @param currentShares Current total shares
     * @return platformFees Amount of platform fees to collect
     */
    function calculatePlatformFees(
        uint256 newRate,
        uint256 currentAssets,
        uint256 currentShares,
        uint32 platformFeeBps
    ) public view returns (uint256 platformFees) {
        if (platformFeeBps == 0) {
            return 0;
        }

        // Get minimum shares between current and last update
        uint256 sharesToUse = currentShares;
        if (lastUpdateTotalShares < sharesToUse) {
            sharesToUse = lastUpdateTotalShares;
        }

        // Calculate minimum assets using the lower rate
        uint256 rateToUse = newRate > redemptionRate ? redemptionRate : newRate;

        uint256 assetsToChargeFees = sharesToUse.mulDiv(
            rateToUse,
            BASIS_POINTS
        );

        // Cap at current total assets if lower
        if (assetsToChargeFees > currentAssets) {
            assetsToChargeFees = currentAssets;
        }

        // Calculate time-weighted platform fee
        uint256 timeElapsed = block.timestamp - lastUpdateTimestamp;

        platformFees = assetsToChargeFees
            .mulDiv(platformFeeBps, BASIS_POINTS)
            .mulDiv(timeElapsed, SECONDS_PER_YEAR);

        return platformFees;
    }

    /**
     * @dev Calculates performance fees based on yield above maxHistoricalRate
     * @param newRate New redemption rate being set
     * @param currentAssets Current total assets in vault
     * @return performanceFees Amount of performance fees to collect
     */
    function calculatePerformanceFees(
        uint256 newRate,
        uint256 currentAssets,
        uint32 performanceFeeBps
    ) public view returns (uint256 performanceFees) {
        if (performanceFeeBps == 0 || newRate <= maxHistoricalRate) {
            return 0;
        }

        // Calculate yield as the increase in value since max historical rate
        uint256 yield = currentAssets.mulDiv(
            newRate - maxHistoricalRate,
            BASIS_POINTS
        );

        // Take fee only from the new yield
        performanceFees = yield.mulDiv(performanceFeeBps, BASIS_POINTS);
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

    function _distributeFees() internal {
        FeeDistributionConfig memory feeDistribution = config.feeDistribution;
        if (feesOwedInAsset == 0) return;

        // Calculate fee shares for strategist
        uint256 strategistAssets = feesOwedInAsset.mulDiv(
            feeDistribution.strategistRatioBps,
            BASIS_POINTS,
            Math.Rounding.Floor
        );

        // Calculate platform's share as the remainder
        uint256 platformAssets = feesOwedInAsset - strategistAssets;

        // Convert assets to shares
        uint256 strategistShares = _convertToShares(
            strategistAssets,
            Math.Rounding.Floor
        );
        uint256 platformShares = _convertToShares(
            platformAssets,
            Math.Rounding.Floor
        );

        // Mint shares to respective accounts
        if (strategistShares > 0) {
            _mint(feeDistribution.strategistAccount, strategistShares);
        }
        if (platformShares > 0) {
            _mint(feeDistribution.platformAccount, platformShares);
        }

        // Reset fees owed
        feesOwedInAsset = 0;

        emit FeesDistributed(
            feeDistribution.strategistAccount,
            feeDistribution.platformAccount,
            strategistShares,
            platformShares
        );
    }
}
