// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {ERC4626Upgradeable} from "@openzeppelin-contracts-upgradeable/token/ERC20/extensions/ERC4626Upgradeable.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {OwnableUpgradeable} from "@openzeppelin-contracts-upgradeable/access/OwnableUpgradeable.sol";
import {Initializable} from "@openzeppelin-contracts-upgradeable/proxy/utils/Initializable.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

// TODO: look into any ERC-4626 defaults and think if anyhthing else needs
// to be overridden

contract ValenceXCV is Initializable, ERC4626Upgradeable, OwnableUpgradeable {
    using Math for uint256;

    uint256 internal ONE_SHARE;

    // current share price
    uint256 public sharePrice;

    // authorized strategist address
    address public strategist;

    // Valence account for holding the deposited funds
    address public depositAccount;

    error InvalidSharePrice();

    error OnlyStrategistAllowed();

    error DepositAccountNotSet();
    error StrategistNotSet();
    error ZeroDepositAmount();

    event SharePriceUpdated(uint256 indexed sharePrice);

    modifier onlyStrategist() {
        if (msg.sender != strategist) {
            revert OnlyStrategistAllowed();
        }
        _;
    }

    constructor() {
        // _disableInitializers();
    }

    function initialize(
        address _owner,
        address strategistAddress,
        address underlying,
        address depositAccountAddress,
        string memory vaultTokenName,
        string memory vaultTokenSymbol,
        uint256 startSharePrice
    ) external initializer {
        // initialize the vault share token
        __ERC20_init(vaultTokenName, vaultTokenSymbol);
        // initialize the underlying (deposit) token
        __ERC4626_init(IERC20(underlying));
        // set up ownership
        __Ownable_init(_owner);

        if (startSharePrice == 0) revert InvalidSharePrice();
        sharePrice = startSharePrice;

        if (depositAccountAddress == address(0)) revert DepositAccountNotSet();
        depositAccount = depositAccountAddress;

        if (strategistAddress == address(0)) revert StrategistNotSet();
        strategist = strategistAddress;

        // set the share precision based on the underlying token decimals
        ONE_SHARE = 10 ** decimals();
    }

    // ========================================================================
    // ============================ DEPOSIT LANE ==============================
    // ========================================================================
    function setSharePrice(uint256 newSharePrice) external onlyStrategist {
        // setting share price to 0 would allow for infinite mint
        if (newSharePrice == 0) revert InvalidSharePrice();

        sharePrice = newSharePrice;

        emit SharePriceUpdated(newSharePrice);
    }

    function _convertToShares(
        uint256 assets,
        Math.Rounding rounding
    ) internal view override returns (uint256) {
        return assets.mulDiv(ONE_SHARE, sharePrice, rounding);
    }

    // ERC-4626 deposit implementation that calls into the ERC-7540 deposit
    // with the addition of controller
    function deposit(
        uint256 assets,
        address receiver
    ) public override returns (uint256 shares) {
        // forward to the 7540 deposit overload with controller = msg.sender
        return deposit(assets, receiver, msg.sender);
    }

    // ERC-7540 deposit overload
    function deposit(
        uint256 assets,
        address receiver,
        address controller
    ) public returns (uint256 shares) {
        // zero deposits are not allowed
        require(assets != 0, ZeroDepositAmount());

        // TODO: do operator checks

        // calculate the shares to be minted based on provided assets
        // and the current share price
        shares = convertToShares(assets);

        _deposit(controller, receiver, assets, shares);
    }

    function _deposit(
        address caller,
        address receiver,
        uint256 assets,
        uint256 shares
    ) internal override {
        // Transfer assets to the deposit account (external contract)
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
