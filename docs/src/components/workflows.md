# Programs

- **Valence Program:** A program is an instance of the Valence Protocol. It is a particular arrangement and configuration of _ValenceAccounts_ and _Services_ across multiple _domains_. Programs are also associated with a set of _ActionBundles_ that can be executed on the Program. For example, a POL lending relationship between two parties may be set up as a program.

- **Domain:** Environments where Valence Accounts or Services can be instantiated. Domains are defined by three properties:
  1. Chain _(e.g., Neutron, Osmosis, Ethereum mainnet)_
  2. Execution environment _(e.g., CosmWasm, EVM, SVM)_
  3. Bridge from main domain _(e.g., Polytone over IBC, Hyperlane)_
- **Main Domain:** Every program has a main domain where the _Authorizations_ module is instantiated.

- **Program Manager:** Off-chain service that manages the configuration, instantiation, and update management for programs.
