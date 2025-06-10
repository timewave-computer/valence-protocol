# Elys Lending library

The **Valence Elys Lending** library enables lending on Elys from a **input account**. Also, the library allows **withdrawing lent assets** from the **input account** on Elys and depositing the withdrawed tokens into an **output account**. Additionally, users can **claim rewards** from the **input account**, and the rewards will also be sent to the **output account** upon claiming them.

## Configuration

The library is configured on instantiation via the `LibraryConfig` type.

```rust
pub struct LibraryConfig {
    /// Address of the input account
    pub input_addr: LibraryAccountType,
    /// Address of the output account
    pub output_addr: LibraryAccountType,
    /// ID of the pool we are going to lend to
    pub pool_id: Uint64,
}
```
