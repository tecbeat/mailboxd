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

//! Mail-source abstraction.
//!
//! This module defines [`MailSource`] / [`MailSession`]: the seam between the
//! part of mailboxd that *produces* mail (IMAP today, JMAP next) and the part
//! that *stores and indexes* it (`envelope::extractor` → content-addressable
//! blob store + Tantivy, which is already source-agnostic).
//!
//! The trait pair captures the five operations any mail source must provide:
//!
//! 1. [`MailSource::connect`] — open a live [`MailSession`] for an account.
//! 2. [`MailSession::list_mailboxes`] — enumerate selectable mailboxes.
//! 3. [`MailSession::changes_since`] — list messages changed since a cursor.
//! 4. [`MailSession::load_raw`] — download one raw RFC 5322 message.
//! 5. [`MailSession::append`] — write a raw message back (used by restore).
//!
//! The design is deliberately **object-safe**: `Box<dyn MailSource>` and
//! `Box<dyn MailSession>` are usable directly, so the sync engine (issue #49)
//! can be written generically over `dyn MailSource` without monomorphising per
//! source. That is why the session is a separate trait object rather than an
//! associated `type Session` (an associated type would make the trait
//! non-dyn-compatible).
//!
//! This module only introduces the seam. The IMAP implementation
//! ([`imap::ImapSource`]) wraps the existing IMAP code with no behavioural
//! change, and the production sync flow is *not* yet rewired onto it — that
//! migration happens in issue #49.

pub mod imap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::{
    account::migration::AccountType, archive::imap::mailbox::MailBox, error::MailboxdResult,
};

/// An opaque, source-defined per-mailbox synchronization cursor.
///
/// The sync engine treats this as an opaque token: it persists whatever a
/// [`MailSession`] hands back and returns the last value on the next run. Each
/// source defines its own encoding:
///
/// * IMAP encodes `(uid_validity, highest_uid)` (see [`imap`]).
/// * JMAP will store its `state` string verbatim.
///
/// Keeping the cursor opaque is what lets the same engine drive UID-based and
/// state-string-based sources without special-casing either.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncCursor(pub String);

impl SyncCursor {
    /// Wrap a raw, already-encoded cursor token.
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// The raw cursor token as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An opaque, source-defined identifier for a single message within a mailbox.
///
/// * IMAP encodes the message UID (a `u32`) as its decimal string.
/// * JMAP will use the email/blob id string.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageRef(pub String);

impl MessageRef {
    /// Build a reference from an IMAP UID.
    pub fn from_uid(uid: u32) -> Self {
        Self(uid.to_string())
    }

    /// Interpret this reference as an IMAP UID, if it is one.
    pub fn as_uid(&self) -> Option<u32> {
        self.0.parse().ok()
    }

    /// The raw identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A raw message downloaded from a source, ready for the storage/indexing
/// pipeline (`envelope::extractor::extract_envelope_from_raw`).
///
/// The fields mirror exactly what `extract_envelope_core` consumes, so a source
/// only has to produce this struct — everything downstream (dedup, parsing,
/// attachment detaching, Tantivy indexing) is shared.
#[derive(Clone, Debug)]
pub struct RawMessage {
    /// The raw RFC 5322 / EML bytes.
    pub body: Vec<u8>,
    /// The IMAP UID, or `0` when the source has no UID concept (matching the
    /// convention used by `extract_envelope_from_eml` / `_from_smtp`).
    pub uid: u32,
    /// The message size in bytes as reported by the source (falls back to the
    /// body length when unknown).
    pub size: u32,
    /// The server-side internal date in epoch milliseconds, or `0` if unknown.
    pub internal_date: i64,
}

impl RawMessage {
    /// Build a [`RawMessage`] from raw bytes with no UID/date metadata,
    /// deriving the size from the body length.
    pub fn from_body(body: Vec<u8>) -> Self {
        let size = body.len() as u32;
        Self {
            body,
            uid: 0,
            size,
            internal_date: 0,
        }
    }
}

/// The result of enumerating changes in a mailbox since a [`SyncCursor`].
#[derive(Clone, Debug, Default)]
pub struct MailboxChanges {
    /// References to messages that are new or changed since the cursor and
    /// should be downloaded via [`MailSession::load_raw`].
    pub messages: Vec<MessageRef>,
    /// The cursor to persist once these messages have been processed. `None`
    /// means "leave the stored cursor unchanged" (e.g. nothing new arrived).
    pub cursor: Option<SyncCursor>,
    /// Set when the source signalled that its incremental state is no longer
    /// usable and a full reconcile is required without creating duplicates
    /// (JMAP `cannotCalculateChanges`, IMAP `UIDVALIDITY` change). The engine
    /// relies on content-hash dedup to make the reconcile idempotent.
    pub requires_full_resync: bool,
}

/// A live connection to a mail source for a single account.
///
/// Obtained from [`MailSource::connect`]. All methods take `&mut self` because
/// the underlying protocol session is stateful (a selected mailbox, an
/// in-flight request pipeline, …).
#[async_trait]
pub trait MailSession: Send {
    /// List the account's selectable mailboxes with their metadata (name,
    /// hierarchy, role/attributes and — where cheap — message counts and the
    /// source's native cursor fields).
    async fn list_mailboxes(&mut self) -> MailboxdResult<Vec<MailBox>>;

