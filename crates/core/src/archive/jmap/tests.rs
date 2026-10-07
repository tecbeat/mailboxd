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

//! Tests for the JMAP [`MailSource`] adapter: pure mapping helpers plus
//! end-to-end session behaviour against the in-crate mock JMAP server.

use tokio_util::sync::CancellationToken;

use crate::archive::imap::mailbox::AttributeEnum;
use crate::archive::source::{MailSession, MessageRef, SyncCursor};
use crate::error::code::ErrorCode;
use crate::jmap::client::{JmapAuth, JmapClient};
use crate::jmap::mail::JmapMailbox;
use crate::jmap::mock_server::MockJmapServer;

use super::source::{
    parse_rfc3339_millis, resolve_hierarchical_name, role_to_attribute, to_mailbox,
    JmapMailSession,
};

// ── Pure mapping helpers ──────────────────────────────────────────────

#[test]
fn role_maps_to_special_use_attribute() {
    assert_eq!(
        role_to_attribute(Some("sent")).map(|a| a.attr),
        Some(AttributeEnum::Sent)
    );
    assert_eq!(
        role_to_attribute(Some("trash")).map(|a| a.attr),
        Some(AttributeEnum::Trash)
    );
    assert_eq!(
        role_to_attribute(Some("archive")).map(|a| a.attr),
        Some(AttributeEnum::Archive)
    );
    // Unknown / no role → no attribute.
    assert!(role_to_attribute(Some("custom")).is_none());
    assert!(role_to_attribute(None).is_none());
}

fn mk(id: &str, name: &str, parent: Option<&str>) -> JmapMailbox {
    JmapMailbox {
        id: id.to_string(),
        name: name.to_string(),
        parent_id: parent.map(|s| s.to_string()),
        ..Default::default()
    }
}

#[test]
fn hierarchical_name_joins_parents() {
    let mut by_id = std::collections::HashMap::new();
    by_id.insert("1".to_string(), mk("1", "INBOX", None));
    by_id.insert("2".to_string(), mk("2", "Invoices", Some("1")));
    by_id.insert("3".to_string(), mk("3", "2026", Some("2")));

    let leaf = mk("3", "2026", Some("2"));
    assert_eq!(resolve_hierarchical_name(&leaf, &by_id), "INBOX/Invoices/2026");
}

#[test]
fn hierarchical_name_handles_broken_parent_chain() {
    let by_id = std::collections::HashMap::new(); // parent not present
    let orphan = mk("9", "Orphan", Some("missing"));
    assert_eq!(resolve_hierarchical_name(&orphan, &by_id), "Orphan");
}

#[test]
fn to_mailbox_sets_stable_id_and_counts() {
    let mut by_id = std::collections::HashMap::new();
    by_id.insert("mb1".to_string(), {
        let mut m = mk("mb1", "Inbox", None);
        m.role = Some("inbox".to_string());
        m.total_emails = 7;
        m.unread_emails = 3;
        m
    });
    let jm = by_id.get("mb1").unwrap().clone();
    let a = to_mailbox(100, &jm, &by_id);
    let b = to_mailbox(100, &jm, &by_id);
    assert_eq!(a.id, b.id, "id must be stable for the same account+jmap id");
    assert_eq!(a.account_id, 100);
    assert_eq!(a.name, "Inbox");
    assert_eq!(a.exists, 7);
    assert_eq!(a.unseen, Some(3));
    assert!(a.sync_cursor.is_none());
}

#[test]
fn parse_rfc3339_millis_roundtrip() {
    // 2026-01-01T00:00:00Z == 1767225600000 ms.
    assert_eq!(
        parse_rfc3339_millis("2026-01-01T00:00:00Z"),
        Some(1767225600000)
    );
    assert_eq!(parse_rfc3339_millis("not-a-date"), None);
}

// ── End-to-end session against the mock JMAP server ───────────────────

