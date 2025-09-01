# Vortex LPer library

The **Valence Vortex LPer library** allows users to **deposit** into an Osmosis pool via Vortex contract from an **input account**. Also, the library allows **withdrawing from position** via vortex contract and receiving the withdrawn tokens into an **output account** and **output account_2** (principal and counterparty tokens).

## Configuration

The library is configured on instantiation via the `LibraryConfig` type.

```rust
pub struct LibraryConfig {
    /// Address of the input account 
    pub input_addr: LibraryAccountType,
    /// Address of the output account 
    pub output_addr: LibraryAccountType,
    /// Address of the second output account 
    pub output_addr_2: LibraryAccountType,
    /// Configuration for the liquidity provider
    /// This includes the pool address and asset data
    pub lp_config: LiquidityProviderConfig,
}

pub struct LiquidityProviderConfig {
    /// Code of the vortex contract we are going to instantiate
    pub vortex_code: u64,
    /// Label for the contract instantiation
    pub label: String,
    /// Id of the pool we are going to provide liquidity for
    pub pool_id: u64,
    /// Duration of the round in seconds
    pub round_duration: u64,
    /// Duration of the auction in seconds
    pub auction_duration: u64,
    /// Denoms of both assets we are going to provide liquidity for
    pub asset_data: AssetData,
    /// Whether the principal token is first in the pool
    pub principal_first: bool,
}
```
