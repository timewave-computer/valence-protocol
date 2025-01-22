// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {ERC4626, ValenceVault} from "../../src/libraries/ValenceVault.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {Math} from "@openzeppelin-contracts/utils/math/Math.sol";

abstract contract VaultHelper is Test {
    using Math for uint256;

    // Constants
    uint256 internal constant BASIS_POINTS = 10000;
    uint256 internal constant INITIAL_USER_BALANCE = 100_000_000_000;
    uint32 internal constant MAX_WITHDRAW_FEE = 2000;
    uint256 internal constant INITIAL_TIMESTAMP = 5000;
    uint256 internal constant INITIAL_BLOCK = 100;

    // Contracts
    ValenceVault internal vault;
    BaseAccount internal depositAccount;
    BaseAccount internal withdrawAccount;
    MockERC20 internal token;

    // Addresses
    address internal owner;
    address internal strategist;
    address internal user;

    // Address for fee distribution
    address internal strategistFeeAccount;
    address internal platformFeeAccount;

    // Events
    event Deposit(address indexed sender, address indexed owner, uint256 assets, uint256 shares);

    function setUp() public virtual {
        // Setup addresses
        owner = makeAddr("owner");
        strategist = makeAddr("strategist");
        user = makeAddr("user");
        strategistFeeAccount = makeAddr("strategistFeeAccount");
        platformFeeAccount = makeAddr("platformFeeAccount");

        // Setup initial block and time
        vm.warp(INITIAL_TIMESTAMP);
        vm.roll(INITIAL_BLOCK);

        // Deploy contracts
        vm.startPrank(owner);
        token = new MockERC20("Test Token", "TEST");
        depositAccount = new BaseAccount(owner, new address[](0));
        withdrawAccount = new BaseAccount(owner, new address[](0));

        // Setup vault configuration
        ValenceVault.VaultConfig memory config = ValenceVault.VaultConfig({
            depositAccount: depositAccount,
            withdrawAccount: withdrawAccount,
            strategist: strategist,
            depositCap: 0,
            maxWithdrawFee: MAX_WITHDRAW_FEE,
            fees: defaultFees(),
            feeDistribution: defaultDistributionFees()
        });

        vault = new ValenceVault(owner, abi.encode(config), address(token), "Valence Vault Token", "VVT");

        // Setup permissions
        depositAccount.approveLibrary(address(vault));
        withdrawAccount.approveLibrary(address(vault));
        vm.stopPrank();

        // Setup user state
        vm.startPrank(owner);
        token.mint(user, INITIAL_USER_BALANCE);
        vm.stopPrank();

        vm.startPrank(user);
        token.approve(address(vault), type(uint256).max);
        vm.stopPrank();
    }

    function defaultFees() public pure returns (ValenceVault.FeeConfig memory) {
        return ValenceVault.FeeConfig({depositFeeBps: 0, platformFeeBps: 0, performanceFeeBps: 0});
    }

    function defaultDistributionFees() public view returns (ValenceVault.FeeDistributionConfig memory) {
        return ValenceVault.FeeDistributionConfig({
            strategistAccount: strategistFeeAccount,
            platformAccount: platformFeeAccount,
            strategistRatioBps: 3000
        });
    }

    function setFeeDistribution(address strategistAccount, address platformAccount, uint32 strategistRatioBps)
        internal
    {
        vm.startPrank(owner);

        (
            BaseAccount _depositAccount,
            BaseAccount _withdrawAccount,
            address _strategist,
            uint256 _depositCap,
            uint32 _maxWithdrawFee,
            ValenceVault.FeeConfig memory _fees,
        ) = vault.config();

        ValenceVault.FeeDistributionConfig memory feeDistConfig = ValenceVault.FeeDistributionConfig({
            strategistAccount: strategistAccount,
            platformAccount: platformAccount,
            strategistRatioBps: strategistRatioBps
        });

        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig({
            depositAccount: _depositAccount,
            withdrawAccount: _withdrawAccount,
            strategist: _strategist,
            depositCap: _depositCap,
            maxWithdrawFee: _maxWithdrawFee,
            fees: _fees,
            feeDistribution: feeDistConfig
        });

        vault.updateConfig(abi.encode(newConfig));
        vm.stopPrank();
    }

    function setFees(uint32 depositFee, uint32 platformFee, uint32 performanceFee) internal {
        vm.startPrank(owner);
        ValenceVault.FeeConfig memory feeConfig = ValenceVault.FeeConfig({
            depositFeeBps: depositFee,
            platformFeeBps: platformFee,
            performanceFeeBps: performanceFee
        });

        (
            BaseAccount _depositAccount,
            BaseAccount _withdrawAccount,
            address _strategist,
            uint256 _depositCap,
            uint32 _maxWithdrawFee,
            ,
            ValenceVault.FeeDistributionConfig memory _feeDistribution
        ) = vault.config();

        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig(
            _depositAccount, _withdrawAccount, _strategist, _depositCap, _maxWithdrawFee, feeConfig, _feeDistribution
        );

        vault.updateConfig(abi.encode(newConfig));
        vm.stopPrank();
    }

    function setDepositCap(uint256 newCap) internal {
        vm.startPrank(owner);
        (
            BaseAccount _depositAccount,
            BaseAccount _withdrawAccount,
            address _strategist,
            ,
            uint32 _maxWithdrawFee,
            ValenceVault.FeeConfig memory _fees,
            ValenceVault.FeeDistributionConfig memory _feeDistribution
        ) = vault.config();

        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig(
            _depositAccount, _withdrawAccount, _strategist, newCap, _maxWithdrawFee, _fees, _feeDistribution
        );

        vault.updateConfig(abi.encode(newConfig));
        vm.stopPrank();
    }
}