fn session_json(with_mail: bool) -> String {
    let caps = if with_mail {
        r#""urn:ietf:params:jmap:core": { "maxObjectsInGet": 500 },
           "urn:ietf:params:jmap:mail": {}"#
    } else {
        r#""urn:ietf:params:jmap:core": {}"#
    };
    format!(
        r#"{{
            "capabilities": {{ {caps} }},
            "accounts": {{ "A1": {{ "name": "u@example.com", "accountCapabilities": {{ "urn:ietf:params:jmap:mail": {{}} }} }} }},
            "primaryAccounts": {{ "urn:ietf:params:jmap:mail": "A1" }},
            "username": "u@example.com",
            "apiUrl": "{{BASE}}/jmap/api",
            "downloadUrl": "{{BASE}}/jmap/dl/{{{{accountId}}}}/{{{{blobId}}}}/{{{{name}}}}?type={{{{type}}}}",
            "uploadUrl": "{{BASE}}/jmap/upload/{{{{accountId}}}}/",
            "state": "s0"
        }}"#
    )
}

/// Build a connected `JmapMailSession` against a mock server, after priming its
/// internal mailbox-id map via `list_mailboxes`.
async fn connect_session(handle_session_url: &str) -> JmapMailSession {
    let client = JmapClient::connect(
        handle_session_url,
        JmapAuth::Bearer { token: "t".into() },
        None,
        false,
    )
    .await
    .expect("connect");
    let jmap_account_id = client.primary_mail_account().unwrap();
    JmapMailSession::new_for_test(7, jmap_account_id, client)
}

