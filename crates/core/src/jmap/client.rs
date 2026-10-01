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

//! JMAP transport client (RFC 8620).
//!
//! A thin, focused client over the workspace `reqwest` (0.13): it authenticates
//! (Basic / Bearer / OAuth2 bearer), fetches the Session resource, asserts the
//! mail capability, executes method-call batches clamped to the server limits,
//! and downloads/uploads blobs. It carries no account/sync knowledge — that is
//! layered on top in a later issue.

use std::time::Duration;

use base64::Engine as _;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};

use crate::error::code::ErrorCode;
use crate::error::MailboxdResult;
use crate::jmap::error::{map_transport_error, JmapMethodError};
use crate::jmap::protocol::{Invocation, Request, Response};
use crate::jmap::session::{Session, CAP_CORE, CAP_MAIL};
use crate::raise_error;
use crate::settings::proxy::Proxy;
use crate::utils::net::parse_proxy_url;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const JSON_CONTENT_TYPE: &str = "application/json";

/// How the JMAP client authenticates to the server (RFC 8620 §8).
///
/// This is the transport-level credential, resolved from the account model's
/// `JmapAuthConfig` (decrypted) by the caller. OAuth2 is a bearer token obtained
/// from the existing OAuth2 subsystem, so it collapses to `Bearer` on the wire.
#[derive(Clone)]
pub enum JmapAuth {
    /// HTTP Basic (`Authorization: Basic base64(user:pass)`).
    Basic { username: String, password: String },
    /// Bearer / API token (also used for OAuth2 access tokens).
    Bearer { token: String },
}

impl JmapAuth {
    fn header_value(&self) -> MailboxdResult<HeaderValue> {
        let raw = match self {
            JmapAuth::Basic { username, password } => {
                let encoded = base64::engine::general_purpose::STANDARD
                    .encode(format!("{username}:{password}"));
                format!("Basic {encoded}")
            }
            JmapAuth::Bearer { token } => format!("Bearer {token}"),
        };
        HeaderValue::from_str(&raw).map_err(|e| {
            raise_error!(
                format!("invalid Authorization header: {e}"),
                ErrorCode::InvalidParameter
            )
        })
    }
}

/// A connected JMAP client. Construct via [`JmapClient::connect`], which fetches
/// and validates the Session resource.
#[derive(Debug)]
pub struct JmapClient {
    http: reqwest::Client,
    session: Session,
}

impl JmapClient {
    /// Build the underlying reqwest client, honouring proxy and `use_dangerous`
    /// (self-signed certs) the same way the IMAP path does (NFA-3).
    fn build_http_client(
        auth: &JmapAuth,
        use_proxy: Option<u64>,
        dangerous: bool,
    ) -> MailboxdResult<reqwest::Client> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(AUTHORIZATION, auth.header_value()?);
        default_headers.insert(
            ACCEPT,
            HeaderValue::from_static(JSON_CONTENT_TYPE),
        );

        let mut builder = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .default_headers(default_headers);

        if dangerous {
            builder = builder
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true);
        }

        if let Some(proxy_id) = use_proxy {
            let proxy = Proxy::get(proxy_id)?;
            let proxy_url = parse_proxy_url(&proxy.url)?.standard_url();
            let p = reqwest::Proxy::all(&proxy_url).map_err(|e| {
                raise_error!(
                    format!("failed to configure JMAP proxy: {e}"),
                    ErrorCode::InvalidParameter
                )
            })?;
            builder = builder.proxy(p);
        }

        builder.build().map_err(|e| {
            raise_error!(
                format!("failed to build JMAP HTTP client: {e}"),
                ErrorCode::InternalError
            )
        })
    }

    /// Connect to a JMAP server: fetch the Session resource from `session_url`,
    /// assert the mail capability, and retain it for subsequent calls (FA-4).
    pub async fn connect(
        session_url: &str,
        auth: JmapAuth,
        use_proxy: Option<u64>,
        dangerous: bool,
    ) -> MailboxdResult<Self> {
        let http = Self::build_http_client(&auth, use_proxy, dangerous)?;
        let session = Self::fetch_session(&http, session_url).await?;

        if !session.supports_mail() {
            return Err(raise_error!(
                format!(
                    "JMAP server at '{session_url}' does not advertise the mail capability ('{CAP_MAIL}')"
                ),
                ErrorCode::JmapCapabilityUnsupported
            ));
        }

        Ok(Self { http, session })
    }

    async fn fetch_session(http: &reqwest::Client, session_url: &str) -> MailboxdResult<Session> {
        let resp = http
            .get(session_url)
            .send()
            .await
            .map_err(|e| map_transport_error("JMAP session fetch", &e))?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(raise_error!(
                format!("JMAP authentication failed (HTTP {status})"),
                ErrorCode::JmapAuthenticationFailed
            ));
        }
        if !status.is_success() {
            return Err(raise_error!(
                format!("JMAP session fetch failed with HTTP {status}"),
                ErrorCode::HttpResponseError
            ));
        }

        resp.json::<Session>().await.map_err(|e| {
            raise_error!(
                format!("failed to parse JMAP session resource: {e}"),
                ErrorCode::JmapUnexpectedResult
            )
        })
    }

    /// The validated Session resource.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// The primary mail account id (or first mail-capable account).
    pub fn primary_mail_account(&self) -> MailboxdResult<String> {
        self.session.primary_mail_account().ok_or_else(|| {
            raise_error!(
                "JMAP server exposes no mail-capable account".into(),
                ErrorCode::JmapCapabilityUnsupported
            )
        })
    }

    /// Execute a method-call batch against the Session `apiUrl`.
    ///
    /// `using` defaults to core+mail when empty. The number of calls is clamped
    /// to `maxCallsInRequest` (NFA-2); callers that build larger batches should
    /// chunk via [`max_calls_in_request`](Self::max_calls_in_request).
    pub async fn request(
        &self,
        using: Vec<String>,
        calls: Vec<Invocation>,
    ) -> MailboxdResult<Response> {
        let using = if using.is_empty() {
            vec![CAP_CORE.to_string(), CAP_MAIL.to_string()]
        } else {
            using
        };

        let max_calls = self.session.core().max_calls_in_request() as usize;
        if calls.len() > max_calls {
            return Err(raise_error!(
                format!(
                    "JMAP request has {} method calls but server allows at most {max_calls}",
                    calls.len()
                ),
                ErrorCode::InvalidParameter
            ));
        }

        let request = Request::new(using, calls);
        let resp = self
            .http
            .post(&self.session.api_url)
            .header(CONTENT_TYPE, JSON_CONTENT_TYPE)
            .json(&request)
            .send()
            .await
            .map_err(|e| map_transport_error("JMAP API request", &e))?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(raise_error!(
                format!("JMAP authentication failed (HTTP {status})"),
                ErrorCode::JmapAuthenticationFailed
            ));
        }
        if !status.is_success() {
            return Err(raise_error!(
                format!("JMAP API request failed with HTTP {status}"),
                ErrorCode::HttpResponseError
            ));
        }

        resp.json::<Response>().await.map_err(|e| {
            raise_error!(
                format!("failed to parse JMAP response: {e}"),
                ErrorCode::JmapUnexpectedResult
            )
        })
    }

    /// Execute a single method call and return its (typed-error-checked) result.
    pub async fn call_one(
        &self,
        using: Vec<String>,
        invocation: Invocation,
    ) -> MailboxdResult<serde_json::Value> {
        let call_id = invocation.call_id.clone();
        let response = self.request(using, vec![invocation]).await?;
        match response.result_for(&call_id) {
            Ok(v) => Ok(v.clone()),
            Err(method_err) => Err(method_err.into_mailboxd_error("JMAP method call")),
        }
    }

    /// Download a blob by id via the Session `downloadUrl` template (RFC 8620 §6.2).
    pub async fn download_blob(
        &self,
        account_id: &str,
        blob_id: &str,
        content_type: &str,
        name: &str,
    ) -> MailboxdResult<Vec<u8>> {
        let url = self
            .session
            .build_download_url(account_id, blob_id, content_type, name);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| map_transport_error("JMAP blob download", &e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(raise_error!(
                format!("JMAP blob download failed with HTTP {status}"),
                ErrorCode::HttpResponseError
            ));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| map_transport_error("JMAP blob download body", &e))?;
        Ok(bytes.to_vec())
    }

    /// Upload a blob via the Session `uploadUrl` template (RFC 8620 §6.1).
    /// Returns the raw upload response JSON (containing `blobId`, `size`, `type`).
    pub async fn upload_blob(
        &self,
        account_id: &str,
        content_type: &str,
        data: Vec<u8>,
    ) -> MailboxdResult<serde_json::Value> {
        let url = self.session.build_upload_url(account_id);
        let resp = self
            .http
            .post(&url)
            .header(CONTENT_TYPE, content_type)
            .body(data)
            .send()
            .await
            .map_err(|e| map_transport_error("JMAP blob upload", &e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(raise_error!(
                format!("JMAP blob upload failed with HTTP {status}"),
                ErrorCode::HttpResponseError
            ));
        }

        resp.json::<serde_json::Value>().await.map_err(|e| {
            raise_error!(
                format!("failed to parse JMAP upload response: {e}"),
                ErrorCode::JmapUnexpectedResult
            )
        })
    }

    /// Server limit: max method calls per request (NFA-2).
    pub fn max_calls_in_request(&self) -> u64 {
        self.session.core().max_calls_in_request()
    }

    /// Server limit: max objects per `/get` (NFA-2).
    pub fn max_objects_in_get(&self) -> u64 {
        self.session.core().max_objects_in_get()
    }

    /// Re-export the method-error classifier so callers can decide on resync.
    pub fn is_cannot_calculate_changes(err: &JmapMethodError) -> bool {
        err.requires_full_resync()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_auth_header_is_base64() {
        let auth = JmapAuth::Basic {
            username: "user".into(),
            password: "pass".into(),
        };
        let hv = auth.header_value().unwrap();
        // base64("user:pass") == "dXNlcjpwYXNz"
        assert_eq!(hv.to_str().unwrap(), "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn bearer_auth_header() {
        let auth = JmapAuth::Bearer {
            token: "abc123".into(),
        };
        assert_eq!(
            auth.header_value().unwrap().to_str().unwrap(),
            "Bearer abc123"
        );
    }
}
