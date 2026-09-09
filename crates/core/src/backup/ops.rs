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

//! High-level backup operations: creating manual and scheduled archives,
//! evaluating the cron schedule, pruning by retention count, and reporting the
//! current backup configuration to the admin UI.

use crate::backup::archive::create_archive;
use crate::backup::{BackupConfig, BackupLayout};
use crate::database::manager::DB_MANAGER;
use crate::error::{code::ErrorCode, MailboxdResult};
use crate::raise_error;
use crate::settings::cli::SETTINGS;
use crate::settings::dir::DATA_DIR_MANAGER;
use chrono::{DateTime, Local, Utc};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;
use tracing::{info, warn};

/// Prefix and suffix of scheduled archive filenames. Used both when writing new
/// archives and when discovering existing ones for retention pruning.
const BACKUP_PREFIX: &str = "mailboxd-backup-";
const BACKUP_SUFFIX: &str = ".tar.zst";

/// A manual backup written to the temp directory, ready to be streamed to the
/// client and then removed.
pub struct ManualBackup {
    /// Absolute path to the freshly written archive in the temp directory.
    pub path: PathBuf,
    /// Suggested download filename (`mailboxd-backup-<timestamp>.tar.zst`).
    pub filename: String,
}

/// Configured backup directory (`MAILBOXD_BACKUP_DIR`), if set and non-empty.
pub fn backup_dir() -> Option<PathBuf> {
    SETTINGS.mailboxd_backup_dir.as_ref().and_then(|dir| {
        let trimmed = dir.trim();
        (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
    })
}

/// Configured cron schedule (`MAILBOXD_BACKUP_SCHEDULE`), if non-empty.
pub fn schedule_expr() -> Option<String> {
    let trimmed = SETTINGS.mailboxd_backup_schedule.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Scheduled backups run only when both a directory and a schedule are set.
pub fn scheduled_enabled() -> bool {
    backup_dir().is_some() && schedule_expr().is_some()
}

/// Snapshot of the backup configuration and current on-disk state for the UI.
pub fn config_snapshot() -> BackupConfig {
    let dir = backup_dir();
    let (last_backup_at, scheduled_count) = match dir.as_ref() {
        Some(dir) => {
            let backups = scheduled_backups(dir);
            let last = backups
                .last()
                .map(|(_, mtime)| DateTime::<Utc>::from(*mtime).to_rfc3339());
            (last, backups.len() as u64)
        }
        None => (None, 0),
    };

    BackupConfig {
        scheduled_enabled: scheduled_enabled(),
        backup_dir: dir.map(|dir| dir.display().to_string()),
        schedule: schedule_expr(),
        retention: SETTINGS.mailboxd_backup_retention,
        last_backup_at,
        scheduled_count,
    }
}

/// Create a one-off backup in the temp directory for immediate download.
///
/// The buffered writes are flushed first so the archive reflects the latest
/// committed state. The heavy archive work runs on a blocking thread.
pub async fn create_manual_backup() -> MailboxdResult<ManualBackup> {
    DB_MANAGER.flush();

    let filename = backup_filename(Utc::now());
    let temp_dir = DATA_DIR_MANAGER.temp_dir.clone();
    let layout = BackupLayout::from_data_dirs();
    let temp_name = format!("{BACKUP_PREFIX}download-{}{BACKUP_SUFFIX}", uuid::Uuid::new_v4());

    let path = tokio::task::spawn_blocking(move || -> MailboxdResult<PathBuf> {
        std::fs::create_dir_all(&temp_dir).map_err(|e| {
            raise_error!(format!("create temp directory: {e}"), ErrorCode::InternalError)
        })?;
        let target = temp_dir.join(&temp_name);
        let file = std::fs::File::create(&target).map_err(|e| {
            raise_error!(format!("create temp backup file: {e}"), ErrorCode::InternalError)
        })?;
        create_archive(&layout, BufWriter::new(file))?;
        Ok(target)
    })
    .await
    .map_err(|e| raise_error!(format!("backup task join error: {e}"), ErrorCode::InternalError))??;

    Ok(ManualBackup { path, filename })
}

/// Create a scheduled backup in the configured backup directory, then prune old
/// archives down to the configured retention count.
pub async fn create_scheduled_backup() -> MailboxdResult<PathBuf> {
    let dir = backup_dir().ok_or_else(|| {
        raise_error!(
            "MAILBOXD_BACKUP_DIR is not configured".into(),
            ErrorCode::MissingConfiguration
        )
    })?;

    DB_MANAGER.flush();

    let filename = backup_filename(Utc::now());
    let layout = BackupLayout::from_data_dirs();
    let dir_for_task = dir.clone();
    let path = tokio::task::spawn_blocking(move || write_archive_file(&layout, &dir_for_task, &filename))
        .await
        .map_err(|e| {
            raise_error!(format!("backup task join error: {e}"), ErrorCode::InternalError)
        })??;

    prune_retention(&dir, SETTINGS.mailboxd_backup_retention)?;
    Ok(path)
}

/// Run a scheduled backup if the cron schedule indicates one is due.
///
/// `started_at` is used as the baseline when no previous archive exists, so the
/// scheduler does not immediately fire on the first boot of a fresh instance.
pub async fn run_scheduled_if_due(started_at: DateTime<Local>) -> MailboxdResult<()> {
    let (Some(dir), Some(schedule)) = (backup_dir(), schedule_expr()) else {
        return Ok(());
    };

    let baseline = newest_backup_mtime(&dir)
        .map(|mtime| DateTime::<Utc>::from(mtime).with_timezone(&Local))
        .unwrap_or(started_at);

    if !schedule_due(&schedule, baseline) {
        return Ok(());
    }

    info!("Scheduled backup is due; creating archive in {}", dir.display());
    let path = create_scheduled_backup().await?;
    info!("Scheduled backup written to {}", path.display());
    Ok(())
}

/// Remove scheduled archives beyond the newest `keep`. `keep == 0` keeps all.
pub fn prune_retention(dir: &Path, keep: u64) -> MailboxdResult<()> {
    if keep == 0 {
        return Ok(());
    }
    let backups = scheduled_backups(dir); // oldest first
    let keep = keep as usize;
    if backups.len() <= keep {
        return Ok(());
    }

    let remove_count = backups.len() - keep;
    for (path, _) in backups.into_iter().take(remove_count) {
        match std::fs::remove_file(&path) {
            Ok(()) => info!("Pruned old backup {}", path.display()),
            Err(e) => warn!("Failed to prune old backup {}: {e}", path.display()),
        }
    }
    Ok(())
}

/// Write a full archive to `<dir>/<filename>`, via a `.partial` sibling that is
/// renamed into place only after a successful, fully-flushed write.
fn write_archive_file(layout: &BackupLayout, dir: &Path, filename: &str) -> MailboxdResult<PathBuf> {
    std::fs::create_dir_all(dir)
        .map_err(|e| raise_error!(format!("create backup directory: {e}"), ErrorCode::InternalError))?;

    let final_path = dir.join(filename);
    let partial_path = dir.join(format!(".{filename}.partial"));

    let file = std::fs::File::create(&partial_path)
        .map_err(|e| raise_error!(format!("create backup file: {e}"), ErrorCode::InternalError))?;
    create_archive(layout, BufWriter::new(file))?;

    std::fs::rename(&partial_path, &final_path)
        .map_err(|e| raise_error!(format!("publish backup file: {e}"), ErrorCode::InternalError))?;
    Ok(final_path)
}

/// Timestamped archive filename in UTC, e.g. `mailboxd-backup-20260101T020000Z.tar.zst`.
fn backup_filename(now: DateTime<Utc>) -> String {
    format!("{BACKUP_PREFIX}{}{BACKUP_SUFFIX}", now.format("%Y%m%dT%H%M%SZ"))
}

/// Whether a filename looks like a scheduled backup archive.
fn is_backup_file(name: &str) -> bool {
    name.starts_with(BACKUP_PREFIX) && name.ends_with(BACKUP_SUFFIX)
}

/// Scheduled archives in `dir`, oldest first, paired with their modified time.
/// `.partial` files and non-archive entries are ignored.
fn scheduled_backups(dir: &Path) -> Vec<(PathBuf, SystemTime)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !is_backup_file(&name) {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                out.push((entry.path(), mtime));
            }
        }
    }
    out.sort_by_key(|(_, mtime)| *mtime);
    out
}

