/// error type to be returned by all client types.
#[derive(Debug, thiserror::Error)]
pub enum StrategistError {
    #[error("client error: {0}")]
    ClientError(String),
    #[error("query error: {0}")]
    QueryError(String),
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("transaction error: {0}")]
    TransactionError(String),
}
