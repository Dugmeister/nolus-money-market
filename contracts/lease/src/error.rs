use std::{any::type_name, fmt::Display};

use thiserror::Error;

use finance::error::Error as FinanceError;
use lpp::error::ContractError as LppError;
use oracle::error::ContractError as OracleError;
use platform::error::Error as PlatformError;
use profit::error::ContractError as ProfitError;
use sdk::cosmwasm_std::StdError;
use swap::error::Error as SwapError;
use timealarms::error::ContractError as TimeAlarmsError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[Lease] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Lease] Unauthorized")]
    Unauthorized {},

    #[error("[Lease] {0}")]
    FinanceError(#[from] FinanceError),

    #[error("[Lease] {0}")]
    PlatformError(#[from] PlatformError),

    #[error("[Lease] {0}")]
    LppError(#[from] LppError),

    #[error("[Lease] {0}")]
    TimeAlarmsError(#[from] TimeAlarmsError),

    #[error("[Lease] {0}")]
    OracleError(#[from] OracleError),

    #[error("[Lease] {0}")]
    ProfitError(#[from] ProfitError),

    #[error("[Lease] {0}")]
    SwapError(#[from] SwapError),

    #[error("[Lease] No downpayment sent")]
    NoDownpaymentError(),

    #[error("[Lease] The underlying loan is not fully repaid")]
    LoanNotPaid(),

    #[error("[Lease] The underlying loan is closed")]
    LoanClosed(),

    #[error("[Lease] Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("[Lease] The operation '{0}' is not supported in state '{1}'")]
    UnsupportedOperation(String, String),

    #[error("[Finance] Programming error or invalid serialized object of '{0}' type, cause '{1}'")]
    BrokenInvariant(String, String),
}

impl ContractError {
    pub fn unsupported_operation<Op, State>(op: Op, state: &State) -> Self
    where
        Op: Into<String>,
        State: Display,
    {
        Self::UnsupportedOperation(op.into(), format!("{}", state))
    }

    pub fn broken_invariant_if<T>(check: bool, msg: &str) -> ContractResult<()> {
        if check {
            Err(Self::BrokenInvariant(type_name::<T>().into(), msg.into()))
        } else {
            Ok(())
        }
    }
}

pub type ContractResult<T> = Result<T, ContractError>;