    /// Enumerate the messages in `mailbox` that changed since `cursor`.
    ///
    /// `cursor == None` means "no prior state" and should yield everything the
    /// source considers in scope (the engine still relies on content-hash
    /// dedup to avoid re-storing already-archived mail).
    async fn changes_since(
        &mut self,
        mailbox: &MailBox,
        cursor: Option<SyncCursor>,
        token: CancellationToken,
    ) -> MailboxdResult<MailboxChanges>;

    /// Download the raw RFC 5322 bytes (plus size/date metadata) of a single
    /// message identified by `msg` within `mailbox`.
    async fn load_raw(&mut self, mailbox: &MailBox, msg: &MessageRef)
        -> MailboxdResult<RawMessage>;

    /// Append/import a raw RFC 5322 message into the named mailbox on the
    /// server. Used by restore (issue #52 / FA-17).
    async fn append(&mut self, mailbox: &str, eml: &[u8]) -> MailboxdResult<()>;

    /// Cleanly close the session. Errors are best-effort and may be ignored by
    /// callers, matching the existing IMAP `logout().await.ok()` pattern.
    async fn logout(&mut self) -> MailboxdResult<()>;
}

/// A factory that opens [`MailSession`]s for accounts of one source type.
///
/// This is the object-safe entry point the sync engine will hold as
/// `Box<dyn MailSource>` (issue #49). Choosing the right implementation for an
/// account is a simple match on its `account_type`.
#[async_trait]
pub trait MailSource: Send + Sync {
    /// A short, stable name for this source (`"imap"`, `"jmap"`), for logs.
    fn name(&self) -> &'static str;

    /// Open a live session for the given account.
    async fn connect(&self, account_id: u64) -> MailboxdResult<Box<dyn MailSession>>;
}

/// Resolve the [`MailSource`] for an account type — the single place that maps
/// `AccountType` → source implementation.
///
/// Every source-generic consumer (restore today; future features) goes through
/// here, so adding a new protocol means implementing [`MailSource`] and adding
/// one arm below — no scattered `match account_type` blocks.
///
/// Returns `None` for account types that have no live server to talk to
/// (`NoSync`). Note this is intentionally *not* used by the archival sync
/// dispatch (`archive::imap::task`): IMAP keeps its dedicated, battle-tested
/// download flow rather than running through the generic engine (see #49), so
/// that dispatch is the one documented exception to this mapping.
pub fn mail_source_for(account_type: AccountType) -> Option<Box<dyn MailSource>> {
    match account_type {
        AccountType::IMAP => Some(Box::new(imap::ImapSource)),
        AccountType::JMAP => Some(Box::new(crate::archive::jmap::source::JmapSource)),
        AccountType::NoSync => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_message_from_body_derives_size_and_defaults() {
        let body = b"From: a@example.com\r\nSubject: hi\r\n\r\nbody".to_vec();
        let expected_len = body.len() as u32;
        let msg = RawMessage::from_body(body);
        assert_eq!(msg.size, expected_len);
        assert_eq!(msg.uid, 0);
        assert_eq!(msg.internal_date, 0);
        assert_eq!(msg.body.len() as u32, expected_len);
    }

    #[test]
    fn sync_cursor_preserves_token() {
        let cursor = SyncCursor::new("state-abc-123");
        assert_eq!(cursor.as_str(), "state-abc-123");
    }

    #[test]
    fn message_ref_non_uid_source_has_no_uid() {
        let jmap_like = MessageRef("Mabc123".to_string());
        assert_eq!(jmap_like.as_uid(), None);
        assert_eq!(jmap_like.as_str(), "Mabc123");
    }

    #[test]
    fn mailbox_changes_default_is_empty_and_stable() {
        let changes = MailboxChanges::default();
        assert!(changes.messages.is_empty());
        assert!(changes.cursor.is_none());
        assert!(!changes.requires_full_resync);
    }

    /// The trait pair must stay object-safe so the sync engine (#49) can hold a
    /// `Box<dyn MailSource>` and drive `Box<dyn MailSession>` generically. This
    /// compiles only if both traits are dyn-compatible.
    #[test]
    fn traits_are_object_safe() {
        fn _assert_source(_: &dyn MailSource) {}
        fn _assert_session(_: &dyn MailSession) {}
    }
}
