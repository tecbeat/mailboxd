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

//! Staged restore-on-restart.
//!
//! Restoring a backup replaces the entire on-disk state, so it cannot be done
//! while the database and indices are open. Instead an uploaded archive is
//! validated and *staged* under `<root>/restore-pending/`, guarded by a
//! `RESTORE_PENDING` marker written last. The process then restarts (the
//! container's `restart: unless-stopped` policy relaunches it) and
//! [`apply_pending_restore`] unpacks the archive on the next boot, *before* any
//! database or index handle is opened.

use crate::backup::archive::{extract_archive, read_manifest};
use crate::backup::{BackupLayout, BACKUP_FORMAT_VERSION};
use crate::error::{code::ErrorCode, MailboxdResult};
use crate::raise_error;
use crate::settings::dir::DATA_DIR_MANAGER;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

const STAGING_DIR: &str = "restore-pending";
const STAGED_ARCHIVE: &str = "archive.tar.zst";
const MARKER_FILE: &str = "RESTORE_PENDING";

/// Whether a restore has been staged and is waiting to be applied on the next
/// boot.
pub fn is_restore_pending() -> bool {
    marker_in(&DATA_DIR_MANAGER.root_dir).is_file()
}

/// Validate and stage an uploaded backup archive for restore-on-restart.
///
/// The caller is responsible for restarting the process after this returns
/// `Ok`; the staged archive is applied on the next boot.
pub fn stage_restore_from_file(uploaded: &Path) -> MailboxdResult<()> {
    stage_from_file_at(&DATA_DIR_MANAGER.root_dir, uploaded)
}

/// Apply a staged restore, if one is pending. Must be called on boot before any
/// database, blob, or index handle is opened. No-op when nothing is staged.
pub fn apply_pending_restore() -> MailboxdResult<()> {
    apply_pending_restore_at(&DATA_DIR_MANAGER.root_dir, &BackupLayout::from_data_dirs()).map(|_| ())
}

fn staging_dir_in(root: &Path) -> PathBuf {
    root.join(STAGING_DIR)
}

fn staged_archive_in(root: &Path) -> PathBuf {
    staging_dir_in(root).join(STAGED_ARCHIVE)
}

fn marker_in(root: &Path) -> PathBuf {
    staging_dir_in(root).join(MARKER_FILE)
}

fn stage_from_file_at(root: &Path, uploaded: &Path) -> MailboxdResult<()> {
    // Validate the upload is a well-formed, compatible backup before staging,
    // so an obviously bad file is rejected while the client is still connected.
    let file = std::fs::File::open(uploaded).map_err(|e| {
        raise_error!(format!("open uploaded backup: {e}"), ErrorCode::InvalidParameter)
    })?;
    let manifest = read_manifest(BufReader::new(file))?;
    if manifest.format_version > BACKUP_FORMAT_VERSION {
        return Err(raise_error!(
            format!(
                "backup archive format version {} is newer than supported ({})",
                manifest.format_version, BACKUP_FORMAT_VERSION
            ),
            ErrorCode::Incompatible
        ));
    }

    let staging = staging_dir_in(root);
    std::fs::create_dir_all(&staging).map_err(|e| {
        raise_error!(format!("create staging directory: {e}"), ErrorCode::InternalError)
    })?;

    // Remove any stale marker first so a crash mid-move never leaves the marker
    // pointing at an incomplete archive.
    remove_path(&marker_in(root))?;
    move_file(uploaded, &staged_archive_in(root))?;

    // Marker written last: its presence means "a complete archive is staged".
    std::fs::write(marker_in(root), chrono::Utc::now().to_rfc3339())
        .map_err(|e| raise_error!(format!("write restore marker: {e}"), ErrorCode::InternalError))?;

    info!(
        "Restore staged from a {} archive created {}; restart required to apply",
        if manifest.encrypt_password_set {
            "password-protected"
        } else {
            "unprotected"
        },
        manifest.created_at
    );
    Ok(())
}

fn apply_pending_restore_at(root: &Path, layout: &BackupLayout) -> MailboxdResult<bool> {
    let marker = marker_in(root);
    if !marker.is_file() {
        return Ok(false);
    }

    let staged = staged_archive_in(root);
    if !staged.is_file() {
        warn!("Restore marker present but no staged archive found; clearing marker");
        clear_staging(root);
        return Ok(false);
    }

    // Validate before touching live data. A corrupt or incompatible staged
    // archive is discarded (rather than crash-looping the boot) and the
    // instance continues with its existing data.
    let manifest = {
        let file = std::fs::File::open(&staged)
            .map_err(|e| raise_error!(format!("open staged backup: {e}"), ErrorCode::InternalError))?;
        match read_manifest(BufReader::new(file)) {
            Ok(manifest) if manifest.format_version <= BACKUP_FORMAT_VERSION => manifest,
            Ok(manifest) => {
                error!(
                    "Staged backup format version {} is newer than supported ({}); discarding",
                    manifest.format_version, BACKUP_FORMAT_VERSION
                );
                clear_staging(root);
                return Ok(false);
            }
            Err(e) => {
                error!("Staged backup is invalid ({e}); discarding");
                clear_staging(root);
                return Ok(false);
            }
        }
    };

    info!(
        "Applying staged restore (archive created {}, storage version {})",
        manifest.created_at, manifest.storage_version
    );

    // Clear the live data first so no stale files survive the restore, then
    // extract. If we crash between these, the marker and staged archive are
    // still present and the restore is simply retried on the next boot.
    clear_existing_data(layout)?;

    let file = std::fs::File::open(&staged)
        .map_err(|e| raise_error!(format!("open staged backup: {e}"), ErrorCode::InternalError))?;
    extract_archive(BufReader::new(file), layout)?;

    // Remove the marker first so a crash while cleaning up staging does not
    // re-trigger the (already applied) restore on the next boot.
    remove_path(&marker)?;
    clear_staging(root);

    info!("Restore applied successfully");
    Ok(true)
}

