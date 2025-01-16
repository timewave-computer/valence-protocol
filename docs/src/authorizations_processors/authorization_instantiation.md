# Instantiation

When the contract is instantiated, it will be provided the following information:

- Processor contract on main domain.

- `[(Domain, Connector(Polytone_note_contract), Processor_contract_on_domain, callback_proxy, IBC_Timeout_settings)]`: If it's a cross domain application, an array will be passed with each external domain label and its corresponding connector contracts and proxies that will be instantiated before hand. For each connector, there will be also a proxy corresponding to that external domain because it’s a two-way communication flow and we need to receive callbacks. Additionally, we need a set of `Timeout` settings for the bridge, to know for how long the messages sent through the connector are going to be valid.

- Admin of the contract (if different to sender).

The instantiation will set up all the processors on each domain so that we can start instantiating the libraries afterwards and providing the correct `Processor` addresses to each of them depending on which domain they are in.
