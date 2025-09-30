// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {ERC4626Upgradeable} from "@openzeppelin-contracts-upgradeable/token/ERC20/extensions/ERC4626Upgradeable.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {OwnableUpgradeable} from "@openzeppelin-contracts-upgradeable/access/OwnableUpgradeable.sol";
import {Initializable} from "@openzeppelin-contracts-upgradeable/proxy/utils/Initializable.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC7540Operator} from "../vaults/interfaces/IERC7540Operator.sol";
import {IValenceVaultStrategist} from "../vaults/interfaces/IValenceVaultStrategist.sol";

contract ValenceXCV is
    Initializable,
    ERC4626Upgradeable,
    IERC7540Operator,
    OwnableUpgradeable,
    IValenceVaultStrategist
{
    using Math for uint256;

    error InvalidSharePrice();
    error OnlyStrategistAllowed();
    error NotControllerOrOperator();
    error DepositAccountNotSet();
    error StrategistNotSet();
    error ZeroDepositAmount();
    error StaleSharePrice();
    error InvalidSharePriceMaxAge();

    /// @dev The precision factor for one share, equivalent to 10^decimals().
    uint256 internal ONE_SHARE;

    /// The current price of one share in terms of the underlying asset.
    uint256 public sharePrice;
    /// The timestamp of the last share price update.
    uint256 public lastUpdateTimestamp;
    /// The maximum age of the share price in seconds before it is considered stale.
    uint256 public sharePriceMaxAge;

    /// The address of the authorized strategist who can update the share price.
    address public strategist;

    /// The Valence account contract that holds the deposited funds.
    address public depositAccount;

    /// Mapping of controllers to their approved operators.
    mapping(address controller => mapping(address operator => bool))
        public operators;

    /// @dev Restricts function access to strategist
    modifier onlyStrategist() {
        if (msg.sender != strategist) {
            revert OnlyStrategistAllowed();
        }
        _;
    }

    /// @dev Restricts function access to the controller or any operators
    /// approved by the controller
    /// @param controller address of the target position owner
    modifier onlyControllerOrOperator(address controller) {
        if (controller != msg.sender && !operators[controller][msg.sender]) {
            revert NotControllerOrOperator();
        }
        _;
    }

    // TODO: think whether this should instead be a hard pause (in state) that
    // would require owner intervention
    /// @dev Restricts function to cases where the share price is up to date
    modifier onlyWhenSharePriceNotStale() {
        if (block.timestamp - lastUpdateTimestamp > sharePriceMaxAge) {
            revert StaleSharePrice();
        }
        _;
    }

    constructor() {
        // _disableInitializers();
    }

    /// Initializes the vault.
    /// @param _owner The owner of the vault.
    /// @param strategistAddress The address of the strategist.
    /// @param underlying The address of the underlying asset token.
    /// @param depositAccountAddress The address of the deposit account.
    /// @param vaultTokenName The name of the vault token.
    /// @param vaultTokenSymbol The symbol of the vault token.
    /// @param startSharePrice The initial share price.
    /// @param maxSharePriceAge The maximum age of the share price in seconds.
    function initialize(
        address _owner,
        address strategistAddress,
        address underlying,
        address depositAccountAddress,
        string memory vaultTokenName,
        string memory vaultTokenSymbol,
        uint256 startSharePrice,
        uint256 maxSharePriceAge
    ) external initializer {
        // initialize the vault share token
        __ERC20_init(vaultTokenName, vaultTokenSymbol);
        // initialize the underlying (deposit) token
        __ERC4626_init(IERC20(underlying));
        // set up ownership
        __Ownable_init(_owner);

        if (startSharePrice == 0) revert InvalidSharePrice();
        sharePrice = startSharePrice;
        lastUpdateTimestamp = block.timestamp;

        if (maxSharePriceAge == 0) revert InvalidSharePriceMaxAge();
        sharePriceMaxAge = maxSharePriceAge;

        if (depositAccountAddress == address(0)) revert DepositAccountNotSet();
        depositAccount = depositAccountAddress;

        if (strategistAddress == address(0)) revert StrategistNotSet();
        strategist = strategistAddress;

        // set the share precision based on the underlying token decimals
        ONE_SHARE = 10 ** decimals();
    }

    // ========================================================================
    // ========================= ERC-7540 OPERATOR ============================
    // ========================================================================

    /// Checks whether a given address is an approved operator for a controller
    /// @param controller The address of the controller
    /// @param operator The address of the operator to check
    /// @return A boolean indicating whether the operator is approved
    function isOperator(
        address controller,
        address operator
    ) external view returns (bool) {
        return operators[controller][operator];
    }

    /// Approves or revokes an operator for the caller
    /// @param operator The address of the operator
    /// @param approved A boolean indicating whether to approve or revoke the operator
    /// @return A boolean indicating the success of the operation (always true)
    function setOperator(
        address operator,
        bool approved
    ) external returns (bool) {
        // set the operator status
        operators[msg.sender][operator] = approved;
        emit OperatorSet(msg.sender, operator, approved);
        return true;
    }

    // ========================================================================
    // ============================ DEPOSIT LANE ==============================
    // ========================================================================

    /// Sets the share price. Can only be called by the strategist
    /// @param newSharePrice The new share price
    function setSharePrice(uint256 newSharePrice) external onlyStrategist {
        // setting share price to 0 would allow for infinite mint
        if (newSharePrice == 0) revert InvalidSharePrice();

        sharePrice = newSharePrice;
        lastUpdateTimestamp = block.timestamp;

        emit SharePriceUpdated(newSharePrice, lastUpdateTimestamp);
    }

    /// Converts a given amount of assets to shares
    /// @param assets The amount of assets to convert
    /// @param rounding The rounding direction
    /// @return The corresponding amount of shares
    function _convertToShares(
        uint256 assets,
        Math.Rounding rounding
    ) internal view override returns (uint256) {
        return assets.mulDiv(ONE_SHARE, sharePrice, rounding);
    }

    /// Deposits assets into the vault and mints shares for the receiver.
    /// This is the ERC-4626 compliant deposit function.
    /// @dev calls into deposit(assets, receiver, controller) with controller = msg.sender
    /// @param assets The amount of assets to deposit
    /// @param receiver The address that will receive the shares
    /// @return shares The amount of shares minted
    function deposit(
        uint256 assets,
        address receiver
    ) public override returns (uint256 shares) {
        return deposit(assets, receiver, msg.sender);
    }

    /// Deposits assets into the vault and mints shares for the receiver.
    /// This is the ERC-7540 compliant deposit function.
    /// @param assets The amount of assets to deposit
    /// @param receiver The address that will receive the shares
    /// @param controller The address of the controller on whose behalf the deposit is made
    /// @return shares The amount of shares minted
    function deposit(
        uint256 assets,
        address receiver,
        address controller
    )
        public
        onlyWhenSharePriceNotStale
        onlyControllerOrOperator(controller)
        returns (uint256 shares)
    {
        // zero deposits are not allowed
        if (assets == 0) {
            revert ZeroDepositAmount();
        }

        // calculate the shares to be minted based on provided assets
        // and the current share price
        shares = convertToShares(assets);

        _deposit(controller, receiver, assets, shares);
    }

    /// Internal deposit logic. Transfers assets to the deposit account and mints shares.
    /// Only modification to the default ERC-4626 is that deposited assets are escrowed
    /// to the deposit account (external contract).
    /// @param caller The address that initiated the deposit
    /// @param receiver The address that will receive the shares
    /// @param assets The amount of assets to deposit
    /// @param shares The amount of shares to mint
    function _deposit(
        address caller,
        address receiver,
        uint256 assets,
        uint256 shares
    ) internal override {
        // escrow the deposited assets to the deposit account (external contract)
        SafeERC20.safeTransferFrom(
            IERC20(asset()),
            receiver,
            depositAccount,
            assets
        );
        _mint(receiver, shares);

        emit Deposit(caller, receiver, assets, shares);
    }
}
