use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::{
    MsgTransferPositions, MsgTransferPositionsResponse,
};
use osmosis_test_tube::{fn_execute, Module, Runner};

#[cfg(test)]
mod test_suite;
#[cfg(test)]
mod tests;

pub struct ConcentratedLiquidityExts<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for ConcentratedLiquidityExts<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> ConcentratedLiquidityExts<'a, R>
where
    R: Runner<'a>,
{
    // transfer CL position
    fn_execute! { pub transfer_positions: MsgTransferPositions => MsgTransferPositionsResponse }
}
