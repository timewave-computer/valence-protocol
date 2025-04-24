# Valence PancakeSwap V3 Position Manager library

The **Valence PancakeSwap V3 Position Manager** library allows **management of liquidity positions** using an **input account** and an **output account** through the [PancakeSwap V3 Protocol](https://docs.pancakeswap.finance/earn/pancakeswap-pools). It is typically used as part of a **Valence Program**. In that context, a **Processor** contract will be the main contract interacting with the PancakeSwap V3 Position Manager library.

## High-level flows

```mermaid
---
title: PancakeSwap V3 Position Manager Create Position Flow
---
flowchart LR
    P[Processor]
    PM[PancakeSwap V3
    Position Manager Library]
    IA((Input Account))
    NPM((Nonfungible
    Position Manager))
    MC((MasterChef V3))

    P -- 1/createPosition(tickLower, tickUpper, amount0, amount1) --> PM
    PM -- 2/Query balances --> IA
    PM -- 3/Approve tokens and call mint --> IA
    IA -- 4/Mint position --> NPM
    NPM -- 5/Return tokenId & mint NFT --> IA
    PM -- 6/Transfer NFT to MasterChef --> IA
    IA -- 7/Transfer NFT --> MC
```

```mermaid
---
title: PancakeSwap V3 Position Manager Withdraw Position Flow
---
flowchart LR
    P[Processor]
    PM[PancakeSwap V3
    Position Manager Library]
    IA((Input Account))
    NPM((Nonfungible
    Position Manager))
    MC((MasterChef V3))
    OA((Output Account))

    P -- 1/withdrawPosition(tokenId) --> PM
    PM -- 2/Call collectTo for fees --> IA
    IA -- 3/Collect fees --> MC
    MC -- 4/Send fees --> OA
    PM -- 5/Call harvest for rewards --> IA
    IA -- 6/Harvest rewards --> MC
    MC -- 7/Send CAKE rewards --> OA
    PM -- 8/Withdraw NFT --> IA
    IA -- 9/Request NFT withdrawal --> MC
    MC -- 10/Return NFT --> IA
    PM -- 11/Query position details --> IA
    IA -- 12/Get position details --> NPM
    PM -- 13/Decrease all liquidity --> IA
    IA -- 14/Remove liquidity --> NPM
    PM -- 15/Collect tokens --> IA
    IA -- 16/Collect tokens --> NPM
    NPM -- 17/Send tokens --> OA
    PM -- 18/Burn empty NFT --> IA
    IA -- 19/Burn NFT --> NPM
```

## Functions

| Function             | Parameters                             | Description                                                                                                                                                                                                                                                                                                      |
| -------------------- | -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **createPosition**   | tickLower, tickUpper, amount0, amount1 | Creates a position on PancakeSwap V3 by providing liquidity in a specific price range and stakes it with the **input account** in MasterChef V3. If amount0 or amount1 is 0, the entire balance of that token will be used. Returns the tokenId of the created position.                                         |
| **withdrawPosition** | tokenId                                | Performs a complete withdrawal of a position: collects accumulated fees, harvests CAKE rewards, unstakes the NFT from MasterChef, removes all liquidity, and burns the NFT. Returns the amounts of fees collected, liquidity withdrawn, and rewards received and deposits all of them in the **output account**. |

## Configuration

The library is configured on deployment using the `PancakeSwapV3PositionManagerConfig` type.

```solidity
/**
 * @notice Configuration parameters for the PancakeSwapV3PositionManager
 * @param inputAccount Account used to provide liquidity and manage positions
 * @param outputAccount Account that receives withdrawn funds and rewards
 * @param positionManager Address of PancakeSwap's NonfungiblePositionManager contract
 * @param masterChef Address of PancakeSwap's MasterChefV3 for staking NFT positions and accrue CAKE rewards
 * @param token0 Address of the first token in the pair
 * @param token1 Address of the second token in the pair
 * @param poolFeeBps Fee tier of the liquidity pool (e.g., 500 = 0.05%)
 * @param timeout Maximum time for transactions to be valid
 * @param slippageBps Maximum allowed slippage in basis points (1 basis point = 0.01%)
 */
struct PancakeSwapV3PositionManagerConfig {
    BaseAccount inputAccount;
    BaseAccount outputAccount;
    address positionManager;
    address masterChef;
    address token0;
    address token1;
    uint24 poolFeeBps;
    uint16 slippageBps; // Basis points (e.g., 100 = 1%)
    uint256 timeout;
}
```
