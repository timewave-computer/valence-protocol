# Valence Program examples

This directory contains a set of polished Valence Programs meant to provide an overview of the system.

## Available Examples

Under `/src` you will find the following Valence Program examples:
- `conditional_lp`: conditional liquidity provision on a remote domain dex

## Running the examples locally

In order to run the examples locally on your machine, the environment needs to be set up.
See intructions under `e2e/README.md` for steps on how that can be done.

Once `local-ic` is available in your path and is running the correct set of nodes,
examples can be executed by running `just run-example <example>`.

For example, in order to run the `conditional_lp` Program, execute the following:

```just
just run-example conditional_lp
```
