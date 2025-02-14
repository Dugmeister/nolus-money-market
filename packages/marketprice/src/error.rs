use std::{convert::Infallible, num::TryFromIntError};

use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum PriceFeedsError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Given address already registered as a price feeder")]
    FeederAlreadyRegistered {},

    #[error("Given address not registered as a price feeder")]
    FeederNotRegistered {},

    #[error("No price")]
    NoPrice(),

    #[error("Invalid price")]
    InvalidPrice(),

    #[error("Found currency {0} expecting {1}")]
    UnexpectedCurrency(String, String),

    #[error("{0}")]
    FromInfallible(#[from] Infallible),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("{0}")]
    TryFromInt(#[from] TryFromIntError),

    #[error("{0}")]
    Finance(#[from] finance::error::Error),

    #[error("Unknown currency")]
    UnknownCurrency {},

    #[error("{0}")]
    FeedSerdeError(String),
}

impl From<postcard::Error> for PriceFeedsError {
    fn from(err: postcard::Error) -> Self {
        Self::FeedSerdeError(format!("Error during (de-)serialization: {}", err))
    }
}

pub(crate) fn config_error_if(check: bool, msg: &str) -> Result<(), PriceFeedsError> {
    if check {
        Err(PriceFeedsError::Configuration(msg.into()))
    } else {
        Ok(())
    }
}