#[tokio::test]
async fn list_mailboxes_maps_role_and_hierarchy() {
    let mailbox_resp = r#"{
        "methodResponses": [
            ["Mailbox/get", {
                "accountId": "A1",
                "state": "m1",
                "list": [
                    {"id": "mb1", "name": "Inbox", "role": "inbox", "totalEmails": 4},
                    {"id": "mb2", "name": "Archive", "role": "archive", "parentId": "mb1"}
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

    let mut session = connect_session(&handle.session_url()).await;
    let boxes = session.list_mailboxes().await.expect("list");
    assert_eq!(boxes.len(), 2);
    let archive = boxes.iter().find(|m| m.name.ends_with("Archive")).unwrap();
    // hierarchy resolved via parentId
    assert_eq!(archive.name, "Inbox/Archive");
    assert_eq!(archive.attributes[0].attr, AttributeEnum::Archive);
}

#[tokio::test]
async fn changes_since_initial_enumerates_all_and_captures_state() {
    // Mailbox/get (for the id map), then Email/query page, then Email/changes("")
    // for the initial state snapshot.
    let mailbox_resp = r#"{"methodResponses":[["Mailbox/get",{"accountId":"A1","state":"m1","list":[{"id":"mb1","name":"Inbox","role":"inbox"}],"notFound":[]},"0"]],"sessionState":"s0"}"#;
    let query_resp = r#"{"methodResponses":[["Email/query",{"accountId":"A1","queryState":"q1","position":0,"total":2,"ids":["e1","e2"]}, "0"]],"sessionState":"s0"}"#;
    let changes_resp = r#"{"methodResponses":[["Email/changes",{"accountId":"A1","oldState":"","newState":"state-123","hasMoreChanges":false,"created":[],"updated":[],"destroyed":[]}, "0"]],"sessionState":"s0"}"#;

    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .route_body("POST", "/jmap/api", "Mailbox/get", mailbox_resp)
        .route_body("POST", "/jmap/api", "Email/query", query_resp)
        .route_body("POST", "/jmap/api", "Email/changes", changes_resp)
        .start()
        .await;

    let mut session = connect_session(&handle.session_url()).await;
    let boxes = session.list_mailboxes().await.unwrap();
    let inbox = &boxes[0];

    let changes = session
        .changes_since(inbox, None, CancellationToken::new())
        .await
        .expect("initial changes");

    assert_eq!(
        changes.messages,
        vec![MessageRef("e1".into()), MessageRef("e2".into())]
    );
    assert_eq!(changes.cursor, Some(SyncCursor::new("state-123")));
    assert!(!changes.requires_full_resync);
}

#[tokio::test]
async fn changes_since_incremental_returns_created_and_updated() {
    let mailbox_resp = r#"{"methodResponses":[["Mailbox/get",{"accountId":"A1","state":"m1","list":[{"id":"mb1","name":"Inbox","role":"inbox"}],"notFound":[]},"0"]],"sessionState":"s0"}"#;
    let changes_resp = r#"{"methodResponses":[["Email/changes",{"accountId":"A1","oldState":"old","newState":"new","hasMoreChanges":false,"created":["e3"],"updated":["e4"],"destroyed":["e0"]}, "0"]],"sessionState":"s0"}"#;

    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .route_body("POST", "/jmap/api", "Mailbox/get", mailbox_resp)
        .route_body("POST", "/jmap/api", "Email/changes", changes_resp)
        .start()
        .await;

    let mut session = connect_session(&handle.session_url()).await;
    let boxes = session.list_mailboxes().await.unwrap();
    let inbox = &boxes[0];

    let changes = session
        .changes_since(inbox, Some(SyncCursor::new("old")), CancellationToken::new())
        .await
        .expect("incremental changes");

    assert_eq!(
        changes.messages,
        vec![MessageRef("e3".into()), MessageRef("e4".into())]
    );
    assert_eq!(changes.cursor, Some(SyncCursor::new("new")));
    assert!(!changes.requires_full_resync);
}

#[tokio::test]
async fn changes_since_cannot_calculate_triggers_full_resync() {
    let mailbox_resp = r#"{"methodResponses":[["Mailbox/get",{"accountId":"A1","state":"m1","list":[{"id":"mb1","name":"Inbox","role":"inbox"}],"notFound":[]},"0"]],"sessionState":"s0"}"#;
    // Email/changes returns the method error; the adapter then falls back to a
    // full Email/query reconcile and re-snapshots the state.
    let error_resp = r#"{"methodResponses":[["error",{"type":"cannotCalculateChanges"}, "0"]],"sessionState":"s0"}"#;
    let query_resp = r#"{"methodResponses":[["Email/query",{"accountId":"A1","queryState":"q1","position":0,"total":1,"ids":["e9"]}, "0"]],"sessionState":"s0"}"#;

    // Both the failing Email/changes and the follow-up snapshot Email/changes("")
    // match the same body pattern; the mock returns the error for both. The
    // adapter treats a failed snapshot as "no cursor change", which is fine.
    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .route_body("POST", "/jmap/api", "Mailbox/get", mailbox_resp)
        .route_body("POST", "/jmap/api", "Email/query", query_resp)
        .route_body("POST", "/jmap/api", "Email/changes", error_resp)
        .start()
        .await;

    let mut session = connect_session(&handle.session_url()).await;
    let boxes = session.list_mailboxes().await.unwrap();
    let inbox = &boxes[0];

    let changes = session
        .changes_since(inbox, Some(SyncCursor::new("stale")), CancellationToken::new())
        .await
        .expect("reconcile");

    assert_eq!(changes.messages, vec![MessageRef("e9".into())]);
    assert!(changes.requires_full_resync);
}

#[tokio::test]
async fn load_raw_downloads_blob_bytes() {
    let mailbox_resp = r#"{"methodResponses":[["Mailbox/get",{"accountId":"A1","state":"m1","list":[{"id":"mb1","name":"Inbox","role":"inbox"}],"notFound":[]},"0"]],"sessionState":"s0"}"#;
    let email_get_resp = r#"{"methodResponses":[["Email/get",{"accountId":"A1","state":"e1","list":[{"id":"e1","blobId":"b1","size":12,"receivedAt":"2026-01-01T00:00:00Z"}],"notFound":[]}, "0"]],"sessionState":"s0"}"#;

    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .route_body("POST", "/jmap/api", "Mailbox/get", mailbox_resp)
        .route_body("POST", "/jmap/api", "Email/get", email_get_resp)
        .route("GET", "/jmap/dl/", "Hello world!")
        .start()
        .await;

    let mut session = connect_session(&handle.session_url()).await;
    let boxes = session.list_mailboxes().await.unwrap();
    let inbox = &boxes[0];

    let raw = session
        .load_raw(inbox, &MessageRef("e1".into()))
        .await
        .expect("load_raw");
    assert_eq!(raw.body, b"Hello world!");
    assert_eq!(raw.size, 12);
    assert_eq!(raw.internal_date, 1767225600000);
}

#[tokio::test]
async fn append_uploads_blob_and_imports_into_original_mailbox() {
    // Restore (FA-17): resolve the mailbox by name, upload the EML as a blob,
    // then Email/import it. The mock returns the uploaded blobId and a
    // successful import.
    let mailbox_resp = r#"{"methodResponses":[["Mailbox/get",{"accountId":"A1","state":"m1","list":[{"id":"mb1","name":"Inbox","role":"inbox"}],"notFound":[]},"0"]],"sessionState":"s0"}"#;
    let upload_resp = r#"{"accountId":"A1","blobId":"uploaded-b1","type":"message/rfc822","size":42}"#;
    let import_resp = r#"{"methodResponses":[["Email/import",{"accountId":"A1","created":{"restore":{"id":"new-e1","blobId":"uploaded-b1","threadId":"t1","size":42}},"notCreated":{}},"0"]],"sessionState":"s0"}"#;

    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .route_body("POST", "/jmap/api", "Mailbox/get", mailbox_resp)
        .route_body("POST", "/jmap/api", "Email/import", import_resp)
        .route("POST", "/jmap/upload/", upload_resp)
        .start()
        .await;

    let mut session = connect_session(&handle.session_url()).await;
    session
        .append("Inbox", b"From: a@example.com\r\nSubject: hi\r\n\r\nbody")
        .await
        .expect("append should succeed");

    // The uploaded blob bytes must have reached the upload endpoint.
    let captured = handle.captured().await;
    let upload = captured
        .iter()
        .find(|r| r.path.contains("/jmap/upload/"))
        .expect("an upload request was made");
    assert!(upload.body.contains("Subject: hi"));
}

#[tokio::test]
async fn append_into_missing_mailbox_fails_gracefully() {
    // The message's original mailbox no longer exists on the server: resolution
    // must fail with ResourceNotFound and no upload/import is attempted.
    let mailbox_resp = r#"{"methodResponses":[["Mailbox/get",{"accountId":"A1","state":"m1","list":[{"id":"mb1","name":"Inbox","role":"inbox"}],"notFound":[]},"0"]],"sessionState":"s0"}"#;

    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .route_body("POST", "/jmap/api", "Mailbox/get", mailbox_resp)
        .start()
        .await;

    let mut session = connect_session(&handle.session_url()).await;
    let err = session
        .append("Archive/2020", b"From: a@example.com\r\n\r\nbody")
        .await
        .expect_err("append into a missing mailbox must fail");
    assert_eq!(err.code(), ErrorCode::ResourceNotFound);

    // No upload must have happened when the target mailbox is gone.
    let captured = handle.captured().await;
    assert!(
        !captured.iter().any(|r| r.path.contains("/jmap/upload/")),
        "no blob should be uploaded when the mailbox does not exist"
    );
}

#[tokio::test]
async fn append_surfaces_import_not_created_as_error() {
    // Email/import reports per-email rejections in `notCreated`; the adapter must
    // surface that as an error so restore reports the failure per message.
    let mailbox_resp = r#"{"methodResponses":[["Mailbox/get",{"accountId":"A1","state":"m1","list":[{"id":"mb1","name":"Inbox","role":"inbox"}],"notFound":[]},"0"]],"sessionState":"s0"}"#;
    let upload_resp = r#"{"accountId":"A1","blobId":"uploaded-b1","type":"message/rfc822","size":42}"#;
    let import_resp = r#"{"methodResponses":[["Email/import",{"accountId":"A1","created":{},"notCreated":{"restore":{"type":"invalidEmail","description":"bad message"}}},"0"]],"sessionState":"s0"}"#;

    let handle = MockJmapServer::new()
        .route("GET", "/jmap/session", session_json(true))
        .route_body("POST", "/jmap/api", "Mailbox/get", mailbox_resp)
        .route_body("POST", "/jmap/api", "Email/import", import_resp)
        .route("POST", "/jmap/upload/", upload_resp)
        .start()
        .await;

    let mut session = connect_session(&handle.session_url()).await;
    let err = session
        .append("Inbox", b"From: a@example.com\r\n\r\nbody")
        .await
        .expect_err("a notCreated import must fail");
    assert_eq!(err.code(), ErrorCode::JmapMethodError);
}