/// Modified time of the newest scheduled archive in `dir`, if any.
fn newest_backup_mtime(dir: &Path) -> Option<SystemTime> {
    scheduled_backups(dir).last().map(|(_, mtime)| *mtime)
}

/// Whether the cron `schedule_str` has a fire time strictly after `last` that is
/// at or before now. An unparseable expression is logged and treated as "not
/// due" so a bad value disables scheduling rather than crashing the task.
fn schedule_due(schedule_str: &str, last: DateTime<Local>) -> bool {
    let schedule = match cron::Schedule::from_str(schedule_str) {
        Ok(schedule) => schedule,
        Err(e) => {
            warn!("Invalid backup cron expression '{schedule_str}': {e}");
            return false;
        }
    };
    let now = Local::now();
    schedule.after(&last).next().is_some_and(|next| next <= now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread::sleep;
    use std::time::Duration;

    fn scratch() -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("backup-ops-tests")
            .join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn backup_filename_is_recognized_as_backup_file() {
        let name = backup_filename(Utc::now());
        assert!(name.starts_with("mailboxd-backup-"));
        assert!(name.ends_with(".tar.zst"));
        assert!(is_backup_file(&name));
        assert!(!is_backup_file("random.txt"));
        assert!(!is_backup_file(".mailboxd-backup-x.tar.zst.partial"));
    }

    #[test]
    fn schedule_due_fires_for_past_baseline_and_not_for_future() {
        let recent_past = Local::now() - chrono::Duration::seconds(5);
        // Every-second cron with a baseline in the past is due now.
        assert!(schedule_due("* * * * * *", recent_past));
        // Yearly cron (next 1 Jan) with a baseline of now is not yet due.
        assert!(!schedule_due("0 0 0 1 1 *", Local::now()));
        // An invalid expression is treated as "not due".
        assert!(!schedule_due("not a cron", Local::now()));
    }

    #[test]
    fn scheduled_backups_ignores_non_archive_files() {
        let dir = scratch();
        fs::write(dir.join("mailboxd-backup-20260101T000000Z.tar.zst"), b"a").unwrap();
        fs::write(dir.join("notes.txt"), b"b").unwrap();
        fs::write(dir.join(".mailboxd-backup-x.tar.zst.partial"), b"c").unwrap();

        let found = scheduled_backups(&dir);
        assert_eq!(found.len(), 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn prune_retention_keeps_the_newest_archives() {
        let dir = scratch();
        let mut names = Vec::new();
        for i in 0..5 {
            let name = format!("mailboxd-backup-2026010{i}T000000Z.tar.zst");
            fs::write(dir.join(&name), [i as u8]).unwrap();
            names.push(name);
            // Stagger modification times so ordering is unambiguous.
            sleep(Duration::from_millis(20));
        }

        prune_retention(&dir, 2).unwrap();

        let remaining: Vec<_> = scheduled_backups(&dir)
            .into_iter()
            .map(|(path, _)| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(remaining.len(), 2);
        // The two newest (last written) must survive.
        assert!(remaining.contains(&names[3]));
        assert!(remaining.contains(&names[4]));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn prune_retention_zero_keeps_all() {
        let dir = scratch();
        for i in 0..3 {
            let name = format!("mailboxd-backup-2026020{i}T000000Z.tar.zst");
            fs::write(dir.join(&name), [i as u8]).unwrap();
        }

        prune_retention(&dir, 0).unwrap();
        assert_eq!(scheduled_backups(&dir).len(), 3);

        fs::remove_dir_all(&dir).ok();
    }
}
