# Library account type

When we build a new program, we don't yet have an on-chain address, but there are several components that require an address to operate, for example a library needs to know the input account address it should operate on.

When building a fresh program config, we are using ids instead of addresses, the manager first predicts all the addresses of to-be instantiated contracts, and replace the ID with the address where an id was used.

To achieve this we are using the `LibraryAccountType` that first uses an id, and allows us to replace it with an address later when this contract was instantiated.

```rust
pub enum LibraryAccountType {
    #[serde(rename = "|library_account_addr|", alias = "library_account_addr")]
    Addr(String),
    #[serde(rename = "|account_id|", alias = "account_id")]
    AccountId(Id),
    #[serde(rename = "|library_id|", alias = "library_id")]
    LibraryId(Id),
}
```

`LibraryAccountType` is an enum that includes 3 options:

- `Addr(String)` - Already instantiated on-chain address, this means we should not replace it
- `AccountId(Id)` - Account id that should be replaced with the address of an account
- `LibraryId(Id)` - Library id that should be replaced with the address of a library

## Methods

### to_string() -> StdResult<Addr>

If `LibraryAccountType:Addr`, we return the address as a string.

```rust
let addr = LibraryAccountType::Addr("some_addr".to_string());

let addr_string = addr.to_string();

assert_eq!(addr_string, "some_addr")
```

Will error if `LibraryAccountType` is an id.

### to_addr(api: &dyn cosmwasm_std::Api) -> StdResult<cosmwasm_std::Addr>

Returns the address in `cosmwasm_std::Addr` type

```rust
let addr = LibraryAccountType::Addr("some_addr".to_string());
let api = mock_api();

let addr_as_addr = addr.to_addr(&api);

assert_eq!(addr_as_addr, cosmwasm_std::Addr::unchecked("some_addr"))
```

Will error if `LibraryAccountType` is an id.

### to_raw_placeholder() -> String

Although it is encouraged for libraries to accept the `LibraryAccountType` directly as an address, some libraries may require a `Strin`. 

`to_raw_placeholder` allows us to still use account ids in library config where a `String` is expected.

```rust
struct LibraryConfig {
    addr: String,
}
let addr_id = LibraryAccountType::AccountId(1);

let library_config = LibraryConfig { addr: addr_id.to_raw_placeholder() }

// Here is library config before passing to the manager:
// LibraryConfig { addr: "|lib_acc_placeholder|:1" }

init_program(&mut program_config);

// Here is the library config after instantiation:
// LibraryConfig { addr: "addres_of_account_id_1" }
```

### from_str(input: &str) -> Result<Self, String>

You can get `LibraryAccountType::Addr` from a string

```rust
let addr = "some_addr";

let LAT = LibraryAccountType::from(addr);
let LAT: LibraryAccountType = addr.into();

// Both are equal to `LibraryAccountType::Addr("some_addr".to_string())`
```

### get_account_id(&self) -> Id

Gets the id if `LibraryAccountType::AccountId`, else it panics.

### get_library_id(&self) -> Id;

Gets the id if `LibraryAccountType::LibraryId`, else it panics.