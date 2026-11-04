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

//! The cached gauges: storage, index and per-account numbers that need
//! index queries or directory scans.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::{Duration, SystemTime},
};

use tracing::warn;

use super::{known_accounts, millis_to_seconds, remember_account, remove_account, METRICS};
use crate::{
    account::{
        migration::{AccountModel, AccountType},
        state::{DownloadState, DownloadStatus},
    },
    archive::imap::mailbox::MailBox,
    backup::ops::newest_scheduled_backup,
    common::periodic::{PeriodicTask, TaskHandle},
    context::MailboxdTask,
    error::{code::ErrorCode, MailboxdResult},
    oauth2::token::OAuth2AccessToken,
    raise_error,
    settings::{
        cli::SETTINGS,
        dir::{available_space, DATA_DIR_MANAGER},
    },
    store::tantivy::{attachment::ATTACHMENT_MANAGER, envelope::ENVELOPE_MANAGER},
    utils::get_total_size,
};

/// Refreshes the cached gauges every `MAILBOXD_METRICS_REFRESH_SECONDS`.
pub struct MetricsRefreshTask;

impl MailboxdTask for MetricsRefreshTask {
    fn start() -> TaskHandle {
        let task = |_: Option<u64>| Box::pin(refresh());
        PeriodicTask::new("metrics-refresh").start(
            task,
            None,
            Duration::from_secs(SETTINGS.mailboxd_metrics_refresh_seconds),
            false,
            true,
        )
    }
}

/// Recompute the cached gauges now. The scans block, so they run on the
/// blocking pool.
pub async fn refresh() -> MailboxdResult<()> {
    tokio::task::spawn_blocking(refresh_blocking)
        .await
        .map_err(|e| raise_error!(format!("metrics refresh: {e}"), ErrorCode::InternalError))?
}

fn refresh_blocking() -> MailboxdResult<()> {
    let accounts = AccountModel::list_all()?;
    refresh_account_counts(&accounts);

    let live: Vec<&AccountModel> = accounts.iter().filter(|a| !a.deleting).collect();
    let live_ids: HashSet<u64> = live.iter().map(|a| a.id).collect();
    for id in known_accounts() {
        if !live_ids.contains(&id) {
            remove_account(id);
        }
    }
    for account in live {
        if let Err(e) = refresh_account(account) {
            warn!(
                account_id = account.id,
                "Failed to refresh account metrics: {e:#?}"
            );
        }
    }

    refresh_totals()?;
    refresh_backup();
    Ok(())
}

fn refresh_account_counts(accounts: &[AccountModel]) {
    let mut counts: HashMap<(&str, &str), i64> = HashMap::new();
    for account in accounts {
        let kind = match account.account_type {
            AccountType::IMAP => "imap",
            AccountType::NoSync => "nosync",
        };
        let enabled = if account.enabled { "true" } else { "false" };
        *counts.entry((kind, enabled)).or_default() += 1;
    }
    let gauge = &METRICS.accounts;
    gauge.reset();
    for kind in ["imap", "nosync"] {
        for enabled in ["true", "false"] {
            gauge
                .with_label_values(&[kind, enabled])
                .set(counts.get(&(kind, enabled)).copied().unwrap_or(0));
        }
    }
}

fn refresh_account(account: &AccountModel) -> MailboxdResult<()> {
    let labels = remember_account(account.id, &account.email);
    let m = &*METRICS;

    let stats = ENVELOPE_MANAGER.get_account_stats(account.id)?;
    m.account_emails
        .with_label_values(&labels)
        .set(stats.total_count as i64);
    m.account_size
        .with_label_values(&labels)
        .set(stats.total_size as i64);
    let attachments = ATTACHMENT_MANAGER.total_attachments(&Some(HashSet::from([account.id])))?;
    m.account_attachments
        .with_label_values(&labels)
        .set(attachments as i64);
    m.account_folders
        .with_label_values(&labels)
        .set(MailBox::list_all(account.id)?.len() as i64);

    if let Some(state) = DownloadState::get(account.id)? {
        let running = state
            .active_session
            .as_ref()
            .is_some_and(|s| s.status == DownloadStatus::Running);
        m.download_running
            .with_label_values(&labels)
            .set(i64::from(running));
        let last_end = |status: DownloadStatus| {
            state
                .history
                .iter()
                .chain(state.active_session.iter())
                .filter(|s| s.status == status)
                .filter_map(|s| s.end_time)
                .max()
        };
        // History is truncated, so only raise what live recording set.
        if let Some(end) = last_end(DownloadStatus::Success) {
            let gauge = m.download_last_success.with_label_values(&labels);
            gauge.set(gauge.get().max(millis_to_seconds(end)));
        }
        if let Some(end) = last_end(DownloadStatus::Failed) {
            let gauge = m.download_last_failure.with_label_values(&labels);
            gauge.set(gauge.get().max(millis_to_seconds(end)));
        }
    }

    if let Some(token) = OAuth2AccessToken::get(account.id)? {
        m.oauth2_token_refreshed
            .with_label_values(&labels)
            .set(millis_to_seconds(token.updated_at));
    }
    Ok(())
}

fn refresh_totals() -> MailboxdResult<()> {
    let m = &*METRICS;
    m.emails.set(ENVELOPE_MANAGER.total_emails(&None)? as i64);
    m.attachments
        .set(ATTACHMENT_MANAGER.total_attachments(&None)? as i64);

    let dirs = &*DATA_DIR_MANAGER;
    m.storage_bytes.set(dir_size(&dirs.storage_dir)?);
    m.index_bytes
        .with_label_values(&["envelope"])
        .set(dir_size(&dirs.envelope_dir)?);
    m.index_bytes
        .with_label_values(&["attachment"])
        .set(dir_size(&dirs.attachment_dir)?);

    for (label, dir) in [
        ("storage", &dirs.storage_dir),
        ("index", &dirs.envelope_dir),
    ] {
        set_disk_free(label, dir);
    }
    Ok(())
}

fn dir_size(dir: &std::path::PathBuf) -> MailboxdResult<i64> {
    get_total_size(dir).map(|size| size as i64).map_err(|e| {
        raise_error!(
            format!("size of {}: {e}", dir.display()),
            ErrorCode::InternalError
        )
    })
}

/// `available_space` reports `u64::MAX` when it cannot tell; leave the
/// series out then instead of exposing a bogus value.
fn set_disk_free(label: &str, dir: &Path) {
    let free = available_space(dir);
    let gauge = &METRICS.disk_free;
    if free == u64::MAX {
        let _ = gauge.remove_label_values(&[label]);
    } else {
        gauge.with_label_values(&[label]).set(free as i64);
    }
}

fn refresh_backup() {
    let Some((path, modified)) = newest_scheduled_backup() else {
        return;
    };
    let m = &*METRICS;
    if let Ok(age) = modified.duration_since(SystemTime::UNIX_EPOCH) {
        m.backup_last_success.set(age.as_secs_f64());
    }
    match std::fs::metadata(&path) {
        Ok(meta) => m.backup_size.set(meta.len() as i64),
        Err(e) => warn!("Failed to read size of backup {}: {e}", path.display()),
    }
}
