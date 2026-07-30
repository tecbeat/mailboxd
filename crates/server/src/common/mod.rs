//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::fmt;

use mailboxd_core::error::code::ErrorCode;
use mailboxd_core::error::MailboxdError;
use poem::error::ResponseError;
use poem::Body;
use poem::{http::StatusCode, Error, Response};
use tracing::error;

use crate::error::code::IntoStatusCode;

pub mod auth;
pub mod error;
pub mod log;
pub mod status;
pub mod timeout;
pub mod tls;
pub mod validator;

#[derive(Debug)]
pub struct MailboxdServerError(pub MailboxdError);

impl fmt::Display for MailboxdServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Internal Server Error: {:?}", self.0)
    }
}

impl std::error::Error for MailboxdServerError {}

impl From<MailboxdError> for MailboxdServerError {
    fn from(err: MailboxdError) -> Self {
        MailboxdServerError(err)
    }
}

#[inline]
fn create_rust_mailer_error(message: &str, code: ErrorCode) -> MailboxdServerError {
    MailboxdError::Generic {
        message: message.into(),
        location: snafu::location!(),
        code,
    }
    .into()
}

#[inline]
pub fn create_api_error_response(message: &str, code: ErrorCode) -> Error {
    let rust_mailer_error = create_rust_mailer_error(message, code);
    rust_mailer_error.into()
}

impl ResponseError for MailboxdServerError {
    fn status(&self) -> StatusCode {
        match self.0 {
            MailboxdError::Generic {
                message: _,
                location: _,
                code,
            } => code.status(),
        }
    }

    fn as_response(&self) -> Response
    where
        Self: std::error::Error + Send + Sync + 'static,
    {
        match &self.0 {
            MailboxdError::Generic {
                message,
                location,
                code,
            } => {
                error!(
                    error_code = code.to_u32(),
                    error_message = %message,
                    error_location = ?location
                );

                let body = Body::from_json(serde_json::json!({
                    "code": code.to_u32(),
                    "message": message.to_string(),
                }))
                .unwrap();

                Response::builder().status(self.status()).body(body)
            }
        }
    }
}
