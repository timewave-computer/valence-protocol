# Assumptions

- **Funds**: You cannot send funds with the messages.

- **Bridging**: For programs that optionally span multiple domains, we assume that messages can be sent and confirmed bidirectionally between domains. The Authorization contract on the main domain communicates with the processor in a different domain in one direction and the callback confirming the correct or failed execution in the other direction.

- **Instantiation**: All these contracts can be instantiated beforehand and off-chain having predictable addresses. Here is an example instantiation flow using Polytone:
  - Predict `authorization` contract address.
  - Instantiate polytone contracts & set up relayers.
  - Predict `proxy` contract address for the `authorization` contract on each external domain.
  - Predict `proxy` contract address on the main domain for each processor on external domains.
  - Instantiate all `processors`. The sender on external domains will be the predicted `proxy` and on the main domain it will be the Authorization contract iself.
  - Instantiate Authorization contract with all the processors and their predicted proxies for external domains and the processor on the main domain.

- **Relaying**: Relayers will be running once everything is instantiated.

- **Tokenfactory**: The main domain has the token factory module with no token creation fee so that we can create and mint these nonfungible tokens with no additional cost.

- **Domains**: In the current version, actions in each authorization will be limited to a single domain.
