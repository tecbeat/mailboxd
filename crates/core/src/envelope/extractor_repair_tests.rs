//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project.
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

//! Blob repair: [`repair_envelope_blobs`] against the IMAP mock server, and
//! the helpers that decide which fetched message is the archived one.

use super::*;
use crate::database::{manager::DB_MANAGER, upsert_impl};
use crate::envelope::extractor::{STATUS_COMPLETE, STATUS_FAILED, STATUS_PARTIAL, STATUS_REPAIRED};
use crate::imap::mock_server::{
    examine_response, register_mock_account, uid_fetch_body_response, uid_search_response,
    MockImapServer, MockImapServerHandle,
};
use crate::store::tantivy::envelope::test_support::{
    commit_documents, envelope, envelope_document, init_globals, register_mailbox, unique_id,
};

/// The UID the archive recorded for every fixture message.
const STORED_UID: u32 = 5;

fn plain_mail(message_id: &str) -> Vec<u8> {
    format!(
        "From: sender@example.com\r\n\
         To: archive@example.com\r\n\
         Subject: Plain {message_id}\r\n\
         Message-ID: <{message_id}>\r\n\
         Date: Mon, 1 Jan 2024 10:00:00 +0000\r\n\
         \r\n\
         Body of {message_id}\r\n"
    )
    .into_bytes()
}

fn mail_with_attachment(message_id: &str) -> Vec<u8> {
    format!(
        "From: sender@example.com\r\n\
         To: archive@example.com\r\n\
         Subject: Attachment {message_id}\r\n\
         Message-ID: <{message_id}>\r\n\
         Date: Mon, 1 Jan 2024 10:00:00 +0000\r\n\
         MIME-Version: 1.0\r\n\
         Content-Type: multipart/mixed; boundary=\"b\"\r\n\
         \r\n\
         --b\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         See the attachment.\r\n\
         --b\r\n\
         Content-Type: application/octet-stream\r\n\
         Content-Disposition: attachment; filename=\"report.bin\"\r\n\
         Content-Transfer-Encoding: base64\r\n\
         \r\n\
         {payload}\r\n\
         --b--\r\n",
        payload = base64_of(message_id),
    )
    .into_bytes()
}

fn base64_of(text: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(format!("attachment of {text}"))
}

fn unique_message_id() -> String {
    format!("{}@repair.test", uuid::Uuid::new_v4())
}

/// Which blobs of an archived message are in the store.
#[derive(Clone, Copy)]
enum Stored {
    Everything,
    NoEmail,
    NoAttachments,
}

struct Archived {
    envelope: Envelope,
    raw: Vec<u8>,
    detached: DetachedBody,
}

/// Indexes `raw` as an archived message at [`STORED_UID`] in a fresh account
/// and mailbox and stores the blobs `stored` names.
async fn archive(raw: Vec<u8>, stored: Stored) -> Archived {
    init_globals();
    let (account_id, mailbox_id) = (unique_id(), unique_id());
    register_mailbox(account_id, mailbox_id, "owner@example.com");

    let message = MessageParser::new().parse(raw.as_slice()).unwrap();
    let detached = strip_attachment_bodies(&raw, &message, None);
    let mut envelope = envelope(account_id, mailbox_id, &unique_id().to_string());
    envelope.uid = STORED_UID;
    // Like the ingest path: a message without the header gets a synthetic id.
    envelope.message_id = message
        .message_id()
        .map(String::from)
        .unwrap_or_else(generate_message_id);
    envelope.subject = message.subject().unwrap_or_default().to_string();
    envelope.content_hash = compute_content_hash(&raw);
    envelope.attachment_count = detached.infos.len();
    let attachments = (!detached.infos.is_empty()).then(|| detached.infos.clone());
    commit_documents(
        &ENVELOPE_MANAGER,
        vec![envelope_document(envelope.clone(), attachments, "")],
    )
    .await;

    if !matches!(stored, Stored::NoEmail) {
        BLOB_MANAGER
            .store(DetachedEmail {
                email: (
                    envelope.content_hash.clone(),
                    Bytes::from(detached.stripped_eml.clone()),
                ),
                attachments: None,
            })
            .await
            .unwrap();
    }
    if !matches!(stored, Stored::NoAttachments) {
        store_attachments(&detached.blobs).await;
    }
    Archived {
        envelope,
        raw,
        detached,
    }
}

async fn store_attachments(blobs: &[(String, Bytes)]) {
    for blob in blobs {
        BLOB_MANAGER
            .store(DetachedEmail {
                // A throwaway email key: only the attachment is of interest.
                email: (format!("{:064x}", unique_id()), Bytes::from_static(b"-")),
                attachments: Some(vec![blob.clone()]),
            })
            .await
            .unwrap();
    }
}

