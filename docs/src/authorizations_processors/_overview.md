# Authorizations & Processors

The **Authorizations** and **Processor** contracts are foundational pieces of the **Valence Protocol**, as they enable on-chain (and cross-chain) execution of **Valence Programs** and enforce access control to the program's **Subroutines** via **Authorizations**.

This section explains the rationale for these contracts and shares insights about their technical implementation, as well as how end-users can interact with **Valence Programs** via **Authorizations**.

## Rationale

- To have a general purpose set of smart contracts that provide users with a single point of entry to interact with the Valence Program, which can have libraries and accounts deployed on multiple chains.
- To have all the user authorizations for multiple domains in a single place, making it very easy to control the application.
- To have a single address (`Processor`) that will execute the messages for all the contracts in a domain using execution queues.
- To only tick a single contract (`Processor`) that will go through the queues to route and execute the messages.
- To create, edit, or remove different application permissions with ease.

## Technical deep-dive:

- [Assumptions](./assumptions.md)
- [Processor Contract](./processor.md)
- [Authorization Contract](./authorization.md)
  - [Instantiation](./authorization_instantiation.md)
  - [Owner Actions](./authorization_owner_actions.md)
  - [User Actions](./authorization_user_actions.md)
- [Callbacks](./callbacks.md)
