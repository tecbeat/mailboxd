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

//! Index format version marker.
//!
//! Each index directory carries an `INDEX_FORMAT` file with the schema
//! generation it was built with:
//! - no marker and the directory is a tantivy index → legacy format (1),
//!   a reindex is required
//! - marker >= [`CURRENT_INDEX_FORMAT_VERSION`] → usable
//! - directory missing or empty → fresh install
//!
//! The marker lives inside the index directory so backup and restore, which
//! copy the index directories, carry it along.

use std::path::{Path, PathBuf};

use crate::migrate::is_tantivy_index_dir;
use crate::store::tantivy::fields::F_SUBJECT_TRI;

/// Current index schema generation.
/// - 1 = legacy: language-guessing stemmed word tokenizer, no trigram fields
///   (a missing marker counts as 1)
/// - 2 = deterministic word tokenizer and trigram sibling fields
///
/// Bump this whenever the schema or the analyzers change; older indexes are
/// then rebuilt on the next start.
pub const CURRENT_INDEX_FORMAT_VERSION: u32 = 2;

const INDEX_FORMAT_FILE: &str = "INDEX_FORMAT";

/// Whether an index directory is usable by the current binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFormatStatus {
    /// Directory missing or empty: an index can be created directly.
    Fresh,
    /// Marker matches the current version.
    Current,
    /// A tantivy index without a marker or with an older one: reindex required.
    NeedsUpgrade,
}

/// Write the current index format version to `dir/INDEX_FORMAT`.
pub fn write_index_format_version(dir: &Path) -> std::io::Result<()> {
    std::fs::write(
        dir.join(INDEX_FORMAT_FILE),
        format!("{CURRENT_INDEX_FORMAT_VERSION}\n"),
    )
}

/// Format version of an index directory; `None` without a readable marker.
pub fn read_index_format_version(dir: &Path) -> Option<u32> {
    let content = std::fs::read_to_string(dir.join(INDEX_FORMAT_FILE)).ok()?;
    content.trim().parse().ok()
}

/// Stamp an index whose marker write was lost, e.g. by a crash between
/// creating the index and writing the marker. Only an index whose schema
/// already has the trigram fields is stamped, so a legacy index still goes
/// through the reindex. Returns `true` when a marker was written.
pub fn heal_index_format_marker(dir: &Path) -> bool {
    if read_index_format_version(dir).is_some() {
        return false;
    }
    let Ok(index) = tantivy::Index::open_in_dir(dir) else {
        return false;
    };
    if index.schema().get_field(F_SUBJECT_TRI).is_err() {
        return false;
    }
    match write_index_format_version(dir) {
        Ok(()) => true,
        Err(error) => {
            tracing::warn!(%error, "Failed to write INDEX_FORMAT marker for {:?}", dir);
            false
        }
    }
}

/// Format status of an index directory.
pub fn check_index_format(dir: &Path) -> IndexFormatStatus {
    let empty = std::fs::read_dir(dir)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(true);
    if empty {
        return IndexFormatStatus::Fresh;
    }

    match read_index_format_version(dir) {
        Some(version) if version >= CURRENT_INDEX_FORMAT_VERSION => IndexFormatStatus::Current,
        Some(_) => IndexFormatStatus::NeedsUpgrade,
        None if is_tantivy_index_dir(&PathBuf::from(dir)).unwrap_or(false) => {
            IndexFormatStatus::NeedsUpgrade
        }
        None => IndexFormatStatus::Fresh,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tantivy::schema::SchemaTools;
    use std::fs;
    use tempfile::TempDir;

    /// A directory `is_tantivy_index_dir` accepts.
    fn tantivy_index_fixture(dir: &Path) {
        fs::write(dir.join("meta.json"), "{}").unwrap();
        for ext in ["store", "term", "idx"] {
            fs::write(dir.join(format!("seg.{ext}")), b"x").unwrap();
        }
    }

    #[test]
    fn missing_dir_is_fresh() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(
            check_index_format(&tmp.path().join("missing")),
            IndexFormatStatus::Fresh
        );
    }

    #[test]
    fn empty_dir_is_fresh() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(check_index_format(tmp.path()), IndexFormatStatus::Fresh);
    }

    #[test]
    fn index_without_marker_needs_upgrade() {
        let tmp = TempDir::new().unwrap();
        tantivy_index_fixture(tmp.path());
        assert_eq!(
            check_index_format(tmp.path()),
            IndexFormatStatus::NeedsUpgrade
        );
    }

    #[test]
    fn current_marker_is_current() {
        let tmp = TempDir::new().unwrap();
        tantivy_index_fixture(tmp.path());
        write_index_format_version(tmp.path()).unwrap();
        assert_eq!(check_index_format(tmp.path()), IndexFormatStatus::Current);
    }

    #[test]
    fn older_marker_needs_upgrade() {
        let tmp = TempDir::new().unwrap();
        tantivy_index_fixture(tmp.path());
        fs::write(tmp.path().join(INDEX_FORMAT_FILE), "1\n").unwrap();
        assert_eq!(
            check_index_format(tmp.path()),
            IndexFormatStatus::NeedsUpgrade
        );
    }

    #[test]
    fn other_dir_without_marker_is_fresh() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("random.txt"), b"hello").unwrap();
        assert_eq!(check_index_format(tmp.path()), IndexFormatStatus::Fresh);
    }

    #[test]
    fn heals_a_missing_marker_only_for_the_current_schema() {
        let current = TempDir::new().unwrap();
        let index =
            tantivy::Index::create_in_dir(current.path(), SchemaTools::email_schema()).unwrap();
        assert!(heal_index_format_marker(current.path()));
        assert_eq!(
            read_index_format_version(current.path()),
            Some(CURRENT_INDEX_FORMAT_VERSION)
        );
        assert!(!heal_index_format_marker(current.path()), "already marked");
        drop(index);

        let legacy = TempDir::new().unwrap();
        let mut builder = tantivy::schema::Schema::builder();
        builder.add_text_field("subject", tantivy::schema::TEXT);
        let index = tantivy::Index::create_in_dir(legacy.path(), builder.build()).unwrap();
        assert!(!heal_index_format_marker(legacy.path()));
        assert_eq!(read_index_format_version(legacy.path()), None);
        drop(index);
    }

    #[test]
    fn a_directory_that_is_no_index_is_not_healed() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("random.txt"), b"hello").unwrap();
        assert!(!heal_index_format_marker(tmp.path()));
        assert!(!tmp.path().join(INDEX_FORMAT_FILE).exists());
    }

    #[test]
    fn an_unwritable_marker_is_not_healed() {
        let tmp = TempDir::new().unwrap();
        let index = tantivy::Index::create_in_dir(tmp.path(), SchemaTools::email_schema()).unwrap();
        // A directory in place of the marker can be neither read nor written.
        fs::create_dir(tmp.path().join(INDEX_FORMAT_FILE)).unwrap();

        assert!(!heal_index_format_marker(tmp.path()));
        assert!(tmp.path().join(INDEX_FORMAT_FILE).is_dir());
        drop(index);
    }
}
