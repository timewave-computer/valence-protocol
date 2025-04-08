use alloy_sol_types::sol;

sol! {
    struct AavePositionManagerConfig {
        address poolAddress;
        address inputAccount;
        address outputAccount;
        address supplyAsset;
        address borrowAsset;
        uint16 referralCode;
    }

    function supply(uint256 amount) external;
    function borrow(uint256 amount) external;
    function withdraw(uint256 amount) external;
    function repay(uint256 amount) external;
    function repayWithShares(uint256 amount) external;
}
