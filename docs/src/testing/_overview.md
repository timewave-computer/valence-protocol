# Testing your programs

Our testing infrastructure is built on several tools that work together to provide a comprehensive local testing environment:

### Core Testing Framework

We use [local-interchain](https://github.com/strangelove-ventures/interchaintest/tree/main/local-interchain), a component of the [interchaintest](https://github.com/strangelove-ventures/interchaintest) developer toolkit. This allows you to deploy and run chains in a local environment, providing a controlled testing space for your blockchain applications.

### Localic Utils

To make these tools more accessible in Rust, we've developed [localic-utils](https://github.com/timewave-computer/localic-utils). This Rust library provides convenient interfaces to interact with the local-interchain testing framework.

### Program Manager

We provide a tool called `Program Manager` that helps you manage your programs. We've created all the abstractions and helper functions to create your programs more efficiently together with local-interchain.

The Program Manager use is optional, it abstracts a lot of functionality and allows creating programs in much less code. But if you want to have more fine-grained control over your programs, we provide helper functions to create and interact with your programs directly without it. In this section, we'll show you two different examples on how to test your programs, one using the Program Manager and the other without it. There are also many more examples each of them for different use cases. They are all in the `examples` folder of [our e2e folder](https://github.com/timewave-computer/valence-protocol/tree/main/e2e).
