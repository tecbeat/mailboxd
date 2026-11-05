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
/// - 3 = CJK text as unigrams and bigrams, not transliterated
/// - 4 = the analyzers of 3; every commit carries the writer stamp, so an
///   index an older mailboxd wrote to is rebuilt (see [`CommitCurrent`])
///
/// Bump this whenever the schema or the analyzers change; older indexes are
/// then rebuilt on the next start.
pub const CURRENT_INDEX_FORMAT_VERSION: u32 = 4;

/// Analyzers of the current version, but written before commits were
/// stamped. Such an index is adopted in place, see [`adopt_unstamped_index`].
const UNSTAMPED_VERSION: u32 = 3;

/// Heap of the writer that stamps an adopted index; the commit adds no
/// documents, and tantivy needs at least 15 MB per thread.
const ADOPT_WRITER_HEAP_BYTES: usize = 15_000_000;

/// The first version with trigram fields. A schema with them proves no more:
/// later versions change only the analyzers.
const TRIGRAM_SCHEMA_VERSION: u32 = 2;

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
    write_version(dir, CURRENT_INDEX_FORMAT_VERSION)
}

fn write_version(dir: &Path, version: u32) -> std::io::Result<()> {
    std::fs::write(dir.join(INDEX_FORMAT_FILE), format!("{version}\n"))
}

/// Format version of an index directory; `None` without a readable marker.
pub fn read_index_format_version(dir: &Path) -> Option<u32> {
    let content = std::fs::read_to_string(dir.join(INDEX_FORMAT_FILE)).ok()?;
    content.trim().parse().ok()
}

/// Stamp an index whose marker write was lost, e.g. by a crash between
/// creating the index and writing the marker. Only an index whose schema
/// already has the trigram fields is stamped, so a legacy index still goes
/// through the reindex. The stamp is version 2, the most the schema proves:
/// an index of an older analyzer is rebuilt, never served as current.
/// Returns `true` when a marker was written.
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
    match write_version(dir, TRIGRAM_SCHEMA_VERSION) {
        Ok(()) => true,
        Err(error) => {
            tracing::warn!(%error, "Failed to write INDEX_FORMAT marker for {:?}", dir);
            false
        }
    }
}

/// Commit payload of a writer that analyzes text like format
/// [`CURRENT_INDEX_FORMAT_VERSION`].
fn writer_stamp() -> String {
    format!("mailboxd-index-format={CURRENT_INDEX_FORMAT_VERSION}")
}

/// Commits that stamp the index as written by the current analyzers.
///
/// Versions before format 3 accept a newer marker and keep writing to the
/// index with their own analyzers, which leaves a mixed index once the user
/// upgrades again. Their commits carry no payload, so every commit of this
/// version must go through [`CommitCurrent::commit_current`].
pub trait CommitCurrent {
    fn commit_current(&mut self) -> tantivy::Result<u64>;
}

impl CommitCurrent for tantivy::IndexWriter {
    fn commit_current(&mut self) -> tantivy::Result<u64> {
        let mut prepared = self.prepare_commit()?;
        prepared.set_payload(&writer_stamp());
        prepared.commit()
    }
}

/// True when the last commit to the index in `dir` came from a writer
/// without the current stamp, i.e. an older mailboxd. An index nobody has
/// committed to yet holds nothing to rebuild. Merges keep the payload of the
/// last commit, so they never count as foreign. An unreadable index is left
/// to the open that follows, which reports the actual error.
fn last_commit_by_an_older_writer(dir: &Path) -> bool {
    let Ok(metas) = tantivy::Index::open_in_dir(dir).and_then(|index| index.load_metas()) else {
        return false;
    };
    metas.opstamp > 0 && metas.payload.as_deref() != Some(writer_stamp().as_str())
}

