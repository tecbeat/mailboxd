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

use crate::context::Initialize;
use crate::migrate::{write_storage_version, CURRENT_STORAGE_VERSION};
use crate::settings::cli::SETTINGS;
use crate::{
    error::{code::ErrorCode, MailboxdResult},
    raise_error,
};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const MEMDB_DIR: &str = "memdb";
pub const INDICES: &str = "mailboxd-indices";
const MAIL_METADATA: &str = "mail_metadata";
const ATTACHMENT_METADATA: &str = "attachment_metadata";
pub const STORAGE: &str = "mailboxd-storage";
const TMP_DIR: &str = "tmp";
const LOG_DIR: &str = "logs";

const TLS_CERT: &str = "cert.pem";
const TLS_KEY: &str = "key.pem";

pub static DATA_DIR_MANAGER: LazyLock<DataDirManager> =
    LazyLock::new(|| DataDirManager::new(PathBuf::from(&SETTINGS.mailboxd_root_dir)));

#[derive(Debug)]
pub struct DataDirManager {
    pub root_dir: PathBuf,
    pub memdb_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub tls_cert: PathBuf,
    pub tls_key: PathBuf,
    pub envelope_dir: PathBuf,
    pub attachment_dir: PathBuf,
    pub storage_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl Initialize for DataDirManager {
    async fn initialize() -> MailboxdResult<()> {
        std::fs::create_dir_all(&DATA_DIR_MANAGER.root_dir)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        std::fs::create_dir_all(&DATA_DIR_MANAGER.log_dir)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        std::fs::create_dir_all(&DATA_DIR_MANAGER.temp_dir)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        std::fs::create_dir_all(&DATA_DIR_MANAGER.storage_dir)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        // Write STORAGE_VERSION on fresh install (no existing data)
        let version_path = DATA_DIR_MANAGER.root_dir.join("STORAGE_VERSION");
        if !version_path.exists() && !DATA_DIR_MANAGER.storage_dir.join("blobs").exists() {
            write_storage_version(&DATA_DIR_MANAGER.root_dir, CURRENT_STORAGE_VERSION)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        }

        Ok(())
    }
}

/// Index and storage directories of a data root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDirs {
    /// `<index parent>/mailboxd-indices`, holding both indexes.
    pub index_root: PathBuf,
    pub envelope_dir: PathBuf,
    pub attachment_dir: PathBuf,
    /// `<data parent>/mailboxd-storage`.
    pub storage_dir: PathBuf,
    pub blobs_dir: PathBuf,
}

/// Resolve the index and storage directories from the data root and the
/// optional index and data parent directories, which default to the root.
/// Shared by [`DataDirManager`] and the startup migrations, which run before
/// the settings-based manager.
pub fn resolve_dirs(
    root: &Path,
    index_parent: Option<&Path>,
    data_parent: Option<&Path>,
) -> ResolvedDirs {
    let index_root = index_parent.unwrap_or(root).join(INDICES);
    let storage_dir = data_parent.unwrap_or(root).join(STORAGE);
    ResolvedDirs {
        envelope_dir: index_root.join(MAIL_METADATA),
        attachment_dir: index_root.join(ATTACHMENT_METADATA),
        blobs_dir: storage_dir.join("blobs"),
        index_root,
        storage_dir,
    }
}

/// Free bytes on the file system holding `dir`, or `u64::MAX` when it cannot
/// be determined.
pub fn available_space(dir: &Path) -> u64 {
    use sysinfo::Disks;
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let disks = Disks::new_with_refreshed_list();
    space_on(
        &canonical,
        disks
            .list()
            .iter()
            .map(|disk| (disk.mount_point(), disk.available_space())),
    )
    .unwrap_or(u64::MAX)
}

/// Free bytes of the most specific of `mounts` holding `path`, not merely
/// the first: `/` holds every path.
fn space_on<'a>(path: &Path, mounts: impl Iterator<Item = (&'a Path, u64)>) -> Option<u64> {
    mounts
        .filter(|(mount, _)| path.starts_with(mount))
        .max_by_key(|(mount, _)| mount.components().count())
        .map(|(_, space)| space)
}

impl DataDirManager {
    pub fn new(root_dir: PathBuf) -> Self {
        let dirs = resolve_dirs(
            &root_dir,
            SETTINGS.mailboxd_index_dir.as_deref().map(Path::new),
            SETTINGS.mailboxd_data_dir.as_deref().map(Path::new),
        );

        Self {
            root_dir: root_dir.clone(),
            memdb_dir: root_dir.join(MEMDB_DIR),
            tls_key: root_dir.join(TLS_KEY),
            tls_cert: root_dir.join(TLS_CERT),
            log_dir: root_dir.join(LOG_DIR),
            envelope_dir: dirs.envelope_dir,
            attachment_dir: dirs.attachment_dir,
            temp_dir: root_dir.join(TMP_DIR),
            storage_dir: dirs.storage_dir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirs_default_to_the_root() {
        let dirs = resolve_dirs(Path::new("/data"), None, None);
        assert_eq!(dirs.index_root, Path::new("/data/mailboxd-indices"));
        assert_eq!(
            dirs.envelope_dir,
            Path::new("/data/mailboxd-indices/mail_metadata")
        );
        assert_eq!(
            dirs.attachment_dir,
            Path::new("/data/mailboxd-indices/attachment_metadata")
        );
        assert_eq!(dirs.blobs_dir, Path::new("/data/mailboxd-storage/blobs"));
    }

    #[test]
    fn space_comes_from_the_most_specific_mount() {
        let mounts = [
            (Path::new("/"), 1),
            (Path::new("/data"), 2),
            (Path::new("/data/tmp"), 3),
            (Path::new("/database"), 4),
        ];
        let space = |path: &str| space_on(Path::new(path), mounts.iter().copied());
        assert_eq!(space("/data/tmp/import"), Some(3));
        assert_eq!(space("/data/mailboxd"), Some(2));
        assert_eq!(space("/srv"), Some(1));
        assert_eq!(
            space_on(Path::new("/srv"), mounts[1..].iter().copied()),
            None
        );
    }

    #[test]
    fn index_and_data_parents_are_honoured() {
        let dirs = resolve_dirs(
            Path::new("/data"),
            Some(Path::new("/fast")),
            Some(Path::new("/bulk")),
        );
        assert_eq!(
            dirs.envelope_dir,
            Path::new("/fast/mailboxd-indices/mail_metadata")
        );
        assert_eq!(dirs.storage_dir, Path::new("/bulk/mailboxd-storage"));
        assert_eq!(dirs.blobs_dir, Path::new("/bulk/mailboxd-storage/blobs"));
    }
}
