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

//! Instance backup and restore (issue #42).
//!
//! A backup is a single `tar` archive wrapped in `zstd` that captures the full
//! on-disk state of an instance: the `memdb` database, the blob store
//! (`mailboxd-storage`), the Tantivy indices (`mailboxd-indices`) and the
//! `STORAGE_VERSION` marker, alongside a small [`BackupManifest`].
//!
//! The `memdb` secrets are captured in their encrypted form, so an archive is
//! only usable together with the matching `MAILBOXD_ENCRYPT_PASSWORD`.
//!
//! Restore is applied *before* any database or index handle is opened: an
//! uploaded archive is staged under `<root>/restore-pending/` with a marker
//! file, the process restarts, and [`restore::apply_pending_restore`] unpacks
//! it on the next boot.

use crate::settings::dir::DATA_DIR_MANAGER;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod archive;
pub mod ops;
pub mod restore;
pub mod task;

/// Version of the backup archive layout itself (manifest + entry names).
/// Bumped only when the archive structure changes in an incompatible way.
pub const BACKUP_FORMAT_VERSION: u32 = 1;

/// zstd compression level for the archive wrapper. Kept low because the blob
/// store and Tantivy segments on disk are already zstd-compressed, so a higher
/// level would burn CPU for almost no size benefit.
pub(crate) const ZSTD_LEVEL: i32 = 3;

// Canonical entry names inside the archive. Decoupled from the (relocatable)
// on-disk directories so a backup taken with `MAILBOXD_INDEX_DIR` /
// `MAILBOXD_DATA_DIR` set can still be restored onto a differently-laid-out
// instance.
pub(crate) const ARCHIVE_MANIFEST: &str = "manifest.json";
pub(crate) const ARCHIVE_STORAGE_VERSION: &str = "STORAGE_VERSION";
pub(crate) const ARCHIVE_MEMDB: &str = "memdb";
pub(crate) const ARCHIVE_STORAGE: &str = "mailboxd-storage";
pub(crate) const ARCHIVE_INDEX_MAIL: &str = "mailboxd-indices/mail_metadata";
pub(crate) const ARCHIVE_INDEX_ATTACHMENT: &str = "mailboxd-indices/attachment_metadata";

/// Filesystem locations of the components captured in a backup. Passed
/// explicitly (rather than read from globals) so the archive/extract logic is
/// unit-testable against scratch directories.
#[derive(Clone, Debug)]
pub struct BackupLayout {
    pub root_dir: PathBuf,
    pub memdb_dir: PathBuf,
    pub storage_dir: PathBuf,
    pub envelope_dir: PathBuf,
    pub attachment_dir: PathBuf,
}

impl BackupLayout {
    /// Build the layout from the running instance's [`DATA_DIR_MANAGER`].
    pub fn from_data_dirs() -> Self {
        Self {
            root_dir: DATA_DIR_MANAGER.root_dir.clone(),
            memdb_dir: DATA_DIR_MANAGER.memdb_dir.clone(),
            storage_dir: DATA_DIR_MANAGER.storage_dir.clone(),
            envelope_dir: DATA_DIR_MANAGER.envelope_dir.clone(),
            attachment_dir: DATA_DIR_MANAGER.attachment_dir.clone(),
        }
    }
}

/// Metadata written as `manifest.json` at the root of every backup archive.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupManifest {
    /// [`BACKUP_FORMAT_VERSION`] at creation time.
    pub format_version: u32,
    /// Storage layout version of the captured data.
    pub storage_version: u32,
    /// mailboxd version that produced the archive (informational).
    pub mailboxd_version: String,
    /// RFC 3339 UTC timestamp of creation.
    pub created_at: String,
    /// Whether the source instance had an encryption password configured.
    /// A restore of encrypted secrets is only usable with the same password.
    pub encrypt_password_set: bool,
    /// Canonical entry prefixes present in the archive.
    pub components: Vec<String>,
}

/// Read-only view of the backup configuration and current scheduled-backup
/// state, surfaced to the admin UI.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct BackupConfig {
    /// True when a backup directory and a non-empty cron schedule are set.
    pub scheduled_enabled: bool,
    /// Configured `MAILBOXD_BACKUP_DIR`, if any.
    pub backup_dir: Option<String>,
    /// Configured `MAILBOXD_BACKUP_SCHEDULE`, if non-empty.
    pub schedule: Option<String>,
    /// Configured `MAILBOXD_BACKUP_RETENTION` (0 keeps all).
    pub retention: u64,
    /// RFC 3339 UTC timestamp of the newest scheduled archive on disk, if any.
    pub last_backup_at: Option<String>,
    /// Number of scheduled archives currently present in the backup directory.
    pub scheduled_count: u64,
}