/// Move an index of version 3 to the current version without a rebuild: its
/// analyzers are the current ones, only its commits lack the stamp. Stamps
/// it with an empty commit first and bumps the marker after, so an
/// interruption in between leaves a version 3 index that is adopted again.
/// Must run before anything else opens a writer on `dir`. Returns `true`
/// when the index was adopted; on failure the index keeps version 3 and is
/// rebuilt.
pub fn adopt_unstamped_index(dir: &Path) -> bool {
    if read_index_format_version(dir) != Some(UNSTAMPED_VERSION) {
        return false;
    }
    let stamped = tantivy::Index::open_in_dir(dir)
        .and_then(|index| index.writer_with_num_threads(1, ADOPT_WRITER_HEAP_BYTES))
        .and_then(|mut writer: tantivy::IndexWriter| {
            writer.commit_current()?;
            writer.wait_merging_threads()
        });
    if let Err(error) = stamped {
        tracing::warn!(%error, "Cannot stamp the index in {:?}; it will be rebuilt", dir);
        return false;
    }
    match write_index_format_version(dir) {
        Ok(()) => {
            tracing::info!(
                "Index in {:?} moved to format v{CURRENT_INDEX_FORMAT_VERSION} in place",
                dir
            );
            true
        }
        Err(error) => {
            tracing::warn!(%error, "Cannot write INDEX_FORMAT marker for {:?}", dir);
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
        Some(version) if version >= CURRENT_INDEX_FORMAT_VERSION => {
            if last_commit_by_an_older_writer(dir) {
                tracing::warn!(
                    "The index in {:?} was written by an older mailboxd version; rebuilding it",
                    dir
                );
                IndexFormatStatus::NeedsUpgrade
            } else {
                IndexFormatStatus::Current
            }
        }
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
        for older in ["1\n", "2\n", "3\n"] {
            fs::write(tmp.path().join(INDEX_FORMAT_FILE), older).unwrap();
            assert_eq!(
                check_index_format(tmp.path()),
                IndexFormatStatus::NeedsUpgrade,
                "marker {older:?}"
            );
        }
    }

    /// Version 3 indexes CJK text as unigrams and bigrams, version 4 adds
    /// the writer stamp on top.
    #[test]
    fn current_version_is_the_stamped_cjk_aware_tokenizer() {
        assert_eq!(UNSTAMPED_VERSION, 3);
        assert_eq!(CURRENT_INDEX_FORMAT_VERSION, 4);
    }

    /// A real email index in `dir` with `ids` committed and `marker`
    /// written. `stamped` picks the commit of this version or the plain
    /// commit of an older mailboxd.
    fn email_index(dir: &Path, ids: &[&str], marker: u32, stamped: bool) {
        let index = tantivy::Index::create_in_dir(dir, SchemaTools::email_schema()).unwrap();
        crate::store::tantivy::tokenizers::register_tokenizers(&index);
        let mut writer: tantivy::IndexWriter = index.writer(15_000_000).unwrap();
        add_ids(&mut writer, ids);
        commit(&mut writer, stamped);
        writer.wait_merging_threads().unwrap();
        write_version(dir, marker).unwrap();
    }

    fn add_ids(writer: &mut tantivy::IndexWriter, ids: &[&str]) {
        let f_id = SchemaTools::email_fields().f_id;
        for id in ids {
            writer.add_document(tantivy::doc!(f_id => *id)).unwrap();
        }
    }

    fn commit(writer: &mut tantivy::IndexWriter, stamped: bool) {
        if stamped {
            writer.commit_current().unwrap();
        } else {
            writer.commit().unwrap();
        }
    }

    /// What an older mailboxd does with the index: open it, add mail and
    /// commit without the stamp.
    fn write_like_an_older_version(dir: &Path, ids: &[&str]) {
        let index = tantivy::Index::open_in_dir(dir).unwrap();
        crate::store::tantivy::tokenizers::register_tokenizers(&index);
        let mut writer: tantivy::IndexWriter = index.writer(15_000_000).unwrap();
        add_ids(&mut writer, ids);
        writer.commit().unwrap();
        writer.wait_merging_threads().unwrap();
    }

    fn num_docs(dir: &Path) -> u64 {
        tantivy::Index::open_in_dir(dir)
            .unwrap()
            .reader()
            .unwrap()
            .searcher()
            .num_docs()
    }

    #[test]
    fn an_index_committed_by_this_version_is_current() {
        let tmp = TempDir::new().unwrap();
        email_index(tmp.path(), &["a", "b"], CURRENT_INDEX_FORMAT_VERSION, true);
        assert_eq!(check_index_format(tmp.path()), IndexFormatStatus::Current);
    }

    /// Versions before format 3 accept marker 4 and write with their own
    /// analyzers; the index must be rebuilt once this version runs again.
    #[test]
    fn an_index_an_older_version_committed_to_needs_a_rebuild() {
        let tmp = TempDir::new().unwrap();
        email_index(tmp.path(), &["a"], CURRENT_INDEX_FORMAT_VERSION, true);

        write_like_an_older_version(tmp.path(), &["b"]);

        assert_eq!(
            read_index_format_version(tmp.path()),
            Some(4),
            "marker untouched"
        );
        assert_eq!(
            check_index_format(tmp.path()),
            IndexFormatStatus::NeedsUpgrade
        );
    }

    #[test]
    fn a_new_index_without_any_commit_is_current() {
        let tmp = TempDir::new().unwrap();
        let index = tantivy::Index::create_in_dir(tmp.path(), SchemaTools::email_schema()).unwrap();
        write_index_format_version(tmp.path()).unwrap();
        assert_eq!(check_index_format(tmp.path()), IndexFormatStatus::Current);
        drop(index);
    }

    /// Merges keep the payload of the last commit, whoever runs them.
    #[test]
    fn merging_segments_keeps_the_stamp() {
        let tmp = TempDir::new().unwrap();
        email_index(tmp.path(), &["a"], CURRENT_INDEX_FORMAT_VERSION, true);
        let index = tantivy::Index::open_in_dir(tmp.path()).unwrap();
        crate::store::tantivy::tokenizers::register_tokenizers(&index);
        let mut writer: tantivy::IndexWriter = index.writer(15_000_000).unwrap();
        writer.set_merge_policy(Box::new(tantivy::indexer::NoMergePolicy));
        add_ids(&mut writer, &["b"]);
        writer.commit_current().unwrap();
        let segments = index.searchable_segment_ids().unwrap();
        assert_eq!(segments.len(), 2);
        writer.merge(&segments).wait().unwrap();
        writer.wait_merging_threads().unwrap();

        assert_eq!(index.searchable_segment_ids().unwrap().len(), 1);
        assert_eq!(check_index_format(tmp.path()), IndexFormatStatus::Current);
        assert_eq!(num_docs(tmp.path()), 2);
    }

    /// Installs that already run the CJK build hold a version 3 index
    /// without stamps. It is adopted in place: no rebuild, no lost mail.
    #[test]
    fn a_version_3_index_is_adopted_without_a_rebuild() {
        let tmp = TempDir::new().unwrap();
        email_index(tmp.path(), &["a", "b", "c"], UNSTAMPED_VERSION, false);
        assert_eq!(
            check_index_format(tmp.path()),
            IndexFormatStatus::NeedsUpgrade,
            "not adopted yet"
        );

        assert!(adopt_unstamped_index(tmp.path()));

        assert_eq!(read_index_format_version(tmp.path()), Some(4));
        assert_eq!(check_index_format(tmp.path()), IndexFormatStatus::Current);
        assert_eq!(num_docs(tmp.path()), 3);
        assert!(!adopt_unstamped_index(tmp.path()), "adopted only once");
    }

    #[test]
    fn an_adopted_index_an_older_version_wrote_to_needs_a_rebuild() {
        let tmp = TempDir::new().unwrap();
        email_index(tmp.path(), &["a"], UNSTAMPED_VERSION, false);
        assert!(adopt_unstamped_index(tmp.path()));

        write_like_an_older_version(tmp.path(), &["b"]);

        assert!(!adopt_unstamped_index(tmp.path()));
        assert_eq!(
            check_index_format(tmp.path()),
            IndexFormatStatus::NeedsUpgrade
        );
    }

    /// A crash between the stamping commit and the marker leaves a stamped
    /// version 3 index, which the next start adopts again.
    #[test]
    fn an_interrupted_adoption_is_finished_by_the_next_one() {
        let tmp = TempDir::new().unwrap();
        email_index(tmp.path(), &["a"], UNSTAMPED_VERSION, true);

        assert!(adopt_unstamped_index(tmp.path()));
        assert_eq!(check_index_format(tmp.path()), IndexFormatStatus::Current);
        assert_eq!(num_docs(tmp.path()), 1);
    }

    #[test]
    fn only_version_3_indexes_are_adopted() {
        for (marker, status) in [
            (TRIGRAM_SCHEMA_VERSION, IndexFormatStatus::NeedsUpgrade),
            (
                CURRENT_INDEX_FORMAT_VERSION,
                IndexFormatStatus::NeedsUpgrade,
            ),
        ] {
            let tmp = TempDir::new().unwrap();
            email_index(tmp.path(), &["a"], marker, false);
            assert!(!adopt_unstamped_index(tmp.path()), "marker {marker}");
            assert_eq!(read_index_format_version(tmp.path()), Some(marker));
            assert_eq!(check_index_format(tmp.path()), status, "marker {marker}");
        }
    }

    /// When the index cannot be stamped, e.g. because a writer holds its
    /// lock, it keeps version 3 and goes through the regular rebuild.
    #[test]
    fn a_locked_version_3_index_is_left_for_the_rebuild() {
        let tmp = TempDir::new().unwrap();
        email_index(tmp.path(), &["a"], UNSTAMPED_VERSION, false);
        let index = tantivy::Index::open_in_dir(tmp.path()).unwrap();
        let writer: tantivy::IndexWriter = index.writer(15_000_000).unwrap();

        assert!(!adopt_unstamped_index(tmp.path()));

        assert_eq!(read_index_format_version(tmp.path()), Some(3));
        assert_eq!(
            check_index_format(tmp.path()),
            IndexFormatStatus::NeedsUpgrade
        );
        drop(writer);
    }

    #[test]
    fn other_dir_without_marker_is_fresh() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("random.txt"), b"hello").unwrap();
        assert_eq!(check_index_format(tmp.path()), IndexFormatStatus::Fresh);
    }

    /// The schema with trigram fields proves version 2, not the tokenizer
    /// of version 3, so a healed index is still rebuilt.
    #[test]
    fn heals_a_missing_marker_as_version_2_only_for_the_trigram_schema() {
        let current = TempDir::new().unwrap();
        let index =
            tantivy::Index::create_in_dir(current.path(), SchemaTools::email_schema()).unwrap();
        assert!(heal_index_format_marker(current.path()));
        assert_eq!(read_index_format_version(current.path()), Some(2));
        assert_eq!(
            check_index_format(current.path()),
            IndexFormatStatus::NeedsUpgrade
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
