# Mars Lending library

The **Valence Mars Lending** library facilitates lending operations on Mars. It allows users to create and fund a Mars credit account from their input account. This credit account, which remains owned by the input account, then manages the **lending** of these assets. Additionally, the library supports the **withdrawal** of lent assets from the credit account, automatically depositing the retrieved tokens into the **output account**

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
    // Denom of the asset we are going to lend
    pub denom: String,
}
```
