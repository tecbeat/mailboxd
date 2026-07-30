use snafu::{Location, Snafu};

use crate::error::code::ErrorCode;

pub mod code;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum MailboxdError {
    #[snafu(display("{message}"))]
    Generic {
        message: String,
        #[snafu(implicit)]
        location: Location,
        code: ErrorCode,
    },
}

impl MailboxdError {
    pub fn code(&self) -> ErrorCode {
        match self {
            MailboxdError::Generic { code, .. } => *code,
        }
    }
}

pub type MailboxdResult<T, E = MailboxdError> = std::result::Result<T, E>;
