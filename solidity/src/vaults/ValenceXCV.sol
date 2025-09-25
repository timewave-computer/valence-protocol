// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {ERC4626Upgradeable} from "@openzeppelin-contracts-upgradeable/token/ERC20/extensions/ERC4626Upgradeable.sol";

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {OwnableUpgradeable} from "@openzeppelin-contracts-upgradeable/access/OwnableUpgradeable.sol";
import {ReentrancyGuardUpgradeable} from "@openzeppelin-contracts-upgradeable/utils/ReentrancyGuardUpgradeable.sol";
import {UUPSUpgradeable} from "@openzeppelin-contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import {Initializable} from "@openzeppelin-contracts-upgradeable/proxy/utils/Initializable.sol";

contract ValenceXCV is
    Initializable,
    ERC4626Upgradeable,
    OwnableUpgradeable,
    ReentrancyGuardUpgradeable,
    UUPSUpgradeable
{
    uint256 public sharePrice;

    constructor() {
        _disableInitializers();
    }

    function initialize(
        address _owner,
        address underlying,
        string memory vaultTokenName,
        string memory vaultTokenSymbol,
        uint256 startSharePrice
    ) external initializer {
        __ERC20_init(vaultTokenName, vaultTokenSymbol);
        __ERC4626_init(IERC20(underlying));
        __Ownable_init(_owner);
        __ReentrancyGuard_init();
        __UUPSUpgradeable_init();

        sharePrice = startSharePrice;
    }

    /**
     * @dev Function that authorizes contract upgrades - required by UUPSUpgradeable
     * @param newImplementation address of the new implementation
     */
    function _authorizeUpgrade(
        address newImplementation
    ) internal override onlyOwner {
        // Upgrade logic comes here
        // No additional logic required beyond owner check in modifier
    }
}
