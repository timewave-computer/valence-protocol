> 🚧 This in-progress document contains information about the design of a cross-chain automation system.

# Introduction

The Valence Protocol is a framework designed to help you build trust-minimized applications, called Valence programs, executing across multiple chains.
Valence programs are:

- Easy to understand and quick to deploy: a program can be set up with a configuration file and no code.
- Extensible: if we don't support a DeFi integration out of the box, you can write one yourself in a matter of hours!

> 👉 **Example Use-case**:
> 
> A DAO wants to bridge tokens to another chain and then deposit the tokens into a vault. After a certain date, it wants to allow a governance proposal to trigger unwinding of the position. While the position is active, It may also want to delegate the right to change vault parameters to a specific committee as long as the parameters are within a certain range.
>
> Without Valence Programs, the DAO would have two choices:  
> **Choice 1:** Give the tokens to a multisig to execute actions on the DAO's behalf  
> **Choice 2:** Write custom smart contracts, and deployed them across multiple chains, to handle the cross-chain token operations.
>
> Valence programs offer a third choice: the DAO does not need to trust a multisig, nor does it need to spend resources writing complex cross-chain logic. Programs allow the DAO to rapidly configure and deploy a solution that meets its needs.
