/// error type to be returned by all client types.
#[derive(Debug)]
pub enum StrategistError {
    ClientError(String),
    QueryError(String),
    ParseError(String),
    TransactionError(String),
}
