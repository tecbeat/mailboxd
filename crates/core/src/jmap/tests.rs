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

//! Integration tests for the JMAP client against the in-crate mock server.
//!
//! The mock server substitutes `{BASE}` in route bodies with its own base URL,
//! so a Session resource can self-reference its apiUrl/download/upload endpoints
//! without the test knowing the random port in advance.

use crate::error::code::ErrorCode;
use crate::jmap::client::{JmapAuth, JmapClient};
use crate::jmap::mock_server::MockJmapServer;

/// Session resource JSON with `{BASE}` placeholders resolved by the mock server.
fn session_json(with_mail: bool) -> String {
    let caps = if with_mail {
        r#""urn:ietf:params:jmap:core": { "maxCallsInRequest": 16, "maxObjectsInGet": 500 },
           "urn:ietf:params:jmap:mail": {}"#
    } else {
        r#""urn:ietf:params:jmap:core": { "maxCallsInRequest": 16, "maxObjectsInGet": 500 }"#
    };
    format!(
        r#"{{
            "capabilities": {{ {caps} }},
            "accounts": {{
                "A1": {{
                    "name": "user@example.com",
                    "isPersonal": true,
                    "isReadOnly": false,
                    "accountCapabilities": {{ "urn:ietf:params:jmap:mail": {{}} }}
                }}
            }},
            "primaryAccounts": {{ "urn:ietf:params:jmap:mail": "A1" }},
            "username": "user@example.com",
            "apiUrl": "{{BASE}}/jmap/api",
            "downloadUrl": "{{BASE}}/jmap/dl/{{{{accountId}}}}/{{{{blobId}}}}/{{{{name}}}}?type={{{{type}}}}",
            "uploadUrl": "{{BASE}}/jmap/upload/{{{{accountId}}}}/",
            "state": "s0"
        }}"#
    )
}

#[tokio::test]
async fn connects_and_exposes_limits_and_primary_account() {
    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .start()
        .await;

    let client = JmapClient::connect(
        &handle.session_url(),
        JmapAuth::Bearer { token: "t".into() },
        None,
        false,
    )
    .await
    .expect("connect");

    assert_eq!(client.max_calls_in_request(), 16);
    assert_eq!(client.max_objects_in_get(), 500);
    assert_eq!(client.primary_mail_account().unwrap(), "A1");

    // The Authorization header must have been sent on the session fetch.
    let reqs = handle.captured().await;
    assert!(reqs
        .iter()
        .any(|r| r.authorization.as_deref() == Some("Bearer t")));
}

#[tokio::test]
async fn basic_auth_header_is_sent() {
    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .start()
        .await;

    let _ = JmapClient::connect(
        &handle.session_url(),
        JmapAuth::Basic {
            username: "user".into(),
            password: "pass".into(),
        },
        None,
        false,
    )
    .await
    .expect("connect");

    let reqs = handle.captured().await;
    // base64("user:pass") == "dXNlcjpwYXNz"
    assert!(reqs
        .iter()
        .any(|r| r.authorization.as_deref() == Some("Basic dXNlcjpwYXNz")));
}

#[tokio::test]
async fn rejects_server_without_mail_capability() {
    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(false))
        .start()
        .await;

    let err = JmapClient::connect(
        &handle.session_url(),
        JmapAuth::Bearer { token: "t".into() },
        None,
        false,
    )
    .await
    .expect_err("must reject missing mail capability");
    assert_eq!(err.code(), ErrorCode::JmapCapabilityUnsupported);
}

#[tokio::test]
async fn unauthorized_session_maps_to_auth_error() {
    let handle = MockJmapServer::new()
        .route_status("GET", "/jmap/session", 401, r#"{"error":"nope"}"#)
        .start()
        .await;

    let err = JmapClient::connect(
        &handle.session_url(),
        JmapAuth::Basic {
            username: "u".into(),
            password: "p".into(),
        },
        None,
        false,
    )
    .await
    .expect_err("401 must be an auth error");
    assert_eq!(err.code(), ErrorCode::JmapAuthenticationFailed);
}

#[tokio::test]
async fn mailbox_get_parses_list() {
    let mailbox_resp = r#"{
        "methodResponses": [
            ["Mailbox/get", {
                "accountId": "A1",
                "state": "m1",
                "list": [
                    {"id": "mb1", "name": "Inbox", "role": "inbox", "totalEmails": 5},
                    {"id": "mb2", "name": "Sent", "role": "sent"}
                ],
                "notFound": []
            }, "0"]
        ],
        "sessionState": "s0"
    }"#;

    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .route("POST", "/jmap/api", mailbox_resp)
        .start()
        .await;

    let client = JmapClient::connect(
        &handle.session_url(),
        JmapAuth::Bearer { token: "t".into() },
        None,
        false,
    )
    .await
    .expect("connect");

    let resp = client.mailbox_get("A1", None).await.expect("mailbox_get");
    assert_eq!(resp.list.len(), 2);
    assert_eq!(resp.list[0].role.as_deref(), Some("inbox"));
    assert_eq!(resp.list[0].total_emails, 5);
}

