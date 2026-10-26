//
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

//! Test support for the tantivy stores: an email index manager on a private
//! directory, the global managers on a runtime that outlives every test, and
//! the account/mailbox records that turning a hit into an envelope needs.

use std::{
    path::Path,
    sync::{LazyLock, OnceLock},
};

use bytes::Bytes;
use tantivy::TantivyDocument;
use tokio::{runtime::Runtime, sync::Mutex};

use super::IndexManager;
use crate::{
    account::migration::AccountModel,
    archive::imap::mailbox::MailBox,
    database::{manager::DB_MANAGER, upsert_impl},
    message::content::AttachmentInfo,
    store::{
        blob::{DetachedEmail, BLOB_MANAGER},
        envelope::Envelope,
        tantivy::{
            attachment::ATTACHMENT_MANAGER, envelope::ENVELOPE_MANAGER,
            model::EnvelopeWithAttachments, tokenizers::register_tokenizers,
        },
    },
};

/// An email index manager on `dir`, opened (or created) the way the global
/// one opens its directory. It has no background ingest task.
pub(crate) fn envelope_manager_at(dir: &Path) -> IndexManager {
    let index = IndexManager::open_or_create_index(&dir.to_path_buf());
    register_tokenizers(&index);
    let index_writer = index.writer_with_num_threads(1, 15_000_000).unwrap();
    let reader = index.reader().unwrap();
    let (sender, _receiver) = tokio::sync::mpsc::channel(1);
    IndexManager {
        index: std::sync::Arc::new(index),
        index_writer: std::sync::Arc::new(Mutex::new(index_writer)),
        sender,
        reader,
        handle: Mutex::new(None),
    }
}

/// The global managers spawn background tasks when first used. Initialise
/// them on a runtime that is never dropped, so those tasks outlive the test
/// that happened to touch them first.
pub(crate) fn init_globals() {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    let runtime = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap()
    });
    let _guard = runtime.enter();
    LazyLock::force(&BLOB_MANAGER);
    LazyLock::force(&ENVELOPE_MANAGER);
    LazyLock::force(&ATTACHMENT_MANAGER);
}

/// An id no earlier test run used: the global indexes and the database live
/// in `MAILBOXD_ROOT_DIR` and survive between runs.
pub(crate) fn unique_id() -> u64 {
    crate::id!(64)
}

/// A blob key (64 hex characters) unique to `seed`.
pub(crate) fn blob_key(seed: u64) -> String {
    format!("{seed:064x}")
}

/// Account and mailbox records, so search hits resolve to envelopes.
pub(crate) fn register_mailbox(account_id: u64, mailbox_id: u64, email: &str) {
    upsert_impl(
        DB_MANAGER.db(),
        AccountModel {
            id: account_id,
            email: email.to_string(),
            ..Default::default()
        },
    )
    .unwrap();
    upsert_impl(
        DB_MANAGER.db(),
        MailBox {
            id: mailbox_id,
            account_id,
            name: "INBOX".to_string(),
            ..Default::default()
        },
    )
    .unwrap();
}

/// An envelope with every field the index needs to give it back.
pub(crate) fn envelope(account_id: u64, mailbox_id: u64, id: &str) -> Envelope {
    Envelope {
        id: id.to_string(),
        message_id: format!("<{id}@test>"),
        account_id,
        mailbox_id,
        uid: 1,
        subject: String::new(),
        from: "sender@example.com".to_string(),
        date: 1_700_000_000_000,
        internal_date: 1_700_000_000_000,
        ingest_at: 1_700_000_000_000,
        size: 100,
        thread_id: id.to_string(),
        content_hash: blob_key(unique_id()),
        ..Default::default()
    }
}

/// A regular (non-inline) attachment called `name`.
pub(crate) fn attachment(name: &str, content_hash: &str) -> AttachmentInfo {
    AttachmentInfo {
        file_type: "application/pdf".to_string(),
        filename: Some(name.to_string()),
        size: 10,
        content_hash: content_hash.to_string(),
        ..Default::default()
    }
}

/// The document the ingest path would write for `envelope` with `body`.
pub(crate) fn envelope_document(
    envelope: Envelope,
    attachments: Option<Vec<AttachmentInfo>>,
    body: &str,
) -> TantivyDocument {
    EnvelopeWithAttachments {
        envelope,
        attachments,
    }
    .to_document(body, 0)
    .unwrap()
}

/// Add and commit documents through the manager's writer.
pub(crate) async fn commit_documents(manager: &IndexManager, docs: Vec<TantivyDocument>) {
    let mut writer = manager.index_writer.lock().await;
    for doc in docs {
        writer.add_document(doc).unwrap();
    }
    writer.commit().unwrap();
}

/// Store an EML blob.
pub(crate) async fn store_email_blob(content_hash: &str, eml: &[u8]) {
    init_globals();
    BLOB_MANAGER.store_now(DetachedEmail {
        email: (content_hash.to_string(), Bytes::copy_from_slice(eml)),
        attachments: None,
    });
    assert!(
        BLOB_MANAGER.get_email(content_hash).unwrap().is_some(),
        "blob {content_hash} was not stored"
    );
}
