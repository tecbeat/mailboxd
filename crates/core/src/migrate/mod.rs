use std::path::{Path, PathBuf};

use crate::settings::cli::SETTINGS;
use crate::settings::dir::{INDICES, STORAGE};

/// Current storage layout version.
/// - 1: fjall-based blob storage (post v0.3.7 migration)
/// - 2: mailboxd-blob based storage
pub const CURRENT_STORAGE_VERSION: u32 = 2;

const VERSION_FILE: &str = "STORAGE_VERSION";

/// Top-level storage directory names used by upstream Bichon. mailboxd renamed
/// only these two directories; every other on-disk artifact (`memdb`,
/// `STORAGE_VERSION`, encrypted secrets, blob segments and Tantivy indices) is
/// byte-compatible between the two projects at storage layout v2.
const BICHON_INDICES: &str = "bichon-indices";
const BICHON_STORAGE: &str = "bichon-storage";

/// Adopt an existing Bichon 2.x data volume in place by renaming its top-level
/// storage directories to the mailboxd names.
///
/// This runs before any directory is created, opened, or version-checked, so a
/// Bichon installation can be migrated by swapping the image and the
/// `BICHON_*` → `MAILBOXD_*` environment variables while keeping the same data
/// volume and encryption password. Idempotent: if the mailboxd-named directory
/// already exists, or no Bichon directory is present, it does nothing.
pub fn migrate_bichon_layout() -> std::io::Result<()> {
    let root_dir = PathBuf::from(&SETTINGS.mailboxd_root_dir);

    let index_base = SETTINGS
        .mailboxd_index_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root_dir.clone());
    adopt_legacy_dir(&index_base.join(BICHON_INDICES), &index_base.join(INDICES))?;

    let data_base = SETTINGS
        .mailboxd_data_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root_dir.clone());
    adopt_legacy_dir(&data_base.join(BICHON_STORAGE), &data_base.join(STORAGE))?;

    Ok(())
}

/// Move `legacy` to `target` if `target` does not yet exist and `legacy` is a
/// directory. Prefers an atomic rename and falls back to a recursive copy when
/// the two paths live on different filesystems.
fn adopt_legacy_dir(legacy: &Path, target: &Path) -> std::io::Result<()> {
    if target.exists() || !legacy.is_dir() {
        return Ok(());
    }

    tracing::info!(
        "Adopting Bichon data directory: {} -> {}",
        legacy.display(),
        target.display()
    );

    match std::fs::rename(legacy, target) {
        Ok(()) => Ok(()),
        Err(rename_err) => {
            tracing::info!(
                "Cross-filesystem move required ({}); copying {} -> {}",
                rename_err,
                legacy.display(),
                target.display()
            );
            copy_dir_recursive(legacy, target)?;
            std::fs::remove_dir_all(legacy)
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

/// Read the storage layout version from `root_dir/STORAGE_VERSION`.
pub fn read_storage_version(root_dir: &Path) -> Option<u32> {
    let content = std::fs::read_to_string(root_dir.join(VERSION_FILE)).ok()?;
    content.trim().parse().ok()
}

/// Write the storage layout version to `root_dir/STORAGE_VERSION`.
pub fn write_storage_version(root_dir: &Path, version: u32) -> std::io::Result<()> {
    std::fs::write(root_dir.join(VERSION_FILE), format!("{}\n", version))
}

pub fn is_tantivy_index_dir(dir: &PathBuf) -> std::io::Result<bool> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(false);
    }

    let tantivy_extensions = [".store", ".term", ".idx", ".fieldnorm", ".pos"];
    let mut match_count = 0;
    let mut has_meta_json = false;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if name == "meta.json" {
            has_meta_json = true;
            continue;
        }

        if tantivy_extensions.iter().any(|ext| name.ends_with(ext)) {
            match_count += 1;
        }
    }

    Ok(has_meta_json && match_count >= 3)
}

/// Check whether the data layout is compatible with the current server.
/// Returns `false` when legacy data (v0.3.7 or v1.x) is detected and migration is required.
pub fn check_data_status() -> std::io::Result<bool> {
    let root_dir = PathBuf::from(&SETTINGS.mailboxd_root_dir);

    // 1. Version file takes precedence
    if let Some(version) = read_storage_version(&root_dir) {
        return Ok(version >= CURRENT_STORAGE_VERSION);
    }

    // 2. No version file — check for existing v1.x-style storage (fjall era)
    let new_data_base = SETTINGS
        .mailboxd_data_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root_dir.clone());
    let new_storage_path = new_data_base.join("mailboxd-storage");

    if is_dir_not_empty(&new_storage_path)? {
        // Existing v1.x install predates version file — mark it as v1
        let _ = write_storage_version(&root_dir, 1);
        return Ok(false); // Needs migration: v1.x → v2.x
    }

    // 3. Check for legacy v0.3.7 Tantivy layout
    let legacy_index_root = SETTINGS
        .mailboxd_index_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root_dir.join("envelope"));
    let legacy_data_root = SETTINGS
        .mailboxd_data_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root_dir.join("eml"));

    let has_legacy_index = is_tantivy_index_dir(&legacy_index_root)?;
    let has_legacy_data = is_tantivy_index_dir(&legacy_data_root)?;

    if has_legacy_index || has_legacy_data {
        Ok(false) // Needs migration
    } else {
        Ok(true) // Fresh install
    }
}

fn is_dir_not_empty(path: &PathBuf) -> std::io::Result<bool> {
    if !path.exists() || !path.is_dir() {
        return Ok(false);
    }
    let mut entries = std::fs::read_dir(path)?;
    Ok(entries.next().is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Project-local, gitignored scratch directory (`crates/*/target` is
    /// ignored). Avoids the system temp directory entirely.
    fn scratch_dir() -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("migrate-tests")
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    #[test]
    fn adopts_legacy_dir_by_rename_preserving_nested_content() {
        let base = scratch_dir();
        let legacy = base.join(BICHON_STORAGE);
        let target = base.join(STORAGE);
        write_file(&legacy.join("blobs").join("0001.seg"), "payload");

        adopt_legacy_dir(&legacy, &target).unwrap();

        assert!(!legacy.exists(), "legacy directory should be gone");
        assert_eq!(
            std::fs::read_to_string(target.join("blobs").join("0001.seg")).unwrap(),
            "payload"
        );

        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn skips_when_target_already_exists() {
        let base = scratch_dir();
        let legacy = base.join(BICHON_STORAGE);
        let target = base.join(STORAGE);
        write_file(&legacy.join("legacy.txt"), "old");
        write_file(&target.join("current.txt"), "new");

        adopt_legacy_dir(&legacy, &target).unwrap();

        assert!(legacy.exists(), "legacy must be left untouched");
        assert!(target.join("current.txt").exists());
        assert!(!target.join("legacy.txt").exists());

        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn no_op_when_legacy_absent() {
        let base = scratch_dir();
        let legacy = base.join(BICHON_STORAGE);
        let target = base.join(STORAGE);

        adopt_legacy_dir(&legacy, &target).unwrap();

        assert!(!target.exists());

        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn copy_dir_recursive_copies_nested_tree() {
        let base = scratch_dir();
        let src = base.join("src");
        let dst = base.join("dst");
        write_file(&src.join("a.txt"), "a");
        write_file(&src.join("nested").join("b.txt"), "b");

        copy_dir_recursive(&src, &dst).unwrap();

        assert_eq!(std::fs::read_to_string(dst.join("a.txt")).unwrap(), "a");
        assert_eq!(
            std::fs::read_to_string(dst.join("nested").join("b.txt")).unwrap(),
            "b"
        );

        std::fs::remove_dir_all(&base).unwrap();
    }
}
