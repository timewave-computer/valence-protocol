# Authorizations & Processors

The **Authorizations** and **Processor** contracts are foundational pieces of the **Valence Protocol**, as they enable on-chain (and cross-chain) execution of **Valence Programs**, and enforce access control to the programs's **subroutines** via **authorizations**.

This section goes into more details in explaining the rationale for these contracts, and shares insights about their technical implementation as well as how end-users can interact with **Valence programs** via authorizations.

## Rationale

- To have a general purpose set of smart contracts that will provide the users (anyone if the authorization is permissionless or authorization token owners if it’s permissioned) with a single point of entry to interact with the Valence program, which can have libraries and accounts deployed on multiple chains.
- To have all the user authorizations for multiple domains in a single place, making it very easy to control the application.
- To have a single address (`Processor`) that will execute the messages for all the contracts in a domain using execution queues.
- To only tick a single contract (`Processor`) which will go through the queues to route and execute the messages.
- Be able to create, edit or remove different application permissions with ease.

## Technical deep-dive:

- [Assumptions](./assumptions.md)
- [Processor Contract](./processor.md)
- [Authorization Contract](./authorization.md)
  - [Instantiation](./authorization_instantiation.md)
  - [Owner Actions](./authorization_owner_actions.md)
  - [User Actions](./authorization_user_actions.md)
- [Callbacks](./callbacks.md)