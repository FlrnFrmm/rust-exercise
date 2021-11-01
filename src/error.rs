use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Path to input file not given as argument")]
    NoInputArgument,
    #[error("Transaction has invalid type `{0}`")]
    InvalidRawTransactionType(String),
    #[error("Amount can't be None in deposit transaction")]
    NoAmountInDeposit,
    #[error("Amount can't be None in withdrawal transaction")]
    NoAmountInWitdrawal,
}
