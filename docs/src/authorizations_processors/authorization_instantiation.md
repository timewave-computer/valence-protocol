# Instantiation

When the contract is instantiated, it will be provided the following information:

- Processor contract on main domain.

- Owner of the contract.

- List of subowners (if any). Users that can execute the same actions as the owner except adding/removing other subowners.

Once the Authorization contract is deployed, we can already start adding and executing authorizations on the domain that the Authorization contract was deployed on. To execute functions on other domains, the owner will have to add external domains to the Authorization contract with all the information required for the Authorization contract to route the messages to that domain.
