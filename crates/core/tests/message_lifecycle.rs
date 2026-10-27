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

//! Import → read → delete of a stored message, through the public API.
//!
//! Runs as its own test binary: the index and blob writers are background
//! tasks bound to the runtime that first touches them, so the whole flow
//! must share one runtime. The index writer only commits on a timer, a size
//! threshold, or shutdown; the test triggers the shutdown commit once the
//! message is imported. Deletions commit directly and still work afterwards.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use mailboxd_core::{
    account::migration::{Account, AccountType},
    common::signal::SIGNAL_MANAGER,
    database::{manager::DB_MANAGER, upsert_impl},
    error::code::ErrorCode,
    import::{BatchEmlRequest, ImportEmls},
    message::{
        content::{retrieve_email_content, retrieve_nested_eml_content},
        delete::delete_messages_impl,
        search::{search_messages_impl, EmailSearchFilter, EmailSearchRequest},
    },
    store::tantivy::envelope::ENVELOPE_MANAGER,
};

/// Account id used by no other test.
const ACCOUNT_ID: u64 = 63_100_000_000_001;

/// A message with an attachment lacking a `Content-Type` header
/// (rustmailer/bichon#369) and a forwarded message that has its own
/// attachment.
const EML: &str = concat!(
    "From: sender@example.com\r\n",
    "To: archive@example.com\r\n",
    "Subject: Lifecycle test\r\n",
    "Message-ID: <lifecycle-outer@example.com>\r\n",
    "Date: Mon, 1 Jan 2024 10:00:00 +0000\r\n",
    "MIME-Version: 1.0\r\n",
    "Content-Type: multipart/mixed; boundary=outer\r\n",
    "\r\n",
    "--outer\r\n",
    "Content-Type: text/plain\r\n",
    "\r\n",
    "Outer body\r\n",
    "--outer\r\n",
    "Content-Disposition: attachment; filename=report.pdf\r\n",
    "\r\n",
    "%PDF-1.4 not really a pdf\r\n",
    "--outer\r\n",
    "Content-Type: message/rfc822\r\n",
    "Content-Disposition: attachment; filename=forwarded.eml\r\n",
    "\r\n",
    "From: inner@example.com\r\n",
    "Subject: Forwarded\r\n",
    "Message-ID: <lifecycle-inner@example.com>\r\n",
    "Date: Sun, 31 Dec 2023 10:00:00 +0000\r\n",
    "MIME-Version: 1.0\r\n",
    "Content-Type: multipart/mixed; boundary=inner\r\n",
    "\r\n",
    "--inner\r\n",
    "Content-Type: text/plain\r\n",
    "\r\n",
    "Inner body\r\n",
    "--inner\r\n",
    "Content-Type: text/csv\r\n",
    "Content-Disposition: attachment; filename=data.csv\r\n",
    "\r\n",
    "a,b\r\n",
    "--inner--\r\n",
    "\r\n",
    "--outer--\r\n",
);

fn stored_envelope_ids() -> Vec<String> {
    let page = search_messages_impl(
        Some(HashSet::from([ACCOUNT_ID])),
        EmailSearchRequest {
            filter: EmailSearchFilter::default(),
            page: 1,
            page_size: 10,
            ..Default::default()
        },
    )
    .unwrap();
    page.items.into_iter().map(|envelope| envelope.id).collect()
}

/// Wait until the committed message is visible to readers.
async fn wait_for_single_envelope() -> String {
    for _ in 0..100 {
        let ids = stored_envelope_ids();
        if let [id] = ids.as_slice() {
            return id.clone();
        }
        assert!(ids.is_empty(), "expected one envelope, got {ids:?}");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the imported message never became searchable");
}

async fn import_message() -> String {
    // Leftovers of an aborted earlier run would make the import a duplicate.
    ENVELOPE_MANAGER
        .delete_account_envelopes(ACCOUNT_ID)
        .await
        .unwrap();
    upsert_impl(
        DB_MANAGER.db(),
        Account {
            id: ACCOUNT_ID,
            email: "archive@example.com".into(),
            account_type: AccountType::NoSync,
            enabled: true,
            ..Default::default()
        },
    )
    .unwrap();

    let result = ImportEmls::do_import(BatchEmlRequest {
        account_id: ACCOUNT_ID,
        mail_folder: "INBOX".into(),
        emls: vec![mailboxd_core::base64_encode_url_safe!(EML.as_bytes())],
    })
    .await
    .unwrap();
    assert_eq!(result.success, 1, "import failed: {result:?}");

    // Let the index writer pick the document up, then force its commit.
    tokio::time::sleep(Duration::from_millis(500)).await;
    SIGNAL_MANAGER.trigger_shutdown();
    ENVELOPE_MANAGER.shutdown().await;
    wait_for_single_envelope().await
}

#[test]
fn imported_message_can_be_read_and_deleted() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let envelope_id = import_message().await;

        // Reading the message reports both attachments; the one without a
        // Content-Type gets a type guessed from its filename.
        let content = retrieve_email_content(ACCOUNT_ID, envelope_id.clone(), false).unwrap();
        assert_eq!(content.text.as_deref().map(str::trim), Some("Outer body"));
        let attachments = content.attachments.expect("attachments are listed");
        let report = attachments
            .iter()
            .find(|a| a.filename.as_deref() == Some("report.pdf"))
            .expect("report.pdf is listed");
        assert_eq!(report.file_type, "application/pdf");
        assert!(!report.is_message);
        assert_eq!(report.raw_content_hash, None);
        let forwarded = attachments
            .iter()
            .find(|a| a.is_message)
            .expect("forwarded message is listed");
        assert_eq!(forwarded.file_type, "message/rfc822");

        // The forwarded message can be opened with its own attachment.
        let nested = retrieve_nested_eml_content(
            ACCOUNT_ID,
            envelope_id.clone(),
            &forwarded.content_hash,
            false,
        )
        .unwrap();
        let nested_attachments = nested.attachments.expect("attachments are listed");
        let csv = nested_attachments
            .iter()
            .find(|a| a.filename.as_deref() == Some("data.csv"))
            .expect("data.csv is listed");
        assert_eq!(csv.file_type, "text/csv");
        // The CRLF before the boundary belongs to the boundary (RFC 2046).
        assert_eq!(csv.size, "a,b".len());
        assert_eq!(csv.raw_content_hash, None);

        // Deleting reports the number of deleted messages ...
        let request = HashMap::from([(ACCOUNT_ID, vec![envelope_id.clone()])]);
        let deleted = delete_messages_impl(request.clone()).await.unwrap();
        assert_eq!(deleted.deleted, 1);
        assert!(stored_envelope_ids().is_empty());

        // ... and deleting it again finds nothing.
        let err = delete_messages_impl(request).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::ResourceNotFound);
    });
}
