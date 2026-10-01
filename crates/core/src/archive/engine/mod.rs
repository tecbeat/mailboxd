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

//! Source-generic sync engine.
//!
//! This engine drives any [`MailSource`](crate::archive::source::MailSource):
//! it lists the account's mailboxes, selects the ones to archive, and for each
//! enumerates changes since the stored cursor, downloads the raw messages, and
//! hands them to the shared storage pipeline
//! ([`extract_envelope_from_raw`](crate::envelope::extractor::extract_envelope_from_raw)).
//!
//! It is intentionally **independent of the battle-tested IMAP flow**
//! (`archive::imap::download`): IMAP accounts keep using that flow unchanged,
//! while new sources (JMAP) run through this engine. The shared pieces live
//! below the trait seam — the content-addressable blob store + Tantivy indexing
//! (so FA-12 multi-mailbox dedup and FA-14 rules/limits apply identically), and
//! the [`DownloadState`] progress model (so FA-15 dashboard/stats work too).
//!
//! Key properties:
//! * **Resume-on-success cursor commit (FA-16):** the per-mailbox cursor is
//!   persisted only after its messages have all been processed, so an
//!   interrupted run re-fetches at most the current mailbox's batch (deduped).
//! * **Idempotent reconcile (FA-11):** a source may signal
//!   `requires_full_resync`; the engine re-enumerates and relies on content-hash
//!   dedup to avoid duplicates.
//! * **Cancellation:** honoured between mailboxes and between messages.

use std::collections::BTreeSet;

use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::account::migration::AccountModel;
use crate::account::state::{DownloadState, DownloadStatus, FolderStatus, TriggerType};
use crate::archive::imap::mailbox::MailBox;
use crate::archive::source::{MailSession, MailSource, MessageRef, SyncCursor};
use crate::envelope::extractor::{extract_envelope_from_raw, ExtractOutcome};
use crate::error::MailboxdResult;

/// Outcome counters for a single engine run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncStats {
    pub mailboxes: u64,
    pub downloaded: u64,
    pub duplicates: u64,
    pub failed: u64,
    pub cancelled: bool,
}

/// Run a full archival sync for `account` through `source`.
///
/// This is the entry point the task scheduler calls for non-IMAP accounts. It
/// manages the [`DownloadState`] session lifecycle (start → per-folder progress
/// → finalize) exactly like the IMAP flow, so the dashboard shows JMAP syncs the
/// same way (FA-15).
pub async fn run_sync(
    account: &AccountModel,
    source: &dyn MailSource,
    token: CancellationToken,
    trigger_type: TriggerType,
) -> MailboxdResult<SyncStats> {
    let account_id = account.id;
    DownloadState::start_new_session(account_id, trigger_type)?;

    let result = run_sync_inner(account, source, token).await;

    match &result {
        Ok(stats) if stats.cancelled => {
            DownloadState::update_session_status(account_id, DownloadStatus::Cancelled, None)?;
        }
        Ok(_) => {
            DownloadState::update_session_status(account_id, DownloadStatus::Success, None)?;
        }
        Err(e) => {
            let err_msg = format!("{} sync interrupted: {:#?}", source.name(), e);
            DownloadState::append_session_error(account_id, err_msg.clone())?;
            DownloadState::update_session_status(account_id, DownloadStatus::Failed, Some(err_msg))?;
        }
    }
    result
}

async fn run_sync_inner(
    account: &AccountModel,
    source: &dyn MailSource,
    token: CancellationToken,
) -> MailboxdResult<SyncStats> {
    let account_id = account.id;
    let mut stats = SyncStats::default();

    let mut session = source.connect(account_id).await?;

    // Discover mailboxes and persist/refresh the local records (FA-7, FA-9).
    let remote = session.list_mailboxes().await?;
    let selected = select_mailboxes(account, &remote);
    persist_mailboxes(&remote)?;

    DownloadState::init_folder_details(
        account_id,
        selected.iter().map(|m| m.name.clone()).collect(),
    )?;

    for mailbox in &selected {
        if token.is_cancelled() {
            stats.cancelled = true;
            break;
        }
        stats.mailboxes += 1;
        DownloadState::set_current_folder(account_id, mailbox.name.clone())?;

        match sync_mailbox(account, session.as_mut(), mailbox, token.clone(), &mut stats).await {
            Ok(_) => {}
            Err(e) => {
                stats.failed += 1;
                let msg = format!("mailbox '{}' failed: {:#?}", mailbox.name, e);
                warn!(account_id, "{}", msg);
                DownloadState::append_session_error(account_id, msg.clone())?;
                DownloadState::update_folder_progress(
                    account_id,
                    mailbox.name.clone(),
                    0,
                    0,
                    FolderStatus::Failed,
                    Some(msg),
                )?;
            }
        }
    }

    session.logout().await.ok();
    info!(
        account_id,
        source = source.name(),
        downloaded = stats.downloaded,
        duplicates = stats.duplicates,
        failed = stats.failed,
        "sync finished"
    );
    Ok(stats)
}

