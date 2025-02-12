# Execution Environment Differences

Depending on the type of `Execution Environment` being used, the behavior of the Processor may vary. In this section we will describe the main differences in how the Processor behaves in the different `Execution Environments` that we support.

### Execution Success

During the execution of a `MessageBatch`, the Processor will execute each function of the subroutine of that batch. If the execution for a specific function fails, we will consider the execution failed in case of `Atomic` batches, and we will stop the execution of the next function in case of `NonAtomic` batches.

Currently, in the `CosmWasm` execution environment, a function fails if the `CosmWasm` contract that we are targeting doesn't exist, if the `entry point` of that contract doesn't exist, or if the execution of the contract fails for any reason. On the contract, in the `EVM` execution environment, a function only fails if the contract explicitly fails or reverts.

To mitigate the differences in behavior between these two execution environments, an `EVM` Processor check was included to check if the contract indeed exists and fail execution if the contract does not exist. Behavior was also added in the `EVM` libraries to revert if the execution of the contract enters the fallback function, which is not allowed in the system. Nevertheless, since Processors are not restricted to `Valence Libraries` but can call any contract, no guarantee can be made that the contract targeted will fail if an entry point does not exist, because the fallback function might not be defined or might not revert.

In `CosmWasm`, execution of a contract will always fail if the entry point does not exist. However, for `EVM` execution, this is not necessarily the case. This is a difference that the owner of the program must take into account when designing and creating their program.

**_In summary_**: if a function of the subroutine targets a contract that meets all of the following conditions:
- It is not a `Valence Library`.
- The entry point of that contract does not exist.
- The fallback function is either not defined or doesn't explicitly revert.

The execution of that function will be considered successful in the `EVM` execution environment but not in the `CosmWasm` execution environment equivalent.