#[tokio::test]
async fn email_query_get_chained_returns_emails() {
    // Single request carrying both Email/query and Email/get responses.
    let resp = r#"{
        "methodResponses": [
            ["Email/query", {"accountId": "A1", "queryState": "q1", "position": 0, "ids": ["e1"]}, "q"],
            ["Email/get", {
                "accountId": "A1",
                "state": "e1",
                "list": [
                    {"id": "e1", "blobId": "b1", "mailboxIds": {"mb1": true, "mb2": true}, "size": 1024, "receivedAt": "2026-01-01T00:00:00Z"}
                ],
                "notFound": []
            }, "g"]
        ],
        "sessionState": "s0"
    }"#;

    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .route("POST", "/jmap/api", resp)
        .start()
        .await;

    let client = JmapClient::connect(
        &handle.session_url(),
        JmapAuth::Bearer { token: "t".into() },
        None,
        false,
    )
    .await
    .expect("connect");

    let got = client
        .email_query_get("A1", None, 50, None)
        .await
        .expect("email_query_get");
    assert_eq!(got.list.len(), 1);
    assert_eq!(got.list[0].blob_id, "b1");
    // one email present in two mailboxes (FA-12 shape)
    assert_eq!(got.list[0].mailbox_ids.len(), 2);
}

#[tokio::test]
async fn email_changes_cannot_calculate_maps_to_typed_error() {
    let error_resp = r#"{
        "methodResponses": [
            ["error", {"type": "cannotCalculateChanges"}, "0"]
        ],
        "sessionState": "s0"
    }"#;

    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .route("POST", "/jmap/api", error_resp)
        .start()
        .await;

    let client = JmapClient::connect(
        &handle.session_url(),
        JmapAuth::Bearer { token: "t".into() },
        None,
        false,
    )
    .await
    .expect("connect");

    let err = client
        .email_changes("A1", "oldstate", None)
        .await
        .expect_err("must surface cannotCalculateChanges");
    assert_eq!(err.code(), ErrorCode::JmapCannotCalculateChanges);
}

#[tokio::test]
async fn request_rejects_batch_exceeding_server_limit() {
    // Server advertises maxCallsInRequest: 16 (via session_json). Build 17 calls.
    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .start()
        .await;

    let client = JmapClient::connect(
        &handle.session_url(),
        JmapAuth::Bearer { token: "t".into() },
        None,
        false,
    )
    .await
    .expect("connect");

    let calls: Vec<_> = (0..17)
        .map(|i| {
            crate::jmap::protocol::Invocation::new(
                "Core/echo",
                serde_json::json!({}),
                i.to_string(),
            )
        })
        .collect();
    let err = client
        .request(vec![], calls)
        .await
        .expect_err("over-limit batch must be rejected");
    assert_eq!(err.code(), ErrorCode::InvalidParameter);
}

#[tokio::test]
async fn blob_download_returns_bytes() {
    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .route("GET", "/jmap/dl/", "RAW-EML-BYTES")
        .start()
        .await;

    let client = JmapClient::connect(
        &handle.session_url(),
        JmapAuth::Bearer { token: "t".into() },
        None,
        false,
    )
    .await
    .expect("connect");

    let bytes = client
        .download_blob("A1", "b1", "message/rfc822", "msg.eml")
        .await
        .expect("download");
    assert_eq!(bytes, b"RAW-EML-BYTES");
}

#[tokio::test]
async fn blob_upload_returns_blob_metadata() {
    let upload_resp = r#"{"accountId":"A1","blobId":"newblob","type":"message/rfc822","size":12}"#;
    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .route("POST", "/jmap/upload/", upload_resp)
        .start()
        .await;

    let client = JmapClient::connect(
        &handle.session_url(),
        JmapAuth::Bearer { token: "t".into() },
        None,
        false,
    )
    .await
    .expect("connect");

    let meta = client
        .upload_blob("A1", "message/rfc822", b"hello world!".to_vec())
        .await
        .expect("upload");
    assert_eq!(meta["blobId"], "newblob");
}
