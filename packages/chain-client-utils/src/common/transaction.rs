use cosmos_sdk_proto::cosmos::base::abci::v1beta1::TxResponse;

use super::error::StrategistError;

#[derive(Debug)]
pub struct TransactionResponse {
    pub hash: String,
    pub success: bool,
    pub block_height: u64,
    pub gas_used: u64,
}

impl TryFrom<TxResponse> for TransactionResponse {
    type Error = StrategistError;

    fn try_from(value: TxResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            hash: value.txhash,
            success: true,
            block_height: u64::try_from(value.height)?,
            gas_used: u64::try_from(value.gas_used)?,
        })
    }
}

impl TryFrom<Option<TxResponse>> for TransactionResponse {
    type Error = StrategistError;

    fn try_from(value: Option<TxResponse>) -> Result<Self, Self::Error> {
        match value {
            Some(tx) => Self::try_from(tx),
            None => Err(StrategistError::TransactionError(
                "failed to find tx_response".to_string(),
            )),
        }
    }
}
