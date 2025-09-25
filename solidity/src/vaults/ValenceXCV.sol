// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {ERC4626Upgradeable} from "@openzeppelin-contracts-upgradeable/token/ERC20/extensions/ERC4626Upgradeable.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {OwnableUpgradeable} from "@openzeppelin-contracts-upgradeable/access/OwnableUpgradeable.sol";
import {Initializable} from "@openzeppelin-contracts-upgradeable/proxy/utils/Initializable.sol";

contract ValenceXCV is Initializable, ERC4626Upgradeable, OwnableUpgradeable {
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
    }

    function setSharePrice(uint256 newSharePrice) external onlyStrategist {
        // setting share price to 0 would allow for infinite mint
        if (newSharePrice == 0) revert InvalidSharePrice();

        sharePrice = newSharePrice;

        emit SharePriceUpdated(newSharePrice);
    }
}