/// Remove the on-disk data components described by `layout` (ignoring anything
/// that is already absent) ahead of an extract.
fn clear_existing_data(layout: &BackupLayout) -> MailboxdResult<()> {
    remove_path(&layout.memdb_dir)?;
    remove_path(&layout.storage_dir)?;
    remove_path(&layout.envelope_dir)?;
    remove_path(&layout.attachment_dir)?;
    remove_path(&layout.root_dir.join(crate::backup::ARCHIVE_STORAGE_VERSION))?;
    Ok(())
}

fn clear_staging(root: &Path) {
    let staging = staging_dir_in(root);
    if let Err(e) = std::fs::remove_dir_all(&staging) {
        if e.kind() != std::io::ErrorKind::NotFound {
            warn!("Failed to clear restore staging {}: {e}", staging.display());
        }
    }
}

/// Remove a file or directory, treating "not found" as success.
fn remove_path(path: &Path) -> MailboxdResult<()> {
    let result = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    match result {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(raise_error!(
            format!("remove '{}': {e}", path.display()),
            ErrorCode::InternalError
        )),
    }
}

/// Move `src` to `dst`, falling back to copy + delete across filesystems.
fn move_file(src: &Path, dst: &Path) -> MailboxdResult<()> {
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    std::fs::copy(src, dst)
        .map_err(|e| raise_error!(format!("stage backup archive: {e}"), ErrorCode::InternalError))?;
    let _ = std::fs::remove_file(src);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::archive::create_archive;
    use std::fs;
    use std::io::BufWriter;

    fn scratch() -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("restore-tests")
            .join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, content: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn sample_layout(base: &Path) -> BackupLayout {
        BackupLayout {
            root_dir: base.to_path_buf(),
            memdb_dir: base.join("memdb"),
            storage_dir: base.join("mailboxd-storage"),
            envelope_dir: base.join("mailboxd-indices").join("mail_metadata"),
            attachment_dir: base.join("mailboxd-indices").join("attachment_metadata"),
        }
    }

    fn build_archive(src: &BackupLayout, dest_file: &Path) {
        let file = fs::File::create(dest_file).unwrap();
        create_archive(src, BufWriter::new(file)).unwrap();
    }

    #[test]
    fn stage_then_apply_replaces_data() {
        let base = scratch();
        let src = sample_layout(&base.join("src"));
        write(&src.root_dir.join("STORAGE_VERSION"), b"2\n");
        write(&src.memdb_dir.join("wal.jsonl"), b"{\"op\":\"put\"}\n");
        write(&src.storage_dir.join("blobs").join("0001.seg"), b"blob");
        write(&src.envelope_dir.join("meta.json"), b"env");
        write(&src.attachment_dir.join("meta.json"), b"att");

        let uploaded = base.join("uploaded.tar.zst");
        build_archive(&src, &uploaded);

        let dst_root = base.join("dst");
        let dst = sample_layout(&dst_root);
        // Pre-existing stale data that must not survive the restore.
        write(&dst.memdb_dir.join("stale.json"), b"stale");

        assert!(!marker_in(&dst_root).is_file());
        stage_from_file_at(&dst_root, &uploaded).unwrap();
        assert!(marker_in(&dst_root).is_file());
        assert!(!uploaded.exists(), "uploaded file should be moved into staging");

        let applied = apply_pending_restore_at(&dst_root, &dst).unwrap();
        assert!(applied);
        assert!(!marker_in(&dst_root).is_file());
        assert!(!staging_dir_in(&dst_root).exists());

        assert_eq!(fs::read(dst.root_dir.join("STORAGE_VERSION")).unwrap(), b"2\n");
        assert_eq!(
            fs::read(dst.memdb_dir.join("wal.jsonl")).unwrap(),
            b"{\"op\":\"put\"}\n"
        );
        assert!(
            !dst.memdb_dir.join("stale.json").exists(),
            "stale data must be cleared before restore"
        );
        assert_eq!(
            fs::read(dst.storage_dir.join("blobs").join("0001.seg")).unwrap(),
            b"blob"
        );

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn apply_without_marker_is_noop() {
        let base = scratch();
        let dst = sample_layout(&base.join("dst"));
        assert!(!apply_pending_restore_at(&base.join("dst"), &dst).unwrap());
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn staging_rejects_non_archive_upload() {
        let base = scratch();
        let bogus = base.join("bogus.bin");
        fs::write(&bogus, b"not a valid archive").unwrap();
        assert!(stage_from_file_at(&base.join("root"), &bogus).is_err());
        assert!(!marker_in(&base.join("root")).is_file());
        fs::remove_dir_all(&base).ok();
    }
}
