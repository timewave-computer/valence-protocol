# Nolus Lending library

The **Valence Nolus Lending** library enables lending on Nolus from a **input account**. Also, the library allows **withdrawing lent assets** from the **input account** on Nolus and depositing the withdrawed tokens into an **output account**.

## Configuration

The library is configured on instantiation via the `LibraryConfig` type.

```rust
pub struct LibraryConfig {
    // Address of the input account 
    pub input_addr: LibraryAccountType,
    /// Address of the output account
    pub output_addr: LibraryAccountType,
    // Address of the pool contract
    pub pool_addr: String,
    // Denom of the asset we are going to lend
    pub denom: String,
}
```
