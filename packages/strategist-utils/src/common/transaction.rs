#[derive(Debug)]
pub struct TransactionResponse {
    pub hash: String,
    pub success: bool,
    pub block_height: Option<u64>,
    pub gas_used: Option<u64>,
}
