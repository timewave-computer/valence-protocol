#[derive(Debug)]
pub enum StrategistError {
    ClientError(String),
    QueryError(String),
    ParseError(String),
    TransactionError(String),
}