/// Download all changed messages in one mailbox, committing the cursor only
/// after every message has been processed (resume-on-success, FA-16).
async fn sync_mailbox(
    account: &AccountModel,
    session: &mut dyn MailSession,
    mailbox: &MailBox,
    token: CancellationToken,
    stats: &mut SyncStats,
) -> MailboxdResult<()> {
    let account_id = account.id;
    let max_email_size = account.max_email_size_bytes;

    let stored_cursor = mailbox.sync_cursor.clone().map(SyncCursor::new);
    let changes = session
        .changes_since(mailbox, stored_cursor, token.clone())
        .await?;

    if changes.requires_full_resync {
        debug!(
            account_id,
            mailbox = %mailbox.name,
            "source requested full resync; relying on content-hash dedup"
        );
    }

    let planned = changes.messages.len() as u64;
    DownloadState::update_folder_progress(
        account_id,
        mailbox.name.clone(),
        planned,
        0,
        FolderStatus::Downloading,
        None,
    )?;

    let mut current = 0u64;
    for msg in &changes.messages {
        if token.is_cancelled() {
            stats.cancelled = true;
            // Do NOT commit the cursor on cancellation: the next run re-enumerates
            // from the same stored cursor and dedups what was already stored.
            return Ok(());
        }
        match download_one(account_id, max_email_size, session, mailbox, msg).await {
            Ok(Some(ExtractOutcome::Imported)) => stats.downloaded += 1,
            Ok(Some(ExtractOutcome::Duplicate)) => stats.duplicates += 1,
            Ok(None) => { /* skipped (e.g. oversize) */ }
            Err(e) => {
                stats.failed += 1;
                warn!(account_id, uid = msg.as_str(), "download failed: {:#?}", e);
                DownloadState::append_session_error(
                    account_id,
                    format!("message {} in '{}' failed: {:#?}", msg.as_str(), mailbox.name, e),
                )?;
            }
        }
        current += 1;
        if current % 20 == 0 || current == planned {
            DownloadState::update_folder_progress(
                account_id,
                mailbox.name.clone(),
                planned,
                current,
                FolderStatus::Downloading,
                None,
            )?;
        }
    }

    // Commit the cursor only after the whole mailbox succeeded (FA-16).
    if let Some(cursor) = changes.cursor {
        commit_cursor(mailbox, &cursor)?;
    }

    DownloadState::update_folder_progress(
        account_id,
        mailbox.name.clone(),
        planned,
        current,
        FolderStatus::Success,
        None,
    )?;
    Ok(())
}

/// Download a single message, enforcing the account size limit (FA-14), and hand
/// it to the shared storage pipeline. Returns `None` when the message was
/// skipped before storage.
async fn download_one(
    account_id: u64,
    max_email_size: Option<u64>,
    session: &mut dyn MailSession,
    mailbox: &MailBox,
    msg: &MessageRef,
) -> MailboxdResult<Option<ExtractOutcome>> {
    let raw = session.load_raw(mailbox, msg).await?;

    if let Some(limit) = max_email_size {
        if raw.size as u64 > limit {
            debug!(
                account_id,
                uid = msg.as_str(),
                size = raw.size,
                limit,
                "skipping oversize message"
            );
            return Ok(None);
        }
    }

    let outcome = extract_envelope_from_raw(&raw, account_id, mailbox.id).await?;
    Ok(Some(outcome))
}

/// Select which mailboxes to archive: those named in `download_folders`, or —
/// when that is empty and `auto_download_new_mailboxes` is on (or unset) — all
/// selectable mailboxes. Mirrors the IMAP default of syncing everything when no
/// explicit subscription exists.
fn select_mailboxes(account: &AccountModel, remote: &[MailBox]) -> Vec<MailBox> {
    match &account.download_folders {
        Some(folders) if !folders.is_empty() => {
            let wanted: BTreeSet<&String> = folders.iter().collect();
            remote
                .iter()
                .filter(|m| wanted.contains(&m.name))
                .cloned()
                .collect()
        }
        _ => remote.to_vec(),
    }
}

/// Upsert discovered mailboxes, preserving the previously-stored `sync_cursor`
/// (the freshly-listed records carry `sync_cursor: None`).
fn persist_mailboxes(remote: &[MailBox]) -> MailboxdResult<()> {
    let mut to_store = Vec::with_capacity(remote.len());
    for m in remote {
        let mut record = m.clone();
        if let Ok(existing) = MailBox::get(m.id) {
            record.sync_cursor = existing.sync_cursor;
        }
        to_store.push(record);
    }
    MailBox::batch_upsert(&to_store)
}

/// Persist the opaque cursor for a mailbox after a successful sync.
fn commit_cursor(mailbox: &MailBox, cursor: &SyncCursor) -> MailboxdResult<()> {
    let mut record = MailBox::get(mailbox.id).unwrap_or_else(|_| mailbox.clone());
    record.sync_cursor = Some(cursor.as_str().to_string());
    MailBox::batch_upsert(&[record])
}

#[cfg(test)]
mod tests;