/// A mock server for the archived message's account that answers `UID FETCH
/// <uid>` with the given bodies and `UID SEARCH` with `search`.
async fn imap_serving(
    archived: &Archived,
    fetches: &[(u32, &[u8])],
    search: &[u32],
) -> MockImapServerHandle {
    let mut server = MockImapServer::new()
        .with_login()
        .respond("EXAMINE", examine_response("INBOX", 10, 42, 11))
        .respond("UID SEARCH", uid_search_response(search));
    for (uid, body) in fetches {
        server = server.respond(
            format!("UID FETCH {uid} ("),
            uid_fetch_body_response(&[(*uid, body)]),
        );
    }
    let handle = server.start().await;
    register_mock_account(&handle, archived.envelope.account_id);
    handle
}

async fn repair(archived: &Archived) -> BlobRepairReport {
    repair_envelope_blobs(archived.envelope.account_id, &archived.envelope.id)
        .await
        .unwrap()
}

/// The read path rebuilds the archived message byte for byte.
fn assert_readable(archived: &Archived) {
    let (_, restored) =
        reattach_eml_content(archived.envelope.account_id, archived.envelope.id.clone()).unwrap();
    assert_eq!(restored.as_ref(), archived.raw.as_slice());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_intact_message_is_reported_complete_without_contacting_the_server() {
    let archived = archive(
        mail_with_attachment(&unique_message_id()),
        Stored::Everything,
    )
    .await;
    // No IMAP account is registered: any fetch attempt would fail the repair.
    let report = repair(&archived).await;

    assert_eq!(report.status, STATUS_COMPLETE);
    assert_eq!(report.envelope_id, archived.envelope.id);
    assert_eq!(report.account_id, archived.envelope.account_id);
    assert_eq!(report.mailbox_id, archived.envelope.mailbox_id);
    assert_eq!(report.subject, archived.envelope.subject);
    assert_eq!(report.message_id, archived.envelope.message_id);
    assert!(!report.email_missing_before && !report.email_missing_after);
    assert!(report.missing_before.is_empty() && report.missing_after.is_empty());
    assert_eq!(report.error, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_email_blob_is_fetched_again_and_stored() {
    let archived = archive(plain_mail(&unique_message_id()), Stored::NoEmail).await;
    let _imap = imap_serving(&archived, &[(STORED_UID, &archived.raw)], &[]).await;

    let report = repair(&archived).await;

    assert_eq!(report.status, STATUS_REPAIRED);
    assert!(report.email_missing_before);
    assert!(!report.email_missing_after);
    assert_eq!(report.error, None);
    assert_readable(&archived);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_attachment_blob_is_fetched_again_and_stored() {
    let archived = archive(
        mail_with_attachment(&unique_message_id()),
        Stored::NoAttachments,
    )
    .await;
    let storage_hash = archived.detached.infos[0].storage_hash().to_string();
    assert!(BLOB_MANAGER
        .get_attachment(&storage_hash)
        .unwrap()
        .is_none());
    let _imap = imap_serving(&archived, &[(STORED_UID, &archived.raw)], &[]).await;

    let report = repair(&archived).await;

    assert_eq!(report.status, STATUS_REPAIRED);
    assert!(!report.email_missing_before);
    assert_eq!(report.missing_before, [storage_hash.clone()]);
    assert!(report.missing_after.is_empty());
    assert!(BLOB_MANAGER
        .get_attachment(&storage_hash)
        .unwrap()
        .is_some());
    assert_readable(&archived);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_that_changed_on_the_server_is_not_stored() {
    let archived = archive(plain_mail(&unique_message_id()), Stored::NoEmail).await;
    let changed = plain_mail(&unique_message_id());
    let _imap = imap_serving(&archived, &[(STORED_UID, &changed)], &[]).await;

    let report = repair(&archived).await;

    assert_eq!(report.status, STATUS_FAILED);
    let error = report.error.unwrap();
    assert!(
        error.contains(&archived.envelope.content_hash),
        "the error names the archived hash: {error}"
    );
    assert!(report.email_missing_before && report.email_missing_after);
    assert!(BLOB_MANAGER
        .get_email(&archived.envelope.content_hash)
        .unwrap()
        .is_none());
    assert!(BLOB_MANAGER
        .get_email(&compute_content_hash(&changed))
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_with_a_new_uid_is_found_by_its_message_id() {
    let archived = archive(plain_mail(&unique_message_id()), Stored::NoEmail).await;
    let other = plain_mail(&unique_message_id());
    let _imap = imap_serving(
        &archived,
        &[(STORED_UID, &other), (7, &other), (9, &archived.raw)],
        &[STORED_UID, 9, 7],
    )
    .await;

    let report = repair(&archived).await;

    assert_eq!(report.status, STATUS_REPAIRED, "{:?}", report.error);
    assert_readable(&archived);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_gone_from_its_uid_is_found_past_missing_candidates() {
    let archived = archive(plain_mail(&unique_message_id()), Stored::NoEmail).await;
    // Neither the stored UID nor candidate 7 returns a message any more.
    let _imap = imap_serving(&archived, &[(9, &archived.raw)], &[7, 9]).await;

    let report = repair(&archived).await;

    assert_eq!(report.status, STATUS_REPAIRED, "{:?}", report.error);
    assert_readable(&archived);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_found_nowhere_reports_the_last_fetch_error() {
    let archived = archive(plain_mail(&unique_message_id()), Stored::NoEmail).await;
    let _imap = imap_serving(&archived, &[], &[7]).await;

    let report = repair(&archived).await;

    assert_eq!(report.status, STATUS_FAILED);
    let error = report.error.unwrap_or_default();
    assert!(error.contains("UID 7 not found"), "{error}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_without_a_real_message_id_is_not_searched_for() {
    let raw = String::from_utf8(plain_mail("unused"))
        .unwrap()
        .replace("Message-ID: <unused>\r\n", "")
        .into_bytes();
    let archived = archive(raw, Stored::NoEmail).await;
    assert!(archived.envelope.message_id.ends_with("@mailboxd>"));
    let other = plain_mail(&unique_message_id());
    // A search would find the message at UID 9; it must not run.
    let _imap = imap_serving(&archived, &[(STORED_UID, &other), (9, &archived.raw)], &[9]).await;

    let report = repair(&archived).await;

    assert_eq!(report.status, STATUS_FAILED);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unreachable_server_fails_the_repair() {
    let archived = archive(plain_mail(&unique_message_id()), Stored::NoEmail).await;
    let imap = MockImapServer::new().start().await;
    let mut account = register_mock_account(&imap, archived.envelope.account_id);
    // A port that was just free and has no listener any more.
    let closed_port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    account.imap.as_mut().unwrap().port = closed_port;
    upsert_impl(DB_MANAGER.db(), account).unwrap();

    let report = repair(&archived).await;

    assert_eq!(report.status, STATUS_FAILED);
    let error = report.error.unwrap();
    assert!(error.contains("ConnectionRefused"), "{error}");
    assert!(report.email_missing_after);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_corrupt_attachment_blob_stays_partial() {
    let archived = archive(
        mail_with_attachment(&unique_message_id()),
        Stored::NoAttachments,
    )
    .await;
    let storage_hash = archived.detached.infos[0].storage_hash().to_string();
    store_attachments(&[(storage_hash.clone(), Bytes::from_static(b"corrupt"))]).await;
    let _imap = imap_serving(&archived, &[(STORED_UID, &archived.raw)], &[]).await;

    let report = repair(&archived).await;

    // The fetch is verified, but the store keeps the first write for a key.
    assert_eq!(report.status, STATUS_PARTIAL);
    assert_eq!(report.missing_before, [storage_hash.clone()]);
    assert_eq!(report.missing_after, [storage_hash]);
    assert_eq!(report.error, None);
}

/// The envelope lookup resolves the mailbox name, so a deleted mailbox is
/// an error before any fetch.
#[tokio::test(flavor = "multi_thread")]
async fn a_message_of_a_deleted_mailbox_cannot_be_repaired() {
    init_globals();
    let account_id = unique_id();
    upsert_impl(
        DB_MANAGER.db(),
        AccountModel {
            id: account_id,
            ..Default::default()
        },
    )
    .unwrap();
    let mut envelope = envelope(account_id, unique_id(), &unique_id().to_string());
    envelope.uid = STORED_UID;
    commit_documents(
        &ENVELOPE_MANAGER,
        vec![envelope_document(envelope.clone(), None, "")],
    )
    .await;

    let err = repair_envelope_blobs(account_id, &envelope.id)
        .await
        .expect_err("a record without its mailbox cannot be repaired");
    assert!(err.to_string().contains("not found"), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn inconsistent_attachment_metadata_fails_without_a_fetch() {
    let archived = archive(plain_mail(&unique_message_id()), Stored::NoEmail).await;
    let mut broken = archived.envelope.clone();
    broken.id = unique_id().to_string();
    broken.attachment_count = 2;
    commit_documents(
        &ENVELOPE_MANAGER,
        vec![envelope_document(broken.clone(), None, "")],
    )
    .await;
    // A fetch would repair the message; the check must stop before it.
    let _imap = imap_serving(&archived, &[(STORED_UID, &archived.raw)], &[]).await;

    let report = repair_envelope_blobs(broken.account_id, &broken.id)
        .await
        .unwrap();

    assert_eq!(report.status, STATUS_FAILED);
    assert!(report.error.unwrap().contains("Consistency check failed"));
    assert!(BLOB_MANAGER
        .get_email(&broken.content_hash)
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_envelope_is_not_found() {
    init_globals();
    let err = repair_envelope_blobs(unique_id(), "no-such-envelope")
        .await
        .expect_err("an unknown envelope cannot be repaired");
    assert_eq!(err.code(), ErrorCode::ResourceNotFound);
}

#[test]
fn only_real_and_safe_message_ids_are_searched_for() {
    assert_eq!(
        searchable_message_id("  abc@example.com \r\n"),
        Some("abc@example.com")
    );
    assert_eq!(searchable_message_id(""), None);
    assert_eq!(searchable_message_id("   "), None);
    assert_eq!(searchable_message_id("<0123.1.2@mailboxd>"), None);
    assert_eq!(searchable_message_id("<0123.1.2@bichon>"), None);
    assert_eq!(searchable_message_id("a\"b@example.com"), None);
    assert_eq!(searchable_message_id("a\\b@example.com"), None);
    assert_eq!(searchable_message_id("a\rb@example.com"), None);
    assert_eq!(searchable_message_id("a\nb@example.com"), None);
    // Only the suffix marks an id as synthetic.
    assert_eq!(
        searchable_message_id("mailboxd@example.com"),
        Some("mailboxd@example.com")
    );
}

#[test]
fn message_id_candidates_skip_the_stored_uid_sorted_and_bounded() {
    assert_eq!(bounded_message_id_candidates([9, 5, 3, 5], 5), [3, 9]);
    assert!(bounded_message_id_candidates([5], 5).is_empty());

    let many = bounded_message_id_candidates((1..=100).rev(), 0);
    assert_eq!(many.len(), MAX_MESSAGE_ID_CANDIDATES);
    assert_eq!(many.first(), Some(&1));
    assert_eq!(many.last(), Some(&(MAX_MESSAGE_ID_CANDIDATES as u32)));
}

#[test]
fn a_recovered_body_must_match_hash_and_message_id() {
    let body = plain_mail("found@example.com");
    let mut archived = Envelope {
        content_hash: compute_content_hash(&body),
        message_id: "found@example.com".into(),
        ..Default::default()
    };
    assert!(recovered_body_matches(&body, &archived));

    assert!(!recovered_body_matches(b"other bytes", &archived));

    archived.message_id = "someone-else@example.com".into();
    assert!(!recovered_body_matches(&body, &archived));

    archived.message_id = String::new();
    assert!(recovered_body_matches(&body, &archived));
}

fn placeholder(hash: &str) -> String {
    format!("{DETACH_PLACEHOLDER_PREFIX}{hash}>>")
}

#[test]
fn placeholder_hashes_are_listed_once_in_order() {
    let eml = format!(
        "a{}b{}c{}d{}",
        placeholder("h2"),
        placeholder("h1"),
        placeholder("h2"),
        DETACH_PLACEHOLDER_PREFIX,
    );
    // The unterminated placeholder at the end is ignored.
    assert_eq!(placeholder_hashes(eml.as_bytes()), ["h2", "h1"]);
    assert!(placeholder_hashes(b"no placeholders").is_empty());

    // A placeholder whose hash is not UTF-8 is skipped, the next one is kept.
    let mut eml = DETACH_PLACEHOLDER_PREFIX.as_bytes().to_vec();
    eml.extend_from_slice(b"\xff>>");
    eml.extend_from_slice(placeholder("h3").as_bytes());
    assert_eq!(placeholder_hashes(&eml), ["h3"]);
}

#[test]
fn only_placeholders_the_metadata_knows_are_named() {
    let eml = format!(
        "{}{}{}",
        placeholder("raw-key"),
        placeholder("legacy"),
        placeholder("literal-in-body"),
    );
    let attachments = [
        AttachmentInfo {
            content_hash: "decoded".into(),
            raw_content_hash: Some("raw-key".into()),
            ..Default::default()
        },
        AttachmentInfo {
            content_hash: "legacy".into(),
            ..Default::default()
        },
    ];
    assert_eq!(
        known_placeholder_hashes(eml.as_bytes(), &attachments),
        ["raw-key", "legacy"]
    );
    assert!(known_placeholder_hashes(eml.as_bytes(), &[]).is_empty());
}
