# Mars Withdrawer library

The **Valence Mars Withdrawer library** allows to **withdrawing lent assets** from a credit account on Mars owned by an **input account** and depositing the withdrawed tokens into an **output account**.

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
    // Denom of the asset we are going to withdraw
    pub denom: String,
}
```