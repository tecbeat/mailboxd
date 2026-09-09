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

//! Creation and extraction of the `tar` + `zstd` backup archive.

use crate::backup::{
    BackupLayout, BackupManifest, ARCHIVE_INDEX_ATTACHMENT, ARCHIVE_INDEX_MAIL, ARCHIVE_MANIFEST,
    ARCHIVE_MEMDB, ARCHIVE_STORAGE, ARCHIVE_STORAGE_VERSION, BACKUP_FORMAT_VERSION, ZSTD_LEVEL,
};
use crate::error::{code::ErrorCode, MailboxdError, MailboxdResult};
use crate::migrate::{read_storage_version, CURRENT_STORAGE_VERSION};
use crate::raise_error;
use crate::settings::cli::SETTINGS;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

fn ierr(ctx: &str, e: impl std::fmt::Display) -> MailboxdError {
    raise_error!(format!("backup: {ctx}: {e}"), ErrorCode::InternalError)
}

impl BackupManifest {
    fn for_layout(layout: &BackupLayout, components: Vec<String>) -> Self {
        Self {
            format_version: BACKUP_FORMAT_VERSION,
            storage_version: read_storage_version(&layout.root_dir)
                .unwrap_or(CURRENT_STORAGE_VERSION),
            mailboxd_version: crate::mailboxd_version!().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            encrypt_password_set: SETTINGS.mailboxd_encrypt_password.is_some()
                || SETTINGS.mailboxd_encrypt_password_file.is_some(),
            components,
        }
    }
}

/// Write a full backup of `layout` to `writer` as a zstd-compressed tar stream.
///
/// The `manifest.json` entry is written first so that [`read_manifest`] can
/// validate an archive without scanning it end to end. Returns the manifest
/// that was embedded.
pub fn create_archive<W: Write>(layout: &BackupLayout, writer: W) -> MailboxdResult<BackupManifest> {
    // Determine which components are present up front so the manifest, written
    // as the first entry, is accurate.
    let mut components = Vec::new();
    if layout.memdb_dir.is_dir() {
        components.push(ARCHIVE_MEMDB.to_string());
    }
    if layout.storage_dir.is_dir() {
        components.push(ARCHIVE_STORAGE.to_string());
    }
    if layout.envelope_dir.is_dir() {
        components.push(ARCHIVE_INDEX_MAIL.to_string());
    }
    if layout.attachment_dir.is_dir() {
        components.push(ARCHIVE_INDEX_ATTACHMENT.to_string());
    }

    let manifest = BackupManifest::for_layout(layout, components);

    let encoder = zstd::stream::write::Encoder::new(writer, ZSTD_LEVEL)
        .map_err(|e| ierr("init zstd encoder", e))?;
    let mut builder = tar::Builder::new(encoder);
    builder.follow_symlinks(false);

    let manifest_bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|e| ierr("serialize manifest", e))?;
    append_bytes(&mut builder, ARCHIVE_MANIFEST, &manifest_bytes)?;

    let version_path = layout.root_dir.join(ARCHIVE_STORAGE_VERSION);
    if version_path.is_file() {
        builder
            .append_path_with_name(&version_path, ARCHIVE_STORAGE_VERSION)
            .map_err(|e| ierr("append STORAGE_VERSION", e))?;
    }

    append_dir_if_present(&mut builder, &layout.memdb_dir, ARCHIVE_MEMDB)?;
    append_dir_if_present(&mut builder, &layout.storage_dir, ARCHIVE_STORAGE)?;
    append_dir_if_present(&mut builder, &layout.envelope_dir, ARCHIVE_INDEX_MAIL)?;
    append_dir_if_present(&mut builder, &layout.attachment_dir, ARCHIVE_INDEX_ATTACHMENT)?;

    // Finish the tar stream, finalize the zstd frame, then flush the writer so
    // a buffered writer (e.g. `BufWriter<File>`) is fully drained before the
    // caller renames the file into place.
    let encoder = builder.into_inner().map_err(|e| ierr("finish tar stream", e))?;
    let mut writer = encoder.finish().map_err(|e| ierr("finish zstd stream", e))?;
    writer.flush().map_err(|e| ierr("flush backup writer", e))?;

    Ok(manifest)
}

fn append_bytes<W: Write>(
    builder: &mut tar::Builder<W>,
    name: &str,
    data: &[u8],
) -> MailboxdResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o600);
    header.set_mtime(chrono::Utc::now().timestamp().max(0) as u64);
    header.set_cksum();
    builder
        .append_data(&mut header, name, data)
        .map_err(|e| ierr(&format!("append entry '{name}'"), e))
}

fn append_dir_if_present<W: Write>(
    builder: &mut tar::Builder<W>,
    src: &Path,
    prefix: &str,
) -> MailboxdResult<()> {
    if src.is_dir() {
        builder
            .append_dir_all(prefix, src)
            .map_err(|e| ierr(&format!("archive directory '{prefix}'"), e))?;
    }
    Ok(())
}

/// Read and parse the `manifest.json` entry from a backup archive.
pub fn read_manifest<R: Read>(reader: R) -> MailboxdResult<BackupManifest> {
    let decoder =
        zstd::stream::read::Decoder::new(reader).map_err(|e| ierr("init zstd decoder", e))?;
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries().map_err(|e| ierr("read archive entries", e))? {
        let mut entry = entry.map_err(|e| ierr("read archive entry", e))?;
        let path = entry.path().map_err(|e| ierr("read entry path", e))?;
        if path.as_os_str() == ARCHIVE_MANIFEST {
            let mut buf = String::new();
            entry
                .read_to_string(&mut buf)
                .map_err(|e| ierr("read manifest", e))?;
            return serde_json::from_str(&buf).map_err(|e| {
                raise_error!(
                    format!("invalid backup manifest: {e}"),
                    ErrorCode::InvalidParameter
                )
            });
        }
    }
    Err(raise_error!(
        "backup archive is missing manifest.json".into(),
        ErrorCode::InvalidParameter
    ))
}

/// Extract a backup archive into the directories described by `layout`.
///
/// Only known entry prefixes are extracted; unknown entries are ignored and any
/// entry whose path escapes its component root (`..`, absolute paths) is
/// rejected outright.
pub fn extract_archive<R: Read>(reader: R, layout: &BackupLayout) -> MailboxdResult<()> {
    let decoder =
        zstd::stream::read::Decoder::new(reader).map_err(|e| ierr("init zstd decoder", e))?;
    let mut archive = tar::Archive::new(decoder);
    archive.set_preserve_permissions(false);
    archive.set_preserve_mtime(false);

    for entry in archive.entries().map_err(|e| ierr("read archive entries", e))? {
        let mut entry = entry.map_err(|e| ierr("read archive entry", e))?;
        let entry_path = entry
            .path()
            .map_err(|e| ierr("read entry path", e))?
            .into_owned();
        let Some(target) = map_entry_path(&entry_path, layout)? else {
            continue;
        };
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ierr("create parent directory", e))?;
        }
        entry
            .unpack(&target)
            .map_err(|e| ierr(&format!("unpack '{}'", entry_path.display()), e))?;
    }
    Ok(())
}

/// Map an archive entry path to its on-disk target under `layout`, rejecting
/// path traversal. Returns `Ok(None)` for metadata (`manifest.json`) and for
/// unknown prefixes that should simply be skipped.
fn map_entry_path(entry_path: &Path, layout: &BackupLayout) -> MailboxdResult<Option<PathBuf>> {
    let mut comps: Vec<String> = Vec::new();
    for c in entry_path.components() {
        match c {
            Component::Normal(s) => comps.push(s.to_string_lossy().into_owned()),
            Component::CurDir => {}
            _ => {
                return Err(raise_error!(
                    format!("unsafe path in backup archive: {}", entry_path.display()),
                    ErrorCode::InvalidParameter
                ))
            }
        }
    }
    if comps.is_empty() {
        return Ok(None);
    }
    if comps.len() == 1 && comps[0] == ARCHIVE_MANIFEST {
        return Ok(None);
    }
    if comps.len() == 1 && comps[0] == ARCHIVE_STORAGE_VERSION {
        return Ok(Some(layout.root_dir.join(ARCHIVE_STORAGE_VERSION)));
    }

    let joined = comps.join("/");
    let mappings: [(&str, &PathBuf); 4] = [
        (ARCHIVE_MEMDB, &layout.memdb_dir),
        (ARCHIVE_STORAGE, &layout.storage_dir),
        (ARCHIVE_INDEX_MAIL, &layout.envelope_dir),
        (ARCHIVE_INDEX_ATTACHMENT, &layout.attachment_dir),
    ];
    for (prefix, base) in mappings {
        if let Some(rest) = strip_prefix_path(&joined, prefix) {
            let mut target = base.clone();
            for part in rest.split('/').filter(|p| !p.is_empty()) {
                target.push(part);
            }
            return Ok(Some(target));
        }
    }
    Ok(None)
}

fn strip_prefix_path(joined: &str, prefix: &str) -> Option<String> {
    if joined == prefix {
        return Some(String::new());
    }
    joined
        .strip_prefix(&format!("{prefix}/"))
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch() -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("backup-tests")
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

    #[test]
    fn round_trip_preserves_all_components() {
        let src = scratch();
        let layout = sample_layout(&src);
        write(&layout.root_dir.join("STORAGE_VERSION"), b"2\n");
        write(&layout.memdb_dir.join("snapshot.json"), b"{\"k\":1}");
        write(&layout.memdb_dir.join("wal.jsonl"), b"{\"op\":\"put\"}\n");
        write(
            &layout.storage_dir.join("blobs").join("0001.seg"),
            b"blob-payload",
        );
        write(&layout.envelope_dir.join("meta.json"), b"envelope-meta");
        write(&layout.attachment_dir.join("meta.json"), b"attachment-meta");

        let mut buf: Vec<u8> = Vec::new();
        let manifest = create_archive(&layout, &mut buf).unwrap();
        assert_eq!(manifest.storage_version, 2);
        assert!(manifest.components.contains(&ARCHIVE_MEMDB.to_string()));
        assert!(manifest.components.contains(&ARCHIVE_STORAGE.to_string()));

        let read_back = read_manifest(buf.as_slice()).unwrap();
        assert_eq!(read_back.format_version, BACKUP_FORMAT_VERSION);
        assert_eq!(read_back.storage_version, 2);

        let dst_base = scratch();
        let dst = sample_layout(&dst_base);
        extract_archive(buf.as_slice(), &dst).unwrap();

        assert_eq!(fs::read(dst.root_dir.join("STORAGE_VERSION")).unwrap(), b"2\n");
        assert_eq!(
            fs::read(dst.memdb_dir.join("snapshot.json")).unwrap(),
            b"{\"k\":1}"
        );
        assert_eq!(
            fs::read(dst.memdb_dir.join("wal.jsonl")).unwrap(),
            b"{\"op\":\"put\"}\n"
        );
        assert_eq!(
            fs::read(dst.storage_dir.join("blobs").join("0001.seg")).unwrap(),
            b"blob-payload"
        );
        assert_eq!(
            fs::read(dst.envelope_dir.join("meta.json")).unwrap(),
            b"envelope-meta"
        );
        assert_eq!(
            fs::read(dst.attachment_dir.join("meta.json")).unwrap(),
            b"attachment-meta"
        );

        fs::remove_dir_all(&src).ok();
        fs::remove_dir_all(&dst_base).ok();
    }

    #[test]
    fn missing_manifest_is_rejected() {
        let mut buf: Vec<u8> = Vec::new();
        {
            let encoder = zstd::stream::write::Encoder::new(&mut buf, 1).unwrap();
            let mut builder = tar::Builder::new(encoder);
            let data = b"hello";
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o600);
            header.set_cksum();
            builder.append_data(&mut header, "random.txt", &data[..]).unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }
        assert!(read_manifest(buf.as_slice()).is_err());
    }

    #[test]
    fn map_entry_path_rejects_traversal_and_maps_known_prefixes() {
        let base = scratch();
        let layout = sample_layout(&base);

        assert!(map_entry_path(Path::new("../escape.txt"), &layout).is_err());
        assert!(map_entry_path(Path::new("memdb/../../escape.txt"), &layout).is_err());

        assert!(map_entry_path(Path::new("manifest.json"), &layout)
            .unwrap()
            .is_none());
        assert!(map_entry_path(Path::new("unknown-top/file"), &layout)
            .unwrap()
            .is_none());

        assert_eq!(
            map_entry_path(Path::new("memdb/wal.jsonl"), &layout)
                .unwrap()
                .unwrap(),
            layout.memdb_dir.join("wal.jsonl")
        );
        assert_eq!(
            map_entry_path(Path::new("mailboxd-indices/mail_metadata/meta.json"), &layout)
                .unwrap()
                .unwrap(),
            layout.envelope_dir.join("meta.json")
        );
        assert_eq!(
            map_entry_path(Path::new("STORAGE_VERSION"), &layout)
                .unwrap()
                .unwrap(),
            layout.root_dir.join("STORAGE_VERSION")
        );

        fs::remove_dir_all(&base).ok();
    }
}
