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

//! Storage-layout migrations for mailboxd. [`run_auto_migrate`] is the
//! unattended engine shared by the server startup path and the admin
//! `--auto-migrate` entry point; the interactive `handle_*` wrappers add
//! operator prompts for the admin menu.

use std::path::PathBuf;

use mailboxd_core::{
    error::{code::ErrorCode, MailboxdResult},
    migrate::adopt_bichon_layout,
    raise_error,
};

pub mod legacy;
pub mod meta;
pub mod migrate_store_v2;
pub mod migrate_v037;
pub mod migrate_v1;

/// Directories the migration operates on (`MAILBOXD_*_DIR`); `index_dir` and
/// `data_dir` fall back to `root_dir` when unset.
#[derive(Debug, Clone)]
pub struct AutoMigratePaths {
    pub root_dir: PathBuf,
    pub index_dir: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
}

/// What [`run_auto_migrate`] did, so callers can report it in their own voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoMigrateOutcome {
    AlreadyCurrent,
    MigratedV1 { emails: u64, attachments: u64 },
    MigratedV037 { migrated: usize, skipped: usize },
}

/// Migrate a data volume up to v2, unattended: adopt a Bichon volume in place,
/// then convert a v1.x fjall or legacy v0.3.7 store. Idempotent, non-destructive
/// and a no-op on fresh or already-v2 volumes; returns an error on failure so
/// the caller can refuse to run against a half-converted volume.
pub fn run_auto_migrate(paths: &AutoMigratePaths) -> MailboxdResult<AutoMigrateOutcome> {
    let root_dir = paths.root_dir.as_path();
    let index_base = paths
        .index_dir
        .clone()
        .unwrap_or_else(|| paths.root_dir.clone());
    let data_base = paths
        .data_dir
        .clone()
        .unwrap_or_else(|| paths.root_dir.clone());

    // 1. Adopt an existing Bichon volume in place.
    adopt_bichon_layout(&index_base, &data_base).map_err(|error| {
        raise_error!(
            format!("failed to adopt Bichon data layout: {error:#?}"),
            ErrorCode::InternalError
        )
    })?;

    // 2. v1.x fjall → v2 mailboxd-blob.
    if let Some(outcome) = migrate_v1::migrate_v1_to_v2(root_dir, &data_base, 1000)? {
        return Ok(AutoMigrateOutcome::MigratedV1 {
            emails: outcome.emails,
            attachments: outcome.attachments,
        });
    }

    // 3. legacy v0.3.7 Tantivy → v2.
    let v037_paths = migrate_v037::V037Paths::derive(
        root_dir,
        paths.index_dir.as_deref(),
        paths.data_dir.as_deref(),
    );
    match migrate_v037::migrate_v037_to_v2(&v037_paths, 3000)? {
        Some(outcome) => Ok(AutoMigrateOutcome::MigratedV037 {
            migrated: outcome.migrated,
            skipped: outcome.skipped,
        }),
        None => Ok(AutoMigrateOutcome::AlreadyCurrent),
    }
}
