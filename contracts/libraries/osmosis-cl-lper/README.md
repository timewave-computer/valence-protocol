# Osmosis CL liquidity provider library

This contract provides the ability to create concentrated liquidity
positions on Osmosis.

Because of the way CL positions are created, there are two ways to achieve it.

## Default

Default position creation centers around the idea of creating a position
with respect to the currently active tick of the pool.

This method expects a single parameter, `bucket_amount`, which describes
how many *buckets* of the pool should be taken into account to both sides
of the price curve.

Consider a situation where the current tick is 125, and the configured
tick spacing is 10.

If this method is called with `bucket_amount` set to 5, the following logic
will be performed:
- find the current bucket range, which is 120 to 130
- extend the current bucket ranges by 5 buckets to both sides, meaning
that the range "to the left" will be extended by 5 * 10 = 50, and the
range "to the right" will be extended by 5 * 10 = 50, resulting in the covered
range from 120 - 50 = 70 to 130 + 50 = 180, giving the position tick range of (70, 180).

## Custom

Custom position creation allows for more fine-grained control over the
way the position is created.

This approach expects users to specify the following parameters:
- `tick_range`, which describes the price range to be covered
- `token_min_amount_0` and `token_min_amount_1` which are optional
parameters that describe the minimum amount of tokens that should be
provided to the pool.

With this flexibility a wide variety of positions can be created, such as
those that are entirely single-sided.
