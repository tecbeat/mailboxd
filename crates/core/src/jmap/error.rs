//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, an email archiving project.
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

//! JMAP error handling (RFC 8620 §3.6.1, RFC 8621).
//!
//! Maps JMAP method-level errors and transport/HTTP failures onto the shared
//! [`MailboxdError`](crate::error::MailboxdError) so the sync layer can react
//! uniformly. The most operationally important one is
//! [`cannotCalculateChanges`](JmapMethodError::CannotCalculateChanges): when a
//! `*/changes` call returns it, the sync engine must fall back to a full resync
//! (FA-11).

use serde::{Deserialize, Serialize};

use crate::error::code::ErrorCode;
use crate::error::MailboxdError;
use crate::raise_error;

/// A JMAP method-level error object, returned in place of a method response
/// (RFC 8620 §3.6.1). The `type` field is the machine-readable error code.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct JmapMethodErrorObject {
    #[serde(rename = "type")]
    pub error_type: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Typed view over the JMAP method-level error `type` values this client
/// distinguishes. Everything not explicitly modelled falls into
/// [`Other`](JmapMethodError::Other) carrying the raw type string.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JmapMethodError {
    /// The server cannot calculate the changes from the supplied state
    /// (RFC 8620 §5.2). Callers must perform a full resync.
    CannotCalculateChanges,
    /// The `sinceState` / supplied state string is no longer valid.
    InvalidArguments,
    /// The account does not support this data type / method.
    AccountNotSupportedByMethod,
    /// Requested object(s) not found.
    RequestTooLarge,
    /// Any other server-defined error type.
    Other(String),
}

impl JmapMethodError {
    pub fn from_type(error_type: &str) -> Self {
        match error_type {
            "cannotCalculateChanges" => Self::CannotCalculateChanges,
            "invalidArguments" => Self::InvalidArguments,
            "accountNotSupportedByMethod" => Self::AccountNotSupportedByMethod,
            "requestTooLarge" => Self::RequestTooLarge,
            other => Self::Other(other.to_string()),
        }
    }

    /// The `ErrorCode` this method error maps onto.
    pub fn error_code(&self) -> ErrorCode {
        match self {
            Self::CannotCalculateChanges => ErrorCode::JmapCannotCalculateChanges,
            _ => ErrorCode::JmapMethodError,
        }
    }

    /// Convenience: `true` when the sync engine must perform a full resync.
    pub fn requires_full_resync(&self) -> bool {
        matches!(self, Self::CannotCalculateChanges)
    }

    /// Convert this method error into a `MailboxdError`.
    pub fn into_mailboxd_error(self, context: &str) -> MailboxdError {
        let code = self.error_code();
        let detail = match &self {
            Self::CannotCalculateChanges => "cannotCalculateChanges".to_string(),
            Self::InvalidArguments => "invalidArguments".to_string(),
            Self::AccountNotSupportedByMethod => "accountNotSupportedByMethod".to_string(),
            Self::RequestTooLarge => "requestTooLarge".to_string(),
            Self::Other(t) => t.clone(),
        };
        raise_error!(format!("{context}: JMAP method error '{detail}'"), code)
    }
}

/// Map a `reqwest` transport error onto a `MailboxdError`, classifying timeouts
/// and connection failures as network errors (NFA-3 proxy/TLS surfaces here too).
pub fn map_transport_error(context: &str, err: &reqwest::Error) -> MailboxdError {
    let code = if err.is_timeout() {
        ErrorCode::ConnectionTimeout
    } else if err.is_connect() {
        ErrorCode::NetworkError
    } else if err.is_status() {
        ErrorCode::HttpResponseError
    } else {
        ErrorCode::JmapRequestFailed
    };
    raise_error!(format!("{context}: {err}"), code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_cannot_calculate_changes() {
        let e = JmapMethodError::from_type("cannotCalculateChanges");
        assert_eq!(e, JmapMethodError::CannotCalculateChanges);
        assert!(e.requires_full_resync());
        assert_eq!(e.error_code(), ErrorCode::JmapCannotCalculateChanges);
    }

    #[test]
    fn maps_unknown_error_type_to_other() {
        let e = JmapMethodError::from_type("somethingNew");
        assert_eq!(e, JmapMethodError::Other("somethingNew".into()));
        assert!(!e.requires_full_resync());
        assert_eq!(e.error_code(), ErrorCode::JmapMethodError);
    }

    #[test]
    fn method_error_object_deserializes() {
        let json = r#"{"type":"cannotCalculateChanges","description":"state too old"}"#;
        let obj: JmapMethodErrorObject = serde_json::from_str(json).unwrap();
        assert_eq!(obj.error_type, "cannotCalculateChanges");
        assert_eq!(obj.description.as_deref(), Some("state too old"));
    }
}
