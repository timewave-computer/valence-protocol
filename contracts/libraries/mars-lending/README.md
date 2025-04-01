# Mars Lending library

The **Valence Mars Lender** library enables lending on Mars by funding a mars credit account from an **input account**. The credit account, owned by the **output account**, then uses the funds to lend assets.

## Configuration

The library is configured on instantiation via the `LibraryConfig` type.

```rust
pub struct LibraryConfig {
    // Address of the input account 
    pub input_addr: LibraryAccountType,
    /// Address of the output account
    pub output_addr: LibraryAccountType,
    // Address of the credit manager contract
    pub credit_manager_addr: String,
    // Denom of the asset we are going to land
    pub denom: String,
}
```
