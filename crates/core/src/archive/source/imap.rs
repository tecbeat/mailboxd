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

//! IMAP implementation of the [`MailSource`] abstraction.
//!
//! [`ImapSource`] / [`ImapMailSession`] are thin adapters over the existing
//! IMAP code (`ImapExecutor`, `ImapConnectionManager`, `mailbox::list`). They
//! introduce **no new behaviour**: each method delegates to the same functions
//! the production sync flow already calls today. The production flow is not yet
//! rewired onto this adapter — that migration is issue #49. Keeping this purely
//! additive makes the trait extraction safe to land on its own.
//!
//! ## Cursor encoding
//!
//! IMAP incremental sync is driven by `(uid_validity, highest_uid)`. This
//! adapter encodes that pair into the opaque [`SyncCursor`] as
//! `"{uid_validity}:{highest_uid}"` (either component may be empty when the
//! server has not reported it). IMAP-specific reconcile policy (the
//! `UIDVALIDITY`-change handling, anomaly guards, reconnect/backoff) stays in
//! the existing flow and is intentionally *not* duplicated here; the adapter
//! only reports `requires_full_resync` when it detects a `uid_validity` change,
//! leaving the idempotent reconcile to the engine + content-hash dedup.

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use super::{MailSession, MailSource, MailboxChanges, MessageRef, RawMessage, SyncCursor};
use crate::{
    archive::imap::mailbox::MailBox,
    encode_mailbox_name,
    error::MailboxdResult,
    imap::{executor::ImapExecutor, session::SessionStream},
    mailbox::list::convert_names_to_mailboxes,
};

/// Encode an IMAP `(uid_validity, highest_uid)` pair into an opaque cursor.
fn encode_cursor(uid_validity: Option<u32>, highest_uid: Option<u32>) -> SyncCursor {
    let uv = uid_validity.map(|v| v.to_string()).unwrap_or_default();
    let hu = highest_uid.map(|v| v.to_string()).unwrap_or_default();
    SyncCursor::new(format!("{uv}:{hu}"))
}

/// Decode an opaque cursor back into `(uid_validity, highest_uid)`.
///
/// Returns `(None, None)` for a malformed or empty cursor so callers fall back
/// to a full enumeration rather than erroring.
fn decode_cursor(cursor: &SyncCursor) -> (Option<u32>, Option<u32>) {
    let mut parts = cursor.as_str().splitn(2, ':');
    let uid_validity = parts.next().and_then(|s| s.parse::<u32>().ok());
    let highest_uid = parts.next().and_then(|s| s.parse::<u32>().ok());
    (uid_validity, highest_uid)
}

/// A live IMAP session wrapping the existing `async_imap` session plus the
/// owning `account_id` (needed for mailbox conversion via STATUS).
pub struct ImapMailSession {
    account_id: u64,
    session: async_imap::Session<Box<dyn SessionStream>>,
}

#[async_trait]
impl MailSession for ImapMailSession {
    async fn list_mailboxes(&mut self) -> MailboxdResult<Vec<MailBox>> {
        let names = ImapExecutor::list_all_mailboxes(&mut self.session).await?;
        convert_names_to_mailboxes(self.account_id, &mut self.session, names.iter()).await
    }

    async fn changes_since(
        &mut self,
        mailbox: &MailBox,
        cursor: Option<SyncCursor>,
        _token: CancellationToken,
    ) -> MailboxdResult<MailboxChanges> {
        // The cursor carries the last-seen (uid_validity, highest_uid). When the
        // server's current uid_validity differs, all previously stored UIDs are
        // invalid and the engine must reconcile from scratch (idempotently, via
        // content-hash dedup) rather than trusting highest_uid.
        let (prev_uid_validity, prev_highest_uid) =
            cursor.as_ref().map(decode_cursor).unwrap_or((None, None));

        let requires_full_resync = match (prev_uid_validity, mailbox.uid_validity) {
            (Some(prev), Some(current)) => prev != current,
            _ => false,
        };

        // Next fetch starts one past the last stored UID (or from 1 on a fresh
        // mailbox / after a validity change).
        let start_uid = if requires_full_resync {
            1
        } else {
            prev_highest_uid
                .or(mailbox.highest_uid)
                .map(|u| u.saturating_add(1))
                .unwrap_or(1)
        };

        let uids = ImapExecutor::enumerate_uids_from(
            &mut self.session,
            &mailbox.encoded_name(),
            start_uid,
        )
        .await?;

        let new_highest = uids.iter().copied().max().or(mailbox.highest_uid);
        let cursor = new_highest.map(|hu| encode_cursor(mailbox.uid_validity, Some(hu)));

        Ok(MailboxChanges {
            messages: uids.into_iter().map(MessageRef::from_uid).collect(),
            cursor,
            requires_full_resync,
        })
    }

    async fn load_raw(
        &mut self,
        mailbox: &MailBox,
        msg: &MessageRef,
    ) -> MailboxdResult<RawMessage> {
        let uid = msg.as_uid().ok_or_else(|| {
            crate::raise_error!(
                format!("IMAP message reference is not a UID: {}", msg.as_str()),
                crate::error::code::ErrorCode::InvalidParameter
            )
        })?;
        let body = ImapExecutor::fetch_single_message_body(
            &mut self.session,
            &mailbox.encoded_name(),
            uid,
        )
        .await?;
        Ok(RawMessage {
            size: body.len() as u32,
            body,
            uid,
            internal_date: 0,
        })
    }

    async fn append(&mut self, mailbox: &str, eml: &[u8]) -> MailboxdResult<()> {
        ImapExecutor::append(
            &mut self.session,
            encode_mailbox_name!(mailbox),
            None,
            None,
            eml,
        )
        .await
    }

    async fn logout(&mut self) -> MailboxdResult<()> {
        self.session.logout().await.ok();
        Ok(())
    }
}

/// The IMAP [`MailSource`]: a factory that opens [`ImapMailSession`]s.
#[derive(Clone, Copy, Debug, Default)]
pub struct ImapSource;

#[async_trait]
impl MailSource for ImapSource {
    fn name(&self) -> &'static str {
        "imap"
    }

    async fn connect(&self, account_id: u64) -> MailboxdResult<Box<dyn MailSession>> {
        let session = ImapExecutor::create_connection(account_id).await?;
        Ok(Box::new(ImapMailSession {
            account_id,
            session,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_cursor, encode_cursor};
    use crate::archive::source::{MessageRef, SyncCursor};

    #[test]
    fn cursor_round_trips() {
        let c = encode_cursor(Some(42), Some(1000));
        assert_eq!(c.as_str(), "42:1000");
        assert_eq!(decode_cursor(&c), (Some(42), Some(1000)));
    }

    #[test]
    fn cursor_handles_missing_components() {
        assert_eq!(decode_cursor(&SyncCursor::new(":")), (None, None));
        assert_eq!(decode_cursor(&SyncCursor::new("7:")), (Some(7), None));
        assert_eq!(decode_cursor(&SyncCursor::new(":9")), (None, Some(9)));
        assert_eq!(decode_cursor(&SyncCursor::new("")), (None, None));
    }

    #[test]
    fn cursor_handles_garbage() {
        assert_eq!(decode_cursor(&SyncCursor::new("abc:def")), (None, None));
    }

    #[test]
    fn message_ref_uid_round_trip() {
        let r = MessageRef::from_uid(12345);
        assert_eq!(r.as_str(), "12345");
        assert_eq!(r.as_uid(), Some(12345));
    }
}
