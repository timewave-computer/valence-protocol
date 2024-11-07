> 🚧 This in-progress document contains information about the design of a cross-chain automation system that powers the Valence programs. Lending applications can be implemented as specific "program".

# Background

Programs are a framework to help you develop trust-minimized applications which require sequential execution across many chains. They are easy to understand and quick to deploy. A Program can be set up with a configuration file and no code. Programs are also extensible. If we don't support a DeFi integration out of the box, you can write one yourself in a matter of hours!

> 👉 **Example Use-case**:  
> A DAO wants to bridge tokens to another chain and then deposit the tokens into a vault. After a certain date, it wants to allow a governance proposal to trigger unwinding of the position. While the position is active, It may also want to delegate the right to change vault parameters to a specific committee as long as the parameters are within a certain range.
>
> Without Programs, the DAO would have two choices:  
> **Choice 1:** Give the tokens to a multisig to execute actions on the DAO's behalf  
> **Choice 2:** Write highly custom smart contracts across multiple chains that handle the token operations across multiple chains.
>
> Programs offer a third choice: the DAO does not need to trust a multisig, nor does it need to spend resources writing complex cross-chain logic. Programs allow the DAO to rapidly configure and deploy a solution that meets its needs.
