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

//! Per-account retention: archived mail older than the account's
//! `retention_days` is deleted by a periodic sweep and skipped on ingest.
//!
//! A message's age is its *effective date*, the older of `date` (Date header)
//! and `internal_date` (IMAP INTERNALDATE or import time), with 0 meaning
//! unknown. The older one is used because imports and mailbox migrations
//! often rewrite INTERNALDATE to the import time.

pub mod task;

use std::collections::HashMap;

use tracing::{info, warn};

use crate::{
    account::migration::AccountModel,
    error::MailboxdResult,
    ext::event_bus::{emit, Event},
    store::tantivy::{attachment::ATTACHMENT_MANAGER, envelope::ENVELOPE_MANAGER},
    utc_now,
};

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// Envelopes deleted per round of a sweep.
const PURGE_BATCH: usize = 1000;

/// Epoch-millisecond cutoff of the account's retention window, or `None`
/// when retention is off.
pub fn cutoff_ms(account: &AccountModel) -> Option<i64> {
    window_days(account).map(|days| cutoff_for(days, utc_now!()))
}

fn window_days(account: &AccountModel) -> Option<u64> {
    account.retention_days.filter(|&days| days > 0)
}

fn cutoff_for(days: u64, now_ms: i64) -> i64 {
    let days = i64::try_from(days).unwrap_or(i64::MAX);
    now_ms.saturating_sub(days.saturating_mul(DAY_MS))
}

/// The older of the two timestamps, ignoring unknown (0) ones.
pub fn effective_date_ms(date: i64, internal_date: i64) -> Option<i64> {
    match (date, internal_date) {
        (0, 0) => None,
        (0, known) | (known, 0) => Some(known),
        (a, b) => Some(a.min(b)),
    }
}

/// Whether a message is older than `cutoff_ms`. Mail without any known
/// timestamp is never expired.
pub fn is_expired(date: i64, internal_date: i64, cutoff_ms: i64) -> bool {
    effective_date_ms(date, internal_date).is_some_and(|effective| effective < cutoff_ms)
}

/// IMAP `UID SEARCH` criteria `base` (e.g. `ALL` or `SINCE 01-Jan-2024`),
/// narrowed to the retention window when it is on, so a full download does
/// not fetch mail the sweep deletes again.
///
/// `SINCE` compares the INTERNALDATE by day, so mail of the cutoff day is
/// still fetched; ingest checks it against the exact window.
pub fn imap_search_criteria(account: &AccountModel, base: &str) -> String {
    match cutoff_ms(account).and_then(chrono::DateTime::from_timestamp_millis) {
        Some(cutoff) => {
            let floor = format!("SINCE {}", cutoff.format("%d-%b-%Y"));
            if base == "ALL" {
                floor
            } else {
                format!("{base} {floor}")
            }
        }
        None => base.to_string(),
    }
}

/// Delete the account's mail that is older than its retention window. Returns
/// the audit event to record, or `None` when retention is off or nothing was
/// deleted.
pub async fn purge_account(account: &AccountModel) -> Option<Event> {
    let days = window_days(account)?;
    let cutoff = cutoff_for(days, utc_now!());
    let mut deleted = 0u64;
    let result = purge_before(account.id, cutoff, PURGE_BATCH, &mut deleted).await;
    purge_event(account, days, deleted, result)
}

async fn purge_before(
    account_id: u64,
    cutoff_ms: i64,
    batch: usize,
    deleted: &mut u64,
) -> MailboxdResult<()> {
    loop {
        let ids = ENVELOPE_MANAGER.retention_expired_ids(account_id, cutoff_ms, batch)?;
        if ids.is_empty() {
            return Ok(());
        }
        // The regular delete path keeps blobs shared with other mail and the
        // attachment index in step.
        let request = HashMap::from([(account_id, ids)]);
        let round = ENVELOPE_MANAGER
            .delete_envelopes_multi_account(request.clone())
            .await?;
        ATTACHMENT_MANAGER
            .delete_attachments_multi_account(request)
            .await?;
        *deleted += round;
        if round == 0 {
            // Deleted concurrently; the next sweep picks up anything left.
            return Ok(());
        }
    }
}

fn purge_event(
    account: &AccountModel,
    window_days: u64,
    deleted: u64,
    result: MailboxdResult<()>,
) -> Option<Event> {
    let success = match result {
        Ok(()) if deleted == 0 => return None,
        Ok(()) => {
            info!(
                "Retention: deleted {deleted} message(s) of account {} older than {window_days} days",
                account.id
            );
            true
        }
        Err(e) => {
            warn!(
                "Retention: sweep of account {} failed after {deleted} deletion(s): {e:?}",
                account.id
            );
            false
        }
    };
    Some(Event::RetentionPurged {
        account: account.email.clone(),
        deleted,
        window_days,
        success,
    })
}

/// Run the retention of every account that is not being deleted and return
/// the audit events.
pub async fn sweep(accounts: &[AccountModel]) -> Vec<Event> {
    let mut events = Vec::new();
    for account in accounts.iter().filter(|account| !account.deleting) {
        events.extend(purge_account(account).await);
    }
    events
}

/// One scheduled sweep over all accounts.
pub async fn run_sweep() -> MailboxdResult<()> {
    for event in sweep(&AccountModel::list_all()?).await {
        emit(event);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
