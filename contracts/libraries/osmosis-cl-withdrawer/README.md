# Osmosis CL liquidity withdrawer library

This contract provides the ability to liquidate concentrated liquidity
positions on Osmosis.

A single function is exposed - `withdraw_liquidity`. It takes in two
parameters:
- `position_id`, specifying the ID of the position to be liquidated
- `liquidity_amount`, specifying an optional amount of liquidity to be withdrawn
expressed in `Decimal256` format

## Validations

During the library validation, both input and output addresses are validated.
In addition, a sanity check is performed on the specified pool id to ensure
that the pool indeed exists.

If `liquidity_amount` is specified, it gets validated to be less than or equal
to the total liquidity of the position.

## Function

On function execution, the only explicit validation performed is that of ensuring
that the position exists.
Any errors beyond that are propagated from the cl module. These may involve:
- position ownership error
- insufficient liquidity error
- other errors related to the osmosis concentrated liquidity module

After that, `MsgWithdrawPosition` is executed as a submessage on behalf of
the input account.

On reply, valence callback is used to extract the `MsgWithdrawPositionResponse`
which contains the amount of tokens that were successfully withdrawn.

With that, the final (bank send) message is fired to the input account.
This message transfers the withdrawn tokens to the output account.
