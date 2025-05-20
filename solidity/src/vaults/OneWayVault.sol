// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {ERC4626Upgradeable} from "@openzeppelin-contracts-upgradeable/token/ERC20/extensions/ERC4626Upgradeable.sol";
import {ReentrancyGuardUpgradeable} from "@openzeppelin-contracts-upgradeable/utils/ReentrancyGuardUpgradeable.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {OwnableUpgradeable} from "@openzeppelin-contracts-upgradeable/access/OwnableUpgradeable.sol";
import {UUPSUpgradeable} from "@openzeppelin-contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import {Initializable} from "@openzeppelin-contracts-upgradeable/proxy/utils/Initializable.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/**
 * @title OneWayVault
 * @dev A one-way vault contract that enables deposits on one chain with withdrawal requests
 * to be processed on another domain/chain. Uses ERC4626 tokenized vault standard.
 *
 * This vault handles:
 * - Deposits with fee collection
 * - Strategist-controlled redemption rate updates
 * - Cross-domain withdrawal requests (one-way from source to destination)
 * - Fee distribution between platform and strategist
 * - Deposit caps
 * - Pausability
 */
contract OneWayVault is
    Initializable,
    ERC4626Upgradeable,
    OwnableUpgradeable,
    ReentrancyGuardUpgradeable,
    UUPSUpgradeable
{
    using Math for uint256;

    /**
     * @dev Emitted when the vault's paused state changes
     * @param paused New paused state
     */
    event PausedStateChanged(bool paused);

    /**
     * @dev Emitted when the redemption rate is updated
     * @param newRate The updated redemption rate
     */
    event RateUpdated(uint256 newRate);

    /**
     * @dev Emitted when fees are distributed to strategist and platform accounts
     * @param strategistAccount Address receiving strategist portion of fees
     * @param platformAccount Address receiving platform portion of fees
     * @param strategistShares Amount of shares distributed to strategist
     * @param platformShares Amount of shares distributed to platform
     */
    event FeesDistributed(
        address indexed strategistAccount,
        address indexed platformAccount,
        uint256 strategistShares,
        uint256 platformShares
    );

    /**
     * @dev Emitted when a withdrawal request is created
     * @param id Unique ID for the withdrawal request
     * @param owner Address that owns the shares being withdrawn
     * @param receiver Address on destination domain to receive assets (as string)
     * @param shares Amount of shares being withdrawn
     */
    event WithdrawRequested(uint64 indexed id, address owner, string receiver, uint256 shares);

    /**
     * @dev Emitted when the vault configuration is updated
     * @param updater Address that updated the config
     * @param newConfig The new configuration
     */
    event ConfigUpdated(address indexed updater, OneWayVaultConfig newConfig);

    /**
     * @dev Restricts function access to only the strategist
     */
    modifier onlyStrategist() {
        if (msg.sender != config.strategist) {
            revert("Only strategist allowed");
        }
        _;
    }

    /**
     * @dev Restricts function access to only the owner or strategist
     */
    modifier onlyOwnerOrStrategist() {
        if (msg.sender != owner() && msg.sender != config.strategist) {
            revert("Only owner or strategist allowed");
        }
        _;
    }

    /**
     * @dev Ensures the vault is not paused
     */
    modifier whenNotPaused() {
        if (vaultState.paused) {
            revert("Vault is paused");
        }
        _;
    }

    /**
     * @dev Configuration structure for the vault
     * @param depositAccount Account where deposits are held
     * @param strategist Address of the vault strategist
     * @param depositFeeBps Fee charged on deposits in basis points (1 BPS = 0.01%)
     * @param depositCap Maximum assets that can be deposited (0 means no cap)
     * @param feeDistribution Configuration for fee distribution between platform and strategist
     */
    struct OneWayVaultConfig {
        BaseAccount depositAccount;
        address strategist;
        uint32 depositFeeBps;
        uint128 depositCap;
        FeeDistributionConfig feeDistribution;
    }

    /**
     * @dev Configuration for fee distribution
     * @param strategistAccount Address to receive strategist's portion of fees
     * @param platformAccount Address to receive platform's portion of fees
     * @param strategistRatioBps Strategist's percentage of total fees in basis points
     */
    struct FeeDistributionConfig {
        address strategistAccount; // Account to receive strategist's portion of fees
        address platformAccount; // Account to receive platform's portion of fees
        uint32 strategistRatioBps; // Strategist's share of total fees in basis points
    }

    /**
     * @dev Vault state information
     * @param paused Whether the vault is currently paused
     * @param pausedByOwner Whether the vault was paused by the owner (affects who can unpause)
     */
    struct VaultState {
        bool paused;
        // If paused by owner, only owner can unpause it
        bool pausedByOwner;
    }

    /**
     * @dev Structure for withdrawal requests to destination domain
     * @param id Unique ID for the request
     * @param owner Owner of the request who burned shares
     * @param receiver Address to receive assets on destination domain (as string, e.g. Neutron address)
     * @param redemptionRate Redemption rate at time of request
     * @param sharesAmount Amount of shares to be redeemed
     */
    struct WithdrawRequest {
        uint64 id;
        address owner;
        string receiver;
        uint256 redemptionRate;
        uint256 sharesAmount;
    }

    // Main state variables

    OneWayVaultConfig public config;

    uint256 public redemptionRate;

    VaultState public vaultState;

    uint64 public currentWithdrawRequestId;

    /**
     * @dev Total fees collected but not yet distributed, denominated in asset
     */
    uint256 public feesOwedInAsset;

    /**
     * @dev Mapping from request ID to withdrawal request details
     */
    mapping(uint64 => WithdrawRequest) public withdrawRequests;

    // Constants

    /**
     * @dev Constant for basis point calculations (100% = 10000)
     */
    uint32 private constant BASIS_POINTS = 1e4;

    uint256 internal ONE_SHARE;

    /**
     * @dev Constructor that disables initializers
     * @notice Required for UUPS proxy pattern
     */
    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    /**
     * @dev Initializes the contract replacing the constructor
     * @param _owner Address of the contract owner
     * @param _config Encoded configuration bytes
     * @param underlying Address of the underlying token
     * @param vaultTokenName Name of the vault token
     * @param vaultTokenSymbol Symbol of the vault token
     * @param startingRate Initial redemption rate
     */
    function initialize(
        address _owner,
        bytes memory _config,
        address underlying,
        string memory vaultTokenName,
        string memory vaultTokenSymbol,
        uint256 startingRate
    ) public initializer {
        __ERC20_init(vaultTokenName, vaultTokenSymbol);
        __ERC4626_init(IERC20(underlying));
        __Ownable_init(_owner);
        __ReentrancyGuard_init();

        config = abi.decode(_config, (OneWayVaultConfig));
        _validateConfig(config);

        unchecked {
            ONE_SHARE = 10 ** decimals();
            redemptionRate = startingRate; // Initialize at specified starting rate (1:1)
        }
    }

    /**
     * @dev Function that authorizes contract upgrades - required by UUPSUpgradeable
     * @param newImplementation address of the new implementation
     */
    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {
        // Upgrade logic comes here
        // No additional logic required beyond owner check in modifier
    }

    /**
     * @notice Updates the vault configuration
     * @dev Validates all configuration parameters before updating
     * @param _config Encoded OneWayVaultConfig struct
     */
    function updateConfig(bytes memory _config) public onlyOwner {
        OneWayVaultConfig memory decodedConfig = abi.decode(_config, (OneWayVaultConfig));

        _validateConfig(decodedConfig);

        // All validations passed, update config
        config = decodedConfig;

        emit ConfigUpdated(msg.sender, decodedConfig);
    }

    /**
     * @dev Validates the configuration parameters
     * @param decodedConfig The decoded OneWayVaultConfig struct
     */
    function _validateConfig(OneWayVaultConfig memory decodedConfig) internal pure {
        if (address(decodedConfig.depositAccount) == address(0)) {
            revert("Deposit account cannot be zero address");
        }

        if (decodedConfig.strategist == address(0)) {
            revert("Strategist cannot be zero address");
        }
        if (decodedConfig.depositFeeBps > BASIS_POINTS) {
            revert("Deposit fee cannot exceed 100%");
        }

        if (decodedConfig.feeDistribution.strategistRatioBps > BASIS_POINTS) {
            revert("Strategist account fee distribution ratio cannot exceed 100%");
        }
        if (decodedConfig.feeDistribution.platformAccount == address(0)) {
            revert("Platform account cannot be zero address");
        }
        if (decodedConfig.feeDistribution.strategistAccount == address(0)) {
            revert("Strategist account cannot be zero address");
        }
    }

    /**
     * @notice Returns the total amount of assets managed by the vault
     * @dev Overrides ERC4626 totalAssets to use current redemption rate
     * @return Total assets calculated from total shares using current redemption rate
     */
    function totalAssets() public view override returns (uint256) {
        return _convertToAssets(totalSupply(), Math.Rounding.Floor);
    }

    /**
     * @notice Returns maximum amount that can be deposited for a receiver
     * @dev Overrides ERC4626 maxDeposit to enforce deposit cap if configured
     * @return Maximum deposit amount allowed
     */
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

    /**
     * @notice Returns maximum shares that can be minted for a receiver
     * @dev Overrides ERC4626 maxMint to enforce deposit cap if configured
     * @return Maximum shares that can be minted
     */
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
     * @notice Deposits assets into the vault, charging a fee if configured
     * @dev Overrides ERC4626 deposit to handle fees before calling _deposit
     * @param assets Amount of assets to deposit
     * @param receiver Address to receive the vault shares
     * @return shares Amount of shares minted to receiver
     */
    function deposit(uint256 assets, address receiver) public override whenNotPaused returns (uint256) {
        uint256 maxAssets = maxDeposit(receiver);
        if (assets > maxAssets) {
            revert ERC4626ExceededMaxDeposit(receiver, assets, maxAssets);
        }

        uint256 depositFee = calculateDepositFee(assets);
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
     * @notice Mints exact shares to receiver, calculating and charging required assets
     * @dev Overrides ERC4626 mint to handle fees before calling _deposit
     * @param shares Amount of shares to mint
     * @param receiver Address to receive the shares
     * @return assets Total amount of assets deposited (including fees)
     */
    function mint(uint256 shares, address receiver) public override whenNotPaused returns (uint256) {
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
     * @notice Updates the redemption rate and distributes accumulated fees
     * @dev Can only be called by the strategist when the vault is not paused
     * @param newRate New redemption rate to set
     */
    function update(uint256 newRate) external onlyStrategist whenNotPaused {
        // Validate the new rate
        if (newRate == 0) {
            revert("Redemption rate cannot be zero");
        }

        OneWayVaultConfig memory _config = config;

        uint256 currentAssets = totalAssets();
        uint256 currentShares = totalSupply();

        if (currentShares == 0) revert("Zero shares");
        if (currentAssets == 0) revert("Zero assets");

        // Distribute accumulated fees
        _distributeFees(_config.feeDistribution);

        // Update state
        redemptionRate = newRate;

        emit RateUpdated(newRate);
    }

    /**
     * @notice Pauses the vault, preventing deposits and withdrawals
     * @dev Can be called by owner or strategist, but only owner can unpause if paused by owner
     */
    function pause() external onlyOwnerOrStrategist {
        VaultState memory _vaultState;
        if (msg.sender == owner()) {
            _vaultState.pausedByOwner = true;
        } else {
            _vaultState.pausedByOwner = false;
        }

        _vaultState.paused = true;
        vaultState = _vaultState;
        emit PausedStateChanged(true);
    }

    /**
     * @notice Unpauses the vault, allowing deposits and withdrawals
     * @dev If paused by owner, only owner can unpause; otherwise either owner or strategist can unpause
     */
    function unpause() external onlyOwnerOrStrategist {
        VaultState memory _vaultState = vaultState;
        if (_vaultState.pausedByOwner && msg.sender != owner()) {
            revert("Only owner can unpause");
        }
        delete vaultState;
        emit PausedStateChanged(false);
    }

    /**
     * @notice Calculates the gross assets needed and fee for minting shares
     * @param shares Amount of shares to mint
     * @return grossAssets Total assets needed including fee
     * @return fee Fee amount in assets
     */
    function calculateMintFee(uint256 shares) public view returns (uint256, uint256) {
        // Calculate base assets needed for shares
        uint256 baseAssets = previewMint(shares);

        // Calculate gross assets required including fee
        uint256 feeBps = config.depositFeeBps;
        if (feeBps == 0) {
            return (baseAssets, 0);
        }

        // grossAssets = baseAssets / (1 - feeRate)
        // This formula ensures that after the fee is deducted, exactly baseAssets remain
        uint256 grossAssets = baseAssets.mulDiv(BASIS_POINTS, BASIS_POINTS - feeBps, Math.Rounding.Ceil);

        uint256 fee;
        unchecked {
            fee = grossAssets - baseAssets;
        }

        if (fee > type(uint256).max) {
            revert("Fee exceeds uint256 max");
        }

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
        uint32 feeBps = config.depositFeeBps;
        if (feeBps == 0) return 0;

        uint256 fee = assets.mulDiv(feeBps, BASIS_POINTS, Math.Rounding.Ceil);

        // Check if fee exceeds uint256 max value
        if (fee > type(uint256).max) {
            revert("Fee exceeds uint256 max");
        }

        return fee;
    }

    /**
     * @notice Creates a withdrawal request for assets to be processed on destination domain
     * @dev Assets are calculated from shares based on current redemption rate
     * @param assets Amount of assets to withdraw
     * @param receiver Address to receive the withdrawn assets on the destination domain (as string)
     * @param owner Address that owns the shares
     */
    function withdraw(uint256 assets, string calldata receiver, address owner)
        public
        payable
        nonReentrant
        whenNotPaused
    {
        _validateWithdrawParams(owner, receiver, assets);

        // Check if assets exceed max withdraw amount
        uint256 maxAssets = maxWithdraw(owner);
        if (assets > maxAssets) {
            revert ERC4626ExceededMaxWithdraw(owner, assets, maxAssets);
        }

        _withdraw(previewWithdraw(assets), receiver, owner);
    }

    /**
     * @notice Creates a redemption request for shares to be processed on destination domain
     * @param shares Amount of shares to redeem
     * @param receiver Address to receive the redeemed assets on destination domain (as string)
     * @param owner Address that owns the shares
     */
    function redeem(uint256 shares, string calldata receiver, address owner)
        public
        payable
        nonReentrant
        whenNotPaused
    {
        _validateWithdrawParams(owner, receiver, shares);

        // Check if shares exceed max redeem amount
        uint256 maxShares = maxRedeem(owner);
        if (shares > maxShares) {
            revert ERC4626ExceededMaxRedeem(owner, shares, maxShares);
        }

        _withdraw(shares, receiver, owner);
    }

    /**
     * @dev Internal function to handle withdrawal/redemption request creation
     * @param shares Amount of shares to withdraw
     * @param receiver Address to receive the assets on the destination domain (as string)
     * @param owner Address that owns the shares
     */
    function _withdraw(uint256 shares, string calldata receiver, address owner) internal {
        // Burn shares first (CEI pattern - Checks, Effects, Interactions)
        if (msg.sender != owner) {
            uint256 allowed = allowance(owner, msg.sender);
            if (allowed < shares) {
                revert("Insufficient allowance");
            }
            _spendAllowance(owner, msg.sender, shares);
        }
        _burn(owner, shares);

        WithdrawRequest memory request = WithdrawRequest({
            id: currentWithdrawRequestId,
            owner: owner,
            receiver: receiver,
            sharesAmount: shares,
            redemptionRate: redemptionRate
        });

        // Store the request
        withdrawRequests[currentWithdrawRequestId] = request;

        // Emit the event
        emit WithdrawRequested(currentWithdrawRequestId, owner, receiver, shares);

        // Increment the request ID for the next request
        currentWithdrawRequestId++;
    }

    /**
     * @dev Internal function to handle deposit implementation
     * @param caller Address initiating the deposit
     * @param receiver Address to receive the minted shares
     * @param assets Amount of assets being deposited
     * @param shares Amount of shares to mint
     */
    function _deposit(address caller, address receiver, uint256 assets, uint256 shares) internal override {
        // Transfer assets to the deposit account (external contract)
        SafeERC20.safeTransferFrom(IERC20(asset()), caller, address(config.depositAccount), assets);
        _mint(receiver, shares);

        emit Deposit(caller, receiver, assets, shares);
    }

    /**
     * @dev Converts shares to assets using current redemption rate
     * @param shares Amount of shares to convert
     * @param rounding Rounding direction (up or down)
     * @return Equivalent amount of assets
     */
    function _convertToAssets(uint256 shares, Math.Rounding rounding) internal view override returns (uint256) {
        return shares.mulDiv(redemptionRate, ONE_SHARE, rounding);
    }

    /**
     * @dev Converts assets to shares using current redemption rate
     * @param assets Amount of assets to convert
     * @param rounding Rounding direction (up or down)
     * @return Equivalent amount of shares
     */
    function _convertToShares(uint256 assets, Math.Rounding rounding) internal view override returns (uint256) {
        return assets.mulDiv(ONE_SHARE, redemptionRate, rounding);
    }

    /**
     * @dev Distributes accumulated fees between strategist and platform
     * @param feeDistribution Fee distribution configuration
     */
    function _distributeFees(FeeDistributionConfig memory feeDistribution) internal {
        uint256 _feesOwedInAsset = feesOwedInAsset;
        if (_feesOwedInAsset == 0) return;

        // Calculate fee shares for strategist
        uint256 strategistAssets =
            _feesOwedInAsset.mulDiv(feeDistribution.strategistRatioBps, BASIS_POINTS, Math.Rounding.Floor);

        // Calculate platform's share as the remainder
        uint256 platformAssets = _feesOwedInAsset - strategistAssets;

        // Convert assets to shares
        uint256 strategistShares = _convertToShares(strategistAssets, Math.Rounding.Floor);
        uint256 platformShares = _convertToShares(platformAssets, Math.Rounding.Floor);

        // Reset fees owed
        feesOwedInAsset = 0;

        // Mint shares to respective accounts
        if (strategistShares > 0) {
            _mint(feeDistribution.strategistAccount, strategistShares);
        }
        if (platformShares > 0) {
            _mint(feeDistribution.platformAccount, platformShares);
        }

        emit FeesDistributed(
            feeDistribution.strategistAccount, feeDistribution.platformAccount, strategistShares, platformShares
        );
    }

    /**
     * @dev Internal function to validate common withdraw/redeem parameters
     * @param owner Address that owns the shares
     * @param receiver Address to receive the assets on the destination domain (as string)
     * @param amount Amount of shares/assets to withdraw
     */
    function _validateWithdrawParams(address owner, string calldata receiver, uint256 amount) internal pure {
        if (owner == address(0)) revert("Owner of shares cannot be zero address");
        if (bytes(receiver).length == 0) revert("Receiver cannot be empty");
        if (amount == 0) revert("Amount to withdraw cannot be zero");
    }

    /**
     * @notice Fallback function that reverts all calls to non-existent functions
     * @dev Called when no other function matches the function signature
     */
    fallback() external {
        revert("Function not found");
    }
}
