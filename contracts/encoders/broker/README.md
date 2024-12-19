# Valence Encoder Broker

The **Valence Encoder Broker** is a smart contract that manages and multiplexes message encoders across different versions. It implements an ownership model and maintains a registry of specialized encoders for a specific virtual machine (e.g. EVM, SVM ...) according to their version.
The broker serves one primary functions: it directs encoding requests to the appropriate encoder based on the version of the encoder requested