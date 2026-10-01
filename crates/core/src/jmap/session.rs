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

//! JMAP Session resource (RFC 8620 §2).
//!
//! The Session object is the entry point: it advertises the server
//! capabilities, the URLs for API/download/upload/eventsource, the accounts the
//! user can access, and — importantly for batching (NFA-2) — the server limits
//! under the core capability.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Well-known JMAP capability URIs.
pub const CAP_CORE: &str = "urn:ietf:params:jmap:core";
pub const CAP_MAIL: &str = "urn:ietf:params:jmap:mail";

/// Server limits advertised under `urn:ietf:params:jmap:core` (RFC 8620 §2).
///
/// All fields are optional in the wire format; the `max_*` helpers provide the
/// spec defaults / safe fallbacks used for request clamping.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CoreCapability {
    #[serde(rename = "maxSizeUpload", default)]
    pub max_size_upload: Option<u64>,
    #[serde(rename = "maxConcurrentUpload", default)]
    pub max_concurrent_upload: Option<u64>,
    #[serde(rename = "maxSizeRequest", default)]
    pub max_size_request: Option<u64>,
    #[serde(rename = "maxConcurrentRequests", default)]
    pub max_concurrent_requests: Option<u64>,
    #[serde(rename = "maxCallsInRequest", default)]
    pub max_calls_in_request: Option<u64>,
    #[serde(rename = "maxObjectsInGet", default)]
    pub max_objects_in_get: Option<u64>,
    #[serde(rename = "maxObjectsInSet", default)]
    pub max_objects_in_set: Option<u64>,
    #[serde(rename = "collationAlgorithms", default)]
    pub collation_algorithms: Vec<String>,
}

impl Default for CoreCapability {
    fn default() -> Self {
        Self {
            max_size_upload: None,
            max_concurrent_upload: None,
            max_size_request: None,
            max_concurrent_requests: None,
            max_calls_in_request: None,
            max_objects_in_get: None,
            max_objects_in_set: None,
            collation_algorithms: Vec::new(),
        }
    }
}

impl CoreCapability {
    /// Max method calls per request, defaulting conservatively when the server
    /// omits it (the spec's own example advertises higher, but we stay safe).
    pub fn max_calls_in_request(&self) -> u64 {
        self.max_calls_in_request.unwrap_or(16).max(1)
    }

    /// Max objects per `/get` call.
    pub fn max_objects_in_get(&self) -> u64 {
        self.max_objects_in_get.unwrap_or(500).max(1)
    }

    /// Max objects per `/set` call.
    pub fn max_objects_in_set(&self) -> u64 {
        self.max_objects_in_set.unwrap_or(500).max(1)
    }
}

/// An account the authenticated principal has access to (RFC 8620 §2).
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct JmapAccount {
    pub name: String,
    #[serde(rename = "isPersonal", default)]
    pub is_personal: bool,
    #[serde(rename = "isReadOnly", default)]
    pub is_read_only: bool,
    /// Per-account capability objects (opaque; we only need their presence).
    #[serde(rename = "accountCapabilities", default)]
    pub account_capabilities: HashMap<String, serde_json::Value>,
}

/// The JMAP Session resource (RFC 8620 §2).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Session {
    /// Set of capability URIs → capability object. We parse the core one
    /// strongly and keep the rest as raw JSON.
    #[serde(default)]
    pub capabilities: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub accounts: HashMap<String, JmapAccount>,
    /// Map of capability URI → account id that is primary for it.
    #[serde(rename = "primaryAccounts", default)]
    pub primary_accounts: HashMap<String, String>,
    pub username: String,
    #[serde(rename = "apiUrl")]
    pub api_url: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
    #[serde(rename = "uploadUrl")]
    pub upload_url: String,
    #[serde(rename = "eventSourceUrl", default)]
    pub event_source_url: Option<String>,
    /// Opaque state string; changes when the Session object changes.
    #[serde(default)]
    pub state: String,
}

impl Session {
    /// `true` if the server advertises the given capability URI.
    pub fn has_capability(&self, uri: &str) -> bool {
        self.capabilities.contains_key(uri)
    }

    /// `true` if the server advertises the JMAP mail capability (RFC 8621).
    pub fn supports_mail(&self) -> bool {
        self.has_capability(CAP_MAIL)
    }

    /// Parsed core capability limits, or defaults if absent/malformed.
    pub fn core(&self) -> CoreCapability {
        self.capabilities
            .get(CAP_CORE)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    /// The primary account id for mail, if the server names one; otherwise the
    /// first account id that advertises the mail capability.
    pub fn primary_mail_account(&self) -> Option<String> {
        if let Some(id) = self.primary_accounts.get(CAP_MAIL) {
            return Some(id.clone());
        }
        self.accounts
            .iter()
            .find(|(_, a)| a.account_capabilities.contains_key(CAP_MAIL))
            .map(|(id, _)| id.clone())
    }

    /// Resolve a blob download URL from the `downloadUrl` URI template
    /// (RFC 8620 §6.2). Substitutes `{accountId}`, `{blobId}`, `{type}`,
    /// `{name}` placeholders.
    pub fn build_download_url(
        &self,
        account_id: &str,
        blob_id: &str,
        content_type: &str,
        name: &str,
    ) -> String {
        self.download_url
            .replace("{accountId}", &url_escape(account_id))
            .replace("{blobId}", &url_escape(blob_id))
            .replace("{type}", &url_escape(content_type))
            .replace("{name}", &url_escape(name))
    }

    /// Resolve the upload URL from the `uploadUrl` template (RFC 8620 §6.1).
    pub fn build_upload_url(&self, account_id: &str) -> String {
        self.upload_url.replace("{accountId}", &url_escape(account_id))
    }
}

/// Minimal percent-encoding for path/query template substitution. Encodes the
/// characters that would otherwise break a URL; sufficient for JMAP ids which
/// are already restricted to a safe alphabet (RFC 8620 §1.2) but defensive for
/// names.
fn url_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "capabilities": {
            "urn:ietf:params:jmap:core": {
                "maxSizeUpload": 50000000,
                "maxCallsInRequest": 32,
                "maxObjectsInGet": 1000,
                "maxObjectsInSet": 1000,
                "collationAlgorithms": ["i;ascii-numeric"]
            },
            "urn:ietf:params:jmap:mail": {}
        },
        "accounts": {
            "A13824": {
                "name": "user@example.com",
                "isPersonal": true,
                "isReadOnly": false,
                "accountCapabilities": { "urn:ietf:params:jmap:mail": {} }
            }
        },
        "primaryAccounts": { "urn:ietf:params:jmap:mail": "A13824" },
        "username": "user@example.com",
        "apiUrl": "https://jmap.example.com/api/",
        "downloadUrl": "https://jmap.example.com/dl/{accountId}/{blobId}/{name}?type={type}",
        "uploadUrl": "https://jmap.example.com/upload/{accountId}/",
        "state": "cyrus-0"
    }"#;

    #[test]
    fn parses_session_and_limits() {
        let s: Session = serde_json::from_str(SAMPLE).unwrap();
        assert!(s.supports_mail());
        assert!(s.has_capability(CAP_CORE));
        let core = s.core();
        assert_eq!(core.max_calls_in_request(), 32);
        assert_eq!(core.max_objects_in_get(), 1000);
        assert_eq!(core.max_objects_in_set(), 1000);
        assert_eq!(s.primary_mail_account().as_deref(), Some("A13824"));
        assert_eq!(s.username, "user@example.com");
    }

    #[test]
    fn missing_core_uses_defaults() {
        let s = Session::default();
        let core = s.core();
        assert_eq!(core.max_calls_in_request(), 16);
        assert_eq!(core.max_objects_in_get(), 500);
        assert!(!s.supports_mail());
    }

    #[test]
    fn builds_download_url_from_template() {
        let s: Session = serde_json::from_str(SAMPLE).unwrap();
        let url = s.build_download_url("A13824", "G123", "application/octet-stream", "file.eml");
        assert_eq!(
            url,
            "https://jmap.example.com/dl/A13824/G123/file.eml?type=application%2Foctet-stream"
        );
    }

    #[test]
    fn builds_upload_url_from_template() {
        let s: Session = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(
            s.build_upload_url("A13824"),
            "https://jmap.example.com/upload/A13824/"
        );
    }

    #[test]
    fn falls_back_to_account_capability_for_primary() {
        let mut s: Session = serde_json::from_str(SAMPLE).unwrap();
        s.primary_accounts.clear();
        assert_eq!(s.primary_mail_account().as_deref(), Some("A13824"));
    }
}
