# Execution Environment Differences

Depending on the type of `Execution Environment` being used, the behavior of the `Processor` may vary. In this section we will describe the main differences in how the `Processor` behaves in the different `Execution Environments` that we support.

### Execution Success

During the execution of a `Message Batch`, the `Processor` will execute each function of the subroutine of that batch. If the execution for a specific function fails, we will consider the execution failed in case of `Atomic` batches, and we will stop the execution of the next function in case of `NonAtomic` batches.

Currently, in the `CosmWasm` Execution Environment, a function fails if the `CosmWasm` contract that we are targeting doesn't exist, if the `entry point` of that contract doesn't exist, or if the execution of the contract fails for any reason. On the contract, in the `EVM` Execution Environment, a function only fails if the contract explicitly fails or reverts.

To mitigate the differences in behavior between these two `Execution Environments`, we have added a check in the `EVM` Processor version to consider the execution failed if the contract doesn't exist by explicitly checking if the contract indeed exists. We've also added a revert in our `EVM` libraries if the execution of the contract enters the fallback function, which is not allowed in our system. Nevertheless, since the `Processors` are not restricted to `Valence Libraries` but can call any contract, we can't guarantee that the contract targeted will fail if an entry point doesn't exist, because the fallback function might not be defined or might not revert.

Therefore, whereas in `CosmWasm` executions, a contract will always fail if the entry point doesn't exist, in `EVM` executions, this is not necessarily the case. This is a difference that the owner of the program needs to be taken into account when designing and creating this program.

***In summary***: if a function of the subroutine targets a contract that is NOT a `Valence Library` AND the entry point of that contract doesn't exist AND the fallback function is either not defined or doesn't revert, the execution of that function will be considered successful in the `EVM` Execution Environment while it wouldn't be in the `CosmWasm` Execution Environment equivalent.
