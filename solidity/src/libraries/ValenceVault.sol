// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {SafeERC20, ERC20, IERC20, ERC4626} from "@openzeppelin-contracts/token/ERC20/extensions/ERC4626.sol";
import "@openzeppelin-contracts/utils/ReentrancyGuard.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {Math} from "@openzeppelin-contracts/utils/math/Math.sol";
import {Ownable} from "@openzeppelin-contracts/access/Ownable.sol";
// import {console} from "forge-std/src/console.sol";

contract ValenceVault is ERC4626, Ownable, ReentrancyGuard {
    using Math for uint256;

    error VaultIsPaused();
    error OnlyOwnerOrStrategistAllowed();
    error OnlyStrategistAllowed();
    error InvalidRate();
    error InvalidUpdateSameBlock();
    error InvalidWithdrawFee();
    error ZeroShares();
    error ZeroAssets();
    error InsufficientAllowance(uint256 required, uint256 available);
    error WithdrawAlreadyExists();
    error InvalidReceiver();
    error InvalidOwner();
    error InvalidMaxLoss();
    error InvalidAmount();
    error InvalidSolverFee(uint256 sent, uint256 required);
    error UnexpectedETH();
    error WithdrawRequestNotFound();
    error SolverNotAllowed();
    error WithdrawNotClaimable();
    error SolverFeeTransferFailed();
    error FeeExceedsUint128();

    // Config errors
    error InvalidDepositAccount();
    error InvalidWithdrawAccount();
    error InvalidStrategist();
    error InvalidFeeConfiguration();
    error InvalidFeeDistribution();
    error InvalidWithdrawLockupPeriod();
    error InvalidMaxWithdrawFee();
    error InvalidPlatformAccount();
    error InvalidStrategistAccount();

    event PausedStateChanged(bool paused);
    event RateUpdated(uint256 newRate);
    event UpdateProcessed(uint256 indexed updateId, uint256 withdrawRate, uint256 totalAssetsToWithdraw);
    event WithdrawFeeUpdated(uint256 newFee);
    event FeesUpdated(uint256 platformFee, uint256 performanceFee);
    event MaxHistoricalRateUpdated(uint256 newRate);
    event FeesDistributed(
        address indexed strategistAccount,
        address indexed platformAccount,
        uint256 strategistShares,
        uint256 platformShares
    );
    event WithdrawRequested(
        address indexed owner,
        address indexed receiver,
        uint256 shares,
        uint256 maxLossBps,
        bool solverEnabled,
        uint64 updateId
    );
    event DepositWithdrawNetting(uint256 netAmount, uint256 timestamp);
    event WithdrawCompleted(
        address indexed owner, address indexed receiver, uint256 assets, uint256 shares, address indexed executor
    );
    event WithdrawCancelled(address indexed owner, uint256 shares, uint256 currentLoss, uint256 maxAllowedLoss);
    // Event to track failed withdraws
    event WithdrawCompletionSkipped(address indexed owner, string reason);
    event ConfigUpdated(address indexed updater, VaultConfig newConfig);

    struct FeeConfig {
        uint32 depositFeeBps; // Deposit fee in basis points
        uint32 platformFeeBps; // Yearly platform fee in basis points
        uint32 performanceFeeBps; // Performance fee in basis points
        uint64 solverCompletionFee; // Fee paid to solver for completion of withdraws
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
        FeeConfig fees;
        FeeDistributionConfig feeDistribution;
        uint128 depositCap; // 0 means no cap
        uint64 withdrawLockupPeriod; // Position + vault lockup period in seconds
        uint32 maxWithdrawFee; // in basis points
    }

    // Withdraw request structure
    struct WithdrawRequest {
        address owner; // Owner of the request
        uint64 claimTime; // Timestamp when request becomes claimable
        uint32 maxLossBps; // Maximum acceptable loss in basis points
        address receiver; // Receiver of the withdrawn assets
        uint64 updateId; // Next update ID
        uint64 solverFee; // Fee for solver completion (only used in solver mapping)
        uint128 sharesAmount; // Amount of shares to be redeemed
    }

    // Struct to store information about each update
    struct UpdateInfo {
        uint32 withdrawRate; // Rate at which withdrawals were processed (redemptionRate - withdrawFee)
        uint32 withdrawFee; // The fee of that update
        uint64 timestamp; // When this update occurred
    }

    struct PackedValues {
        // Current position withdraw fee in basis points (1/10000)
        uint64 positionWithdrawFee;
        // Current update ID (increments with each update)
        uint64 currentUpdateId;
        // Withdraw request ID counter
        uint64 nextWithdrawRequestId;
        bool paused;
    }

    /**
     * @dev Internal struct to hold the result of a withdraw completion attempt
     */
    struct WithdrawResult {
        bool success;
        uint256 assetsToWithdraw;
        uint256 solverFee;
        string errorReason;
    }

    VaultConfig public config;
    PackedValues public packedValues;

    // Current redemption rate in basis points (1/10000)
    uint32 public redemptionRate;
    // Maximum historical redemption rate for performance fee calculation
    uint32 public maxHistoricalRate;
    // Total shares at last update
    uint256 public lastUpdateTotalShares;
    // Last update timestamp for fee calculation
    uint64 public lastUpdateTimestamp;
    // Fees to be collected in asset
    uint128 public feesOwedInAsset;
    // The total amount we should withdraw in the next update
    uint128 public totalAssetsToWithdrawNextUpdate;

    // Separate mappings for the actual requests
    mapping(address => WithdrawRequest) public userWithdrawRequest;

    // Mapping from update ID to update information
    mapping(uint64 => UpdateInfo) public updateInfos;

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
        if (packedValues.paused) {
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
    ) ERC20(vaultTokenName, vaultTokenSymbol) ERC4626(IERC20(underlying)) Ownable(_owner) {
        config = abi.decode(_config, (VaultConfig));
        _validateConfig(config);
        unchecked {
            redemptionRate = BASIS_POINTS; // Initialize at 1:1
            maxHistoricalRate = BASIS_POINTS;
            lastUpdateTimestamp = uint64(block.timestamp);
            lastUpdateTotalShares = 0;
        }
    }

    /**
     * @notice Updates the vault configuration
     * @dev Validates all configuration parameters before updating
     * @param _config Encoded VaultConfig struct
     */
    function updateConfig(bytes memory _config) public onlyOwner {
        VaultConfig memory decodedConfig = abi.decode(_config, (VaultConfig));

        _validateConfig(decodedConfig);

        // All validations passed, update config
        config = decodedConfig;

        emit ConfigUpdated(msg.sender, decodedConfig);
    }

    function _validateConfig(VaultConfig memory decodedConfig) internal pure {
        if (address(decodedConfig.depositAccount) == address(0)) {
            revert InvalidDepositAccount();
        }
        if (address(decodedConfig.withdrawAccount) == address(0)) {
            revert InvalidWithdrawAccount();
        }
        if (decodedConfig.strategist == address(0)) {
            revert InvalidStrategist();
        }
        if (
            decodedConfig.fees.depositFeeBps > BASIS_POINTS || decodedConfig.fees.platformFeeBps > BASIS_POINTS
                || decodedConfig.fees.performanceFeeBps > BASIS_POINTS
        ) {
            revert InvalidFeeConfiguration();
        }
        if (decodedConfig.feeDistribution.strategistRatioBps > BASIS_POINTS) {
            revert InvalidFeeDistribution();
        }
        if (decodedConfig.feeDistribution.platformAccount == address(0)) {
            revert InvalidPlatformAccount();
        }
        if (decodedConfig.feeDistribution.strategistAccount == address(0)) {
            revert InvalidStrategistAccount();
        }
        if (decodedConfig.maxWithdrawFee > BASIS_POINTS) {
            revert InvalidMaxWithdrawFee();
        }
        if (decodedConfig.withdrawLockupPeriod == 0) {
            revert InvalidWithdrawLockupPeriod();
        }
    }

    function totalAssets() public view override returns (uint256) {
        return _convertToAssets(totalSupply(), Math.Rounding.Floor);
    }

    function maxDeposit(address) public view override returns (uint256) {
        uint128 cap = config.depositCap;
        if (cap == 0) {
            return type(uint256).max;
        }

        uint256 totalDeposits = totalAssets();
        if (totalDeposits >= cap) {
            return 0;
        }

        unchecked {
            return cap - totalDeposits;
        }
    }

    function maxMint(address) public view override returns (uint256) {
        uint128 cap = config.depositCap;
        if (cap == 0) {
            return type(uint256).max;
        }

        uint256 totalDeposits = totalAssets();
        if (totalDeposits >= cap) {
            return 0;
        }

        return _convertToShares(cap - totalDeposits, Math.Rounding.Floor);
    }

    /**
     * @dev Override deposit to handle fees before calling _deposit
     */
    function deposit(uint256 assets, address receiver) public override whenNotPaused returns (uint256) {
        uint256 maxAssets = maxDeposit(receiver);
        if (assets > maxAssets) {
            revert ERC4626ExceededMaxDeposit(receiver, assets, maxAssets);
        }

        uint128 depositFee = calculateDepositFee(assets);
        uint256 assetsAfterFee;
        unchecked {
            assetsAfterFee = assets - depositFee;
        }

        if (depositFee > 0) {
            feesOwedInAsset += depositFee;
        }

        uint256 shares = previewDeposit(assetsAfterFee);
        _deposit(_msgSender(), receiver, assets, shares);

        return shares;
    }

    /**
     * @dev Override mint to handle fees before calling _deposit
     */
    function mint(uint256 shares, address receiver) public override whenNotPaused returns (uint256) {
        uint256 maxShares = maxMint(receiver);
        if (shares > maxShares) {
            revert ERC4626ExceededMaxMint(receiver, shares, maxShares);
        }

        (uint256 grossAssets, uint128 fee) = calculateMintFee(shares);

        if (fee > 0) {
            feesOwedInAsset += fee;
        }

        _deposit(_msgSender(), receiver, grossAssets, shares);

        return grossAssets;
    }

    function _validateUpdate(uint32 newRate, uint32 newWithdrawFee, uint64 _lastUpdateTimestamp, uint32 maxWithdrawFee)
        internal
        view
    {
        if (_lastUpdateTimestamp == block.timestamp) {
            revert InvalidUpdateSameBlock();
        }

        if (newRate == 0) {
            revert InvalidRate();
        }
        if (newWithdrawFee > maxWithdrawFee) {
            revert InvalidWithdrawFee();
        }
    }

    /**
     * @notice Updates the redemption rate, calculates/collects fees, and handles deposit/withdraw netting
     * @param newRate New redemption rate in basis points (1/10000)
     * @param newWithdrawFee New position withdrawal fee in basis points (1/10000)
     * @param nettingAmount Amount to transfer from deposit to withdraw account for netting
     */
    function update(uint32 newRate, uint32 newWithdrawFee, uint256 nettingAmount) external onlyStrategist {
        uint64 _lastUpdateTimestamp = lastUpdateTimestamp;
        VaultConfig memory _config = config;

        // Input validation
        _validateUpdate(newRate, newWithdrawFee, _lastUpdateTimestamp, _config.maxWithdrawFee);

        uint256 currentAssets = totalAssets();
        uint256 currentShares = totalSupply();

        if (currentShares == 0) revert ZeroShares();
        if (currentAssets == 0) revert ZeroAssets();

        // Calculate fees
        uint128 platformFees = calculatePlatformFees(newRate, currentAssets, currentShares, _config.fees.platformFeeBps);

        uint128 performanceFees = calculatePerformanceFees(newRate, currentAssets, _config.fees.performanceFeeBps);

        // Update fees owed
        if (platformFees > 0 || performanceFees > 0) {
            feesOwedInAsset += platformFees + performanceFees;
            emit FeesUpdated(platformFees, performanceFees);
        }

        // Distribute accumulated fees
        _distributeFees();

        // Handle withdraws
        _handleWithdraws(newWithdrawFee, nettingAmount, _config.depositAccount, address(_config.withdrawAccount));

        // Update state
        redemptionRate = newRate;
        lastUpdateTotalShares = currentShares;
        lastUpdateTimestamp = uint64(block.timestamp);

        // Update max historical rate if new rate is higher
        if (newRate > maxHistoricalRate) {
            maxHistoricalRate = newRate;
            emit MaxHistoricalRateUpdated(newRate);
        }

        emit RateUpdated(newRate);
    }

    function pause() external onlyOwnerOrStrategist {
        packedValues.paused = true;
        emit PausedStateChanged(true);
    }

    function unpause() external onlyOwnerOrStrategist {
        packedValues.paused = false;
        emit PausedStateChanged(false);
    }

    /**
     * @notice Calculates the gross assets needed and fee for minting shares
     * @param shares Amount of shares to mint
     * @return grossAssets Total assets needed including fee
     * @return fee Fee amount in assets
     */
    function calculateMintFee(uint256 shares) public view returns (uint256, uint128) {
        // Calculate base assets needed for shares
        uint256 baseAssets = previewMint(shares);

        // Calculate gross assets required including fee
        uint256 feeBps = config.fees.depositFeeBps;
        if (feeBps == 0) {
            return (baseAssets, 0);
        }

        // grossAssets = baseAssets / (1 - feeRate)
        uint256 grossAssets = baseAssets.mulDiv(BASIS_POINTS, BASIS_POINTS - feeBps, Math.Rounding.Ceil);

        uint256 fee;
        unchecked {
            fee = grossAssets - baseAssets;
        }

        if (fee > type(uint128).max) {
            revert FeeExceedsUint128();
        }

        return (grossAssets, uint128(fee));
    }

    /**
     * @notice Calculates the fee amount to be charged for a deposit
     * @dev Uses basis points (BPS) for fee calculation where 1 BPS = 0.01%
     *      The fee is rounded up to ensure the protocol doesn't lose dust amounts
     *      If the deposit fee BPS is set to 0, returns 0 to optimize gas
     * @param assets The amount of assets being deposited
     * @return The fee amount in the same decimals as the asset
     */
    function calculateDepositFee(uint256 assets) public view returns (uint128) {
        uint256 feeBps = config.fees.depositFeeBps;
        if (feeBps == 0) return 0;

        uint256 fee = assets.mulDiv(feeBps, BASIS_POINTS, Math.Rounding.Ceil);

        // Check if fee exceeds uint128 max value
        if (fee > type(uint128).max) {
            revert FeeExceedsUint128();
        }

        return uint128(fee);
    }

    /**
     * @dev Calculates platform fees based on minimum values to prevent manipulation
     * @param newRate New redemption rate being set
     * @param currentAssets Current total assets in vault
     * @param currentShares Current total shares
     * @return platformFees Amount of platform fees to collect
     */
    function calculatePlatformFees(uint256 newRate, uint256 currentAssets, uint256 currentShares, uint32 platformFeeBps)
        public
        view
        returns (uint128)
    {
        if (platformFeeBps == 0) {
            return 0;
        }

        uint32 _redemptionRate = redemptionRate;

        // Get minimum shares between current and last update
        uint256 sharesToUse = currentShares < lastUpdateTotalShares ? currentShares : lastUpdateTotalShares;

        // Calculate minimum assets using the lower rate
        uint256 rateToUse = newRate > _redemptionRate ? _redemptionRate : newRate;

        uint256 assetsToChargeFees = sharesToUse.mulDiv(rateToUse, BASIS_POINTS);

        // Cap at current total assets if lower
        if (assetsToChargeFees > currentAssets) {
            assetsToChargeFees = currentAssets;
        }

        // Calculate time-weighted platform fee
        uint256 timeElapsed = block.timestamp - lastUpdateTimestamp;

        uint256 platformFees =
            assetsToChargeFees.mulDiv(platformFeeBps, BASIS_POINTS).mulDiv(timeElapsed, SECONDS_PER_YEAR);

        if (platformFees > type(uint128).max) {
            revert FeeExceedsUint128();
        }

        return uint128(platformFees);
    }

    /**
     * @dev Calculates performance fees based on yield above maxHistoricalRate
     * @param newRate New redemption rate being set
     * @param currentAssets Current total assets in vault
     * @return performanceFees Amount of performance fees to collect
     */
    function calculatePerformanceFees(uint256 newRate, uint256 currentAssets, uint32 performanceFeeBps)
        public
        view
        returns (uint128)
    {
        uint32 _maxHistoricalRate = maxHistoricalRate;

        if (performanceFeeBps == 0 || newRate <= _maxHistoricalRate) {
            return 0;
        }

        // Calculate yield as the increase in value since max historical rate
        uint256 yield = currentAssets.mulDiv(newRate - _maxHistoricalRate, BASIS_POINTS);

        // Take fee only from the new yield
        uint256 performanceFees = yield.mulDiv(performanceFeeBps, BASIS_POINTS);

        if (performanceFees > type(uint128).max) {
            revert FeeExceedsUint128();
        }

        return uint128(performanceFees);
    }

    function hasActiveWithdraw(address owner) public view returns (bool) {
        return userWithdrawRequest[owner].owner != address(0);
    }

    /**
     * @notice Creates a withdrawal request for assets
     * @param assets Amount of assets to withdraw
     * @param receiver Address to receive the withdrawn assets
     * @param owner Address that owns the shares
     * @param maxLossBps Maximum acceptable loss in basis points
     * @param allowSolverCompletion Whether to allow solvers to complete this request
     */
    function withdraw(uint256 assets, address receiver, address owner, uint32 maxLossBps, bool allowSolverCompletion)
        public
        payable
        nonReentrant
        whenNotPaused
    {
        _validateWithdrawParams(receiver, owner, assets, maxLossBps);

        // Calculate shares needed for the requested assets
        uint256 shares = previewWithdraw(assets);

        // Check if assets exceed max withdraw amount
        uint256 maxAssets = maxWithdraw(owner);
        if (assets > maxAssets) {
            revert ERC4626ExceededMaxWithdraw(owner, assets, maxAssets);
        }

        _withdraw(shares, assets, receiver, owner, maxLossBps, allowSolverCompletion);
    }

    /**
     * @notice Creates a redemption request for shares
     * @param shares Amount of shares to redeem
     * @param receiver Address to receive the redeemed assets
     * @param owner Address that owns the shares
     * @param maxLossBps Maximum acceptable loss in basis points
     * @param allowSolverCompletion Whether to allow solvers to complete this request
     */
    function redeem(uint256 shares, address receiver, address owner, uint32 maxLossBps, bool allowSolverCompletion)
        public
        payable
        nonReentrant
        whenNotPaused
    {
        _validateWithdrawParams(receiver, owner, shares, maxLossBps);

        // Check if shares exceed max redeem amount
        uint256 maxShares = maxRedeem(owner);
        if (shares > maxShares) {
            revert ERC4626ExceededMaxRedeem(owner, shares, maxShares);
        }

        _withdraw(shares, previewRedeem(shares), receiver, owner, maxLossBps, allowSolverCompletion);
    }

    /**
     * @notice Completes a single withdrawal request
     * @param owner The owner of the withdrawal request
     */
    function completeWithdraw(address owner) external nonReentrant whenNotPaused {
        WithdrawResult memory result = _processWithdraw(owner, true);

        // Handle solver fee if successful
        if (result.solverFee > 0) {
            (bool success,) = payable(msg.sender).call{value: result.solverFee}("");
            if (!success) revert SolverFeeTransferFailed();
        }
    }

    /**
     * @notice Completes multiple withdrawal requests in a single transaction
     * @param owners Array of withdrawal request owners to process
     */
    function completeWithdraws(address[] calldata owners) external nonReentrant whenNotPaused {
        uint256 totalSolverFee = 0;

        for (uint256 i = 0; i < owners.length; i++) {
            WithdrawResult memory result = _processWithdraw(owners[i], false);

            if (!result.success) {
                emit WithdrawCompletionSkipped(owners[i], result.errorReason);
                continue;
            }

            totalSolverFee += result.solverFee;
        }

        // Transfer total solver fee if any
        if (totalSolverFee > 0) {
            (bool success,) = payable(msg.sender).call{value: totalSolverFee}("");
            if (!success) revert SolverFeeTransferFailed();
        }
    }

    /**
     * @dev Processes a single withdraw request and returns the result
     * @param owner The owner of the withdraw request
     * @param revertOnFailure If true, reverts on failure instead of returning result
     * @return result The withdraw completion result
     */
    function _processWithdraw(address owner, bool revertOnFailure) internal returns (WithdrawResult memory result) {
        // Get the withdrawal request
        WithdrawRequest memory request = userWithdrawRequest[owner];

        // Check if request exists
        if (request.sharesAmount == 0) {
            if (revertOnFailure) revert WithdrawRequestNotFound();
            return WithdrawResult(false, 0, 0, "Request not found");
        }

        // Check if sender is authorized
        if (msg.sender != request.owner && request.solverFee == 0) {
            if (revertOnFailure) revert SolverNotAllowed();
            return WithdrawResult(false, 0, 0, "Solver not allowed");
        }

        // Check if request is claimable
        if (block.timestamp < request.claimTime) {
            if (revertOnFailure) revert WithdrawNotClaimable();
            return WithdrawResult(false, 0, 0, "Not claimable yet");
        }

        // Cache state variables
        uint32 _redemptionRate = redemptionRate;
        PackedValues memory _packedValues = packedValues;
        UpdateInfo memory updateInfo = updateInfos[request.updateId];
        uint256 assetsToWithdraw;

        if (_redemptionRate < updateInfo.withdrawRate) {
            // Calculate current withdraw rate since we have a loss
            uint256 currentWithdrawRate = _redemptionRate - _packedValues.positionWithdrawFee;
            uint256 lossBps = ((updateInfo.withdrawRate - currentWithdrawRate) * BASIS_POINTS) / updateInfo.withdrawRate;

            if (lossBps > request.maxLossBps) {
                uint256 refundShares = uint256(request.sharesAmount).mulDiv(
                    BASIS_POINTS - updateInfo.withdrawFee, BASIS_POINTS, Math.Rounding.Floor
                );

                // Loss too high, refund shares minus the withdraw fee
                _mint(owner, refundShares);
                delete userWithdrawRequest[owner];
                emit WithdrawCancelled(owner, refundShares, lossBps, request.maxLossBps);

                return WithdrawResult(false, 0, 0, "Loss exceeds maximum");
            }

            assetsToWithdraw =
                uint256(request.sharesAmount).mulDiv(currentWithdrawRate, BASIS_POINTS, Math.Rounding.Floor);
        } else {
            assetsToWithdraw =
                uint256(request.sharesAmount).mulDiv(updateInfo.withdrawRate, BASIS_POINTS, Math.Rounding.Floor);
        }

        // Delete request before transfer to prevent reentrancy
        delete userWithdrawRequest[owner];

        // Prepare the transfer
        bytes memory transferCalldata = abi.encodeCall(IERC20.transfer, (request.receiver, assetsToWithdraw));

        // Execute transfer
        try config.withdrawAccount.execute(asset(), 0, transferCalldata) {
            emit WithdrawCompleted(owner, request.receiver, assetsToWithdraw, request.sharesAmount, msg.sender);
            return WithdrawResult(true, assetsToWithdraw, request.solverFee, "");
        } catch {
            if (revertOnFailure) revert("Asset transfer failed");
            return WithdrawResult(false, 0, 0, "Asset transfer failed");
        }
    }

    /**
     * @dev Internal function to handle withdrawal/redemption request creation
     * @param shares Amount of shares to withdraw
     * @param receiver Address to receive the assets
     * @param owner Address that owns the shares
     * @param maxLossBps Maximum acceptable loss in basis points
     * @param allowSolverCompletion Whether to allow solvers to complete this request
     */
    function _withdraw(
        uint256 shares,
        uint256 assetsToWithdraw,
        address receiver,
        address owner,
        uint32 maxLossBps,
        bool allowSolverCompletion
    ) internal {
        if (hasActiveWithdraw(owner)) {
            revert WithdrawAlreadyExists();
        }

        PackedValues memory _packedValues = packedValues;
        uint64 _withdrawLockupPeriod = config.withdrawLockupPeriod;
        uint64 _solverFee = allowSolverCompletion ? config.fees.solverCompletionFee : 0;

        // Burn shares first (CEI pattern)
        if (msg.sender != owner) {
            uint256 allowed = allowance(owner, msg.sender);
            if (allowed < shares) {
                revert InsufficientAllowance(shares, allowed);
            }
            _spendAllowance(owner, msg.sender, shares);
        }
        _burn(owner, shares);

        // Create withdrawal request
        uint64 updateId = _packedValues.currentUpdateId + 1;

        // Update the total to withdraw on next update
        totalAssetsToWithdrawNextUpdate += uint128(assetsToWithdraw);

        WithdrawRequest memory request = WithdrawRequest({
            sharesAmount: uint128(shares),
            claimTime: uint64(block.timestamp + _withdrawLockupPeriod),
            maxLossBps: maxLossBps,
            solverFee: _solverFee,
            owner: owner,
            receiver: receiver,
            updateId: updateId
        });

        // Handle solver completion setup if enabled
        if (_solverFee > 0) {
            // Check if sent ETH matches solver fee
            if (msg.value != _solverFee) {
                revert InvalidSolverFee(msg.value, _solverFee);
            }
        } else {
            if (msg.value > 0) {
                revert UnexpectedETH();
            }
        }

        // Store request
        userWithdrawRequest[owner] = request;

        emit WithdrawRequested(owner, receiver, shares, maxLossBps, allowSolverCompletion, updateId);
    }

    function _deposit(address caller, address receiver, uint256 assets, uint256 shares) internal override {
        SafeERC20.safeTransferFrom(IERC20(asset()), caller, address(config.depositAccount), assets);
        _mint(receiver, shares);

        emit Deposit(caller, receiver, assets, shares);
    }

    function _convertToAssets(uint256 shares, Math.Rounding rounding) internal view override returns (uint256) {
        return shares.mulDiv(redemptionRate, BASIS_POINTS, rounding);
    }

    function _convertToShares(uint256 assets, Math.Rounding rounding) internal view override returns (uint256) {
        return assets.mulDiv(BASIS_POINTS, redemptionRate, rounding);
    }

    function _distributeFees() internal {
        uint128 _feesOwedInAsset = feesOwedInAsset;
        if (_feesOwedInAsset == 0) return;

        FeeDistributionConfig memory feeDistribution = config.feeDistribution;

        // Calculate fee shares for strategist
        uint256 strategistAssets =
            uint256(_feesOwedInAsset).mulDiv(feeDistribution.strategistRatioBps, BASIS_POINTS, Math.Rounding.Floor);

        // Calculate platform's share as the remainder
        uint256 platformAssets = _feesOwedInAsset - strategistAssets;

        // Convert assets to shares
        uint256 strategistShares = _convertToShares(strategistAssets, Math.Rounding.Floor);
        uint256 platformShares = _convertToShares(platformAssets, Math.Rounding.Floor);

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
            feeDistribution.strategistAccount, feeDistribution.platformAccount, strategistShares, platformShares
        );
    }

    /**
     * @dev Handles all withdraw-related operations during an update
     * @param newWithdrawFee New position withdrawal fee in basis points
     * @param nettingAmount Amount to transfer from deposit to withdraw account
     */
    function _handleWithdraws(
        uint32 newWithdrawFee,
        uint256 nettingAmount,
        BaseAccount depositAccount,
        address withdrawAccount
    ) internal {
        PackedValues memory _packedValues = packedValues;
        uint32 _redemptionRate = redemptionRate;
        uint128 _totalAssetsToWithdraw = totalAssetsToWithdrawNextUpdate;

        // Check deposit account balance and validate netting amount
        if (nettingAmount > 0) {
            // Prepare the transfer calldata
            bytes memory transferCalldata = abi.encodeCall(IERC20.transfer, (withdrawAccount, nettingAmount));

            // Execute the transfer through the deposit account
            depositAccount.execute(asset(), 0, transferCalldata);

            emit DepositWithdrawNetting(nettingAmount, block.timestamp);
        }

        // Calculate withdraw rate and increment update ID
        uint32 withdrawRate = _redemptionRate - newWithdrawFee;
        _packedValues.currentUpdateId++;

        // Store withdraw info for this update
        updateInfos[_packedValues.currentUpdateId] = UpdateInfo({
            withdrawRate: withdrawRate,
            withdrawFee: uint32(newWithdrawFee),
            timestamp: uint64(block.timestamp)
        });

        // Emit withdraw-related events
        emit UpdateProcessed(_packedValues.currentUpdateId, withdrawRate, _totalAssetsToWithdraw);
        emit WithdrawFeeUpdated(newWithdrawFee);

        // Update withdraw-related state
        _packedValues.positionWithdrawFee = newWithdrawFee;
        totalAssetsToWithdrawNextUpdate = 0;

        packedValues = _packedValues;
    }

    /**
     * @dev Internal function to validate common withdraw/redeem parameters
     */
    function _validateWithdrawParams(address receiver, address owner, uint256 amount, uint32 maxLossBps)
        internal
        pure
    {
        if (amount == 0) revert InvalidAmount();
        if (receiver == address(0)) revert InvalidReceiver();
        if (owner == address(0)) revert InvalidOwner();
        if (maxLossBps > BASIS_POINTS) revert InvalidMaxLoss();
    }
}
