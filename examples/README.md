# Valence Program examples

This directory contains a set of polished Valence Programs meant to provide an overview of
Valence Protocol in action.

These examples are built upon the utilities provided in the `e2e` directory.
If you wish to start developing your own Valence Programs, it is recommended to get
familiar with the contents of that directory.

## Available Examples

Under `/src` you will find the following Valence Program examples:
- `osmo_cl`: creation and liquidation of a Concentrated Liquidity position on Osmosis
- `osmo_gamm`: creation and liquidation of a GAMM (xyk) position on Osmosis
- `token_swap`: token swap between two parties

## Running the examples locally

Testing environment needs to be set up in order to run the examples locally on your machine.

See intructions under `e2e/README.md` for steps on how that can be done.

Once `local-ic` is available in your path and is running the correct set of nodes,
examples can be executed by running `just run-example <example>`.

For example, in order to run the `osmo_cl` Program, execute the following:

```just
just run-example osmo_cl
```
