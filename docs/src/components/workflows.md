# Workflows

- **Valence Workflow:** A workflow is an instance of the Valence Protocol. It is a particular arrangement and configuration of _ValenceAccounts_ and _Services_ across multiple _domains_. Workflows are also associated with a set of _ActionBundles_ that can be executed on the Workflow. For example, a POL lending relationship between two parties may be set up as a workflow.

- **Domain:** Environments where Valence Accounts or Services can be instantiated. Domains are defined by three properties:
  1. Chain _(e.g., Neutron, Osmosis, Ethereum mainnet)_
  2. Execution environment _(e.g., CosmWasm, EVM, SVM)_
  3. Bridge from main domain _(e.g., Polytone over IBC, Hyperlane)_
- **Main Domain:** Every workflow has a main domain where the _Authorizations_ module is instantiated.

- **Workflow Manager:** Off-chain service that manages the configuration, instantiation, and update management for workflows.
