use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Swap] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Swap] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Swap] The value {0} is an invalid amount")]
    InvalidAmount(String),

    #[error("[Swap] Expected response to {0} is not found")]
    MissingResponse(String),
}

pub type Result<T> = core::result::Result<T, Error>;
