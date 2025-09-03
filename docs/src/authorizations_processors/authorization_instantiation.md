# Instantiation

Instantiation parameters vary slightly between CosmWasm and EVM.

## CosmWasm

The Authorization contract is instantiated with:

- Processor contract address on the main domain
- Owner address
- Optional list of sub‑owners (second‑tier owners who can perform all actions except sub‑owner management)

Once deployed, authorizations can be created and executed on the main domain. To execute on other domains, the owner adds external domains with connector details (Polytone for CosmWasm domains; Hyperlane + encoder info for EVM domains).

## EVM

`constructor(address owner, address processor, bool storeCallbacks)`

- `owner`: the contract owner (Ownable)
- `processor`: the Processor contract address
- `storeCallbacks`: whether to persist processor callbacks on‑chain (otherwise only events are emitted)

EVM does not use sub‑owners; instead, the owner can add or remove admin addresses that are permitted to perform privileged updates. Cross‑domain routing is handled via Hyperlane mailboxes (set during Processor deployment), not at Authorization instantiation time.
