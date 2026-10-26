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

//! Rebuild the search indexes in the current index format.
//!
//! Account, mailbox, UID and tag associations live only in the stored fields
//! of the index documents, so the indexes are rebuilt from the old indexes,
//! not from the blobs: every stored value is carried over, and the indexed
//! but not stored fields are derived again (the body from the EML blob, the
//! attachment text from the envelope's attachment metadata).
//!
//! The new indexes are built next to the live ones in `<index root>.new`,
//! validated, and swapped in by renaming. The previous index root is kept as
//! `<index root>.old-<timestamp>` and never deleted. Documents that cannot be
//! carried over completely are listed in `<root>/reindex-skipped.ndjson`.
//!
//! Adapted from upstream `bichon-admin reindex` (ee92a95a, 0da1620b).

use std::{
    collections::BTreeSet,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use mailboxd_blob::{Codec, Config, Engine, Error as BlobError};
use mailboxd_core::{
    error::{code::ErrorCode, MailboxdResult},
    message::content::AttachmentInfo,
    migrate::{is_tantivy_index_dir, read_storage_version, CURRENT_STORAGE_VERSION},
    raise_error,
    settings::dir::{available_space, resolve_dirs, ResolvedDirs},
    store::tantivy::{
        format::{
            check_index_format, write_index_format_version, IndexFormatStatus,
            CURRENT_INDEX_FORMAT_VERSION,
        },
        rebuild::{
            document_tags, extracted_text_of, rebuild_attachment_document,
            rebuild_envelope_document, stored_attachments,
        },
        schema::SchemaTools,
        tokenizers::register_tokenizers,
    },
};
use tantivy::{
    collector::{Count, TopDocs},
    indexer::NoMergePolicy,
    query::{AllQuery, TermQuery},
    schema::{Field, IndexRecordOption, Schema, Value},
    Index, IndexWriter, Searcher, TantivyDocument, Term,
};
use tracing::{info, warn};

use crate::AutoMigratePaths;

/// The rebuild coexists with the live indexes on the same volume.
const DISK_SAFETY_FACTOR: f64 = 1.15;
const COMMIT_EVERY: u64 = 10_000;
const WRITER_THREADS: usize = 3;
const WRITER_HEAP_BYTES: usize = 256 * 1024 * 1024;
const NEW_SUFFIX: &str = ".new";
/// File name of the skip report in the data root.
pub const SKIP_REPORT_FILE: &str = "reindex-skipped.ndjson";

/// Result of a completed reindex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReindexOutcome {
    pub emails: u64,
    pub attachments: u64,
    /// Documents that could not be carried over.
    pub skipped: u64,
    /// Emails carried over without body text because their blob is missing.
    pub missing_bodies: u64,
    /// Attachments carried over without text although they had some.
    pub missing_texts: u64,
    /// The previous index root, kept for the operator to remove.
    pub backup_dir: PathBuf,
    /// The skip report, if any document was recorded in it.
    pub report: Option<PathBuf>,
    pub elapsed: Duration,
}

/// Whether an existing index has to be rebuilt before the server can use it.
pub fn reindex_required(paths: &AutoMigratePaths) -> bool {
    let dirs = resolve(paths);
    [&dirs.envelope_dir, &dirs.attachment_dir]
        .iter()
        .any(|dir| check_index_format(dir) == IndexFormatStatus::NeedsUpgrade)
}

/// Rebuild the search indexes if they are in an older format, or always with
/// `force`. Returns `None` when there is nothing to rebuild.
///
/// Fails without touching the live indexes when the storage is not migrated,
/// the blob store is in use, the disk is too full, or a rebuilt index does
/// not validate.
pub fn reindex(paths: &AutoMigratePaths, force: bool) -> MailboxdResult<Option<ReindexOutcome>> {
    let start = Instant::now();
    let dirs = resolve(paths);
    let has_email = is_index(&dirs.envelope_dir);
    let has_attachment = is_index(&dirs.attachment_dir);
    if !has_email && !has_attachment {
        return Ok(None);
    }
    if !force && !reindex_required(paths) {
        return Ok(None);
    }

    match read_storage_version(&paths.root_dir) {
        Some(version) if version >= CURRENT_STORAGE_VERSION => {}
        _ => {
            return Err(raise_error!(
                format!(
                    "cannot rebuild the search index: the storage in {:?} is not migrated to v{CURRENT_STORAGE_VERSION}",
                    paths.root_dir
                ),
                ErrorCode::InternalError
            ))
        }
    }

    let blobs = BlobGuard::open(&dirs.blobs_dir)?;

    let index_parent = dirs
        .index_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| paths.root_dir.clone());
    let needed = (dir_size(&dirs.index_root) as f64 * DISK_SAFETY_FACTOR) as u64;
    let available = available_space(&index_parent);
    if available < needed {
        return Err(raise_error!(
            format!(
                "not enough disk space to rebuild the search index in {:?}: need about {}, {} available",
                index_parent,
                human_bytes(needed),
                human_bytes(available)
            ),
            ErrorCode::InternalError
        ));
    }

    let new_root = sibling(&dirs.index_root, NEW_SUFFIX);
    if new_root.exists() {
        info!("Removing leftover {:?} of an interrupted reindex", new_root);
        std::fs::remove_dir_all(&new_root).map_err(|e| {
            raise_error!(
                format!("cannot remove {:?}: {e:#?}", new_root),
                ErrorCode::InternalError
            )
        })?;
    }
    let new_envelope = new_root.join(leaf(&dirs.envelope_dir)?);
    let new_attachment = new_root.join(leaf(&dirs.attachment_dir)?);

    info!(
        "Rebuilding the search index in {:?} into {:?}",
        dirs.index_root, new_root
    );
    let mut report = SkipReport::new(paths.root_dir.join(SKIP_REPORT_FILE));
    let built = build(
        &dirs,
        has_email,
        has_attachment,
        &new_envelope,
        &new_attachment,
        blobs.engine(),
        &mut report,
    );
    let totals = match built {
        Ok(totals) => totals,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&new_root);
            return Err(error);
        }
    };

    let backup_dir = backup_path(&dirs.index_root);
    swap(&dirs.index_root, &new_root, &backup_dir)?;
    drop(blobs);

    let outcome = ReindexOutcome {
        emails: totals.emails,
        attachments: totals.attachments,
        skipped: totals.skipped,
        missing_bodies: totals.missing_bodies,
        missing_texts: totals.missing_texts,
        backup_dir,
        report: (report.count > 0).then(|| report.path.clone()),
        elapsed: start.elapsed(),
    };
    info!(
        emails = outcome.emails,
        attachments = outcome.attachments,
        skipped = outcome.skipped,
        missing_bodies = outcome.missing_bodies,
        missing_texts = outcome.missing_texts,
        "Search index rebuilt in {:.1}s; the previous index is kept in {:?}",
        outcome.elapsed.as_secs_f64(),
        outcome.backup_dir
    );
    Ok(Some(outcome))
}

fn resolve(paths: &AutoMigratePaths) -> ResolvedDirs {
    resolve_dirs(
        &paths.root_dir,
        paths.index_dir.as_deref(),
        paths.data_dir.as_deref(),
    )
}

fn is_index(dir: &Path) -> bool {
    is_tantivy_index_dir(&dir.to_path_buf()).unwrap_or(false)
}

#[derive(Default)]
struct Totals {
    emails: u64,
    attachments: u64,
    skipped: u64,
    missing_bodies: u64,
    missing_texts: u64,
}

/// Build and validate the new indexes. The old indexes are closed on return.
fn build(
    dirs: &ResolvedDirs,
    has_email: bool,
    has_attachment: bool,
    new_envelope: &Path,
    new_attachment: &Path,
    blobs: &Engine,
    report: &mut SkipReport,
) -> MailboxdResult<Totals> {
    let mut totals = Totals::default();
    let old_email = has_email
        .then(|| OldIndex::open(&dirs.envelope_dir))
        .transpose()?;

    if let Some(old) = &old_email {
        let target = NewIndex::create(new_envelope, SchemaTools::email_schema())?;
        let phase = rebuild_emails(old, target, blobs, report)?;
        validate(
            new_envelope,
            phase.docs,
            phase.sample_id.as_deref(),
            SchemaTools::email_fields().f_id,
            "email",
        )?;
        totals.emails = phase.docs;
        totals.skipped += phase.skipped;
        totals.missing_bodies = phase.missing;
    }

    if has_attachment {
        let old = OldIndex::open(&dirs.attachment_dir)?;
        let target = NewIndex::create(new_attachment, SchemaTools::attachment_schema())?;
        let phase = rebuild_attachments(&old, old_email.as_ref(), target, report)?;
        validate(
            new_attachment,
            phase.docs,
            phase.sample_id.as_deref(),
            SchemaTools::attachment_fields().f_id,
            "attachment",
        )?;
        totals.attachments = phase.docs;
        totals.skipped += phase.skipped;
        totals.missing_texts = phase.missing;
    }

    Ok(totals)
}

// ─── old and new indexes ──────────────────────────────────────────────────────

struct OldIndex {
    schema: Schema,
    searcher: Searcher,
}

impl OldIndex {
    fn open(dir: &Path) -> MailboxdResult<Self> {
        let index = Index::open_in_dir(dir).map_err(|e| {
            raise_error!(
                format!("cannot open index {:?}: {e:#?}", dir),
                ErrorCode::InternalError
            )
        })?;
        let reader = index.reader().map_err(|e| {
            raise_error!(
                format!("cannot read index {:?}: {e:#?}", dir),
                ErrorCode::InternalError
            )
        })?;
        Ok(Self {
            schema: index.schema(),
            searcher: reader.searcher(),
        })
    }

    /// A stored document with its values moved to the fields of the same
    /// name in `schema`. Field ids differ between index formats, so values
    /// must not be copied by id. Values of fields `schema` lacks are dropped
    /// and their names collected in `dropped`.
    fn remap(
        &self,
        doc: &TantivyDocument,
        schema: &Schema,
        dropped: &mut BTreeSet<String>,
    ) -> TantivyDocument {
        let mut remapped = TantivyDocument::new();
        for (field, value) in doc.field_values() {
            let name = self.schema.get_field_name(field);
            match schema.get_field(name) {
                Ok(target) => remapped.add_field_value(target, value),
                Err(_) => {
                    dropped.insert(name.to_string());
                }
            }
        }
        remapped
    }

    /// Every stored field of `doc` as JSON, for the skip report.
    fn describe(&self, doc: &TantivyDocument) -> serde_json::Value {
        let mut fields = serde_json::Map::new();
        for (field, value) in doc.field_values() {
            let value = if let Some(s) = value.as_str() {
                serde_json::Value::from(s)
            } else if let Some(n) = value.as_u64() {
                serde_json::Value::from(n)
            } else if let Some(n) = value.as_i64() {
                serde_json::Value::from(n)
            } else if let Some(b) = value.as_bool() {
                serde_json::Value::from(b)
            } else {
                serde_json::Value::Null
            };
            let name = self.schema.get_field_name(field).to_string();
            match fields.get_mut(&name) {
                Some(serde_json::Value::Array(values)) => values.push(value),
                Some(existing) => *existing = serde_json::json!([existing.take(), value]),
                None => {
                    fields.insert(name, value);
                }
            }
        }
        serde_json::Value::Object(fields)
    }
}

struct NewIndex {
    schema: Schema,
    writer: IndexWriter,
}

impl NewIndex {
    fn create(dir: &Path, schema: Schema) -> MailboxdResult<Self> {
        std::fs::create_dir_all(dir).map_err(|e| {
            raise_error!(
                format!("cannot create {:?}: {e:#?}", dir),
                ErrorCode::InternalError
            )
        })?;
        let index = Index::create_in_dir(dir, schema.clone()).map_err(|e| {
            raise_error!(
                format!("cannot create index {:?}: {e:#?}", dir),
                ErrorCode::InternalError
            )
        })?;
        register_tokenizers(&index);
        let writer = index
            .writer_with_num_threads(WRITER_THREADS, WRITER_HEAP_BYTES)
            .map_err(|e| {
                raise_error!(
                    format!("cannot create index writer: {e:#?}"),
                    ErrorCode::InternalError
                )
            })?;
        writer.set_merge_policy(Box::new(NoMergePolicy));
        write_index_format_version(dir).map_err(|e| {
            raise_error!(
                format!("cannot write the index format marker in {:?}: {e:#?}", dir),
                ErrorCode::InternalError
            )
        })?;
        Ok(Self { schema, writer })
    }

    fn commit(&mut self) -> MailboxdResult<()> {
        self.writer
            .commit()
            .map(|_| ())
            .map_err(|e| raise_error!(format!("commit failed: {e:#?}"), ErrorCode::InternalError))
    }

    /// Commit, merge into one segment and close the writer.
    fn finish(mut self) -> MailboxdResult<()> {
        self.commit()?;
        let segments = self
            .writer
            .index()
            .searchable_segment_ids()
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
        if segments.len() > 1 {
            self.writer.merge(&segments).wait().map_err(|e| {
                raise_error!(format!("merge failed: {e:#?}"), ErrorCode::InternalError)
            })?;
        }
        self.writer
            .wait_merging_threads()
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))
    }
}

#[derive(Default)]
struct PhaseStats {
    docs: u64,
    skipped: u64,
    /// Bodies (emails) or texts (attachments) that could not be restored.
    missing: u64,
    /// Id of a written document, queried back by the validation.
    sample_id: Option<String>,
}

/// Visit every live stored document of `old`. `visit` gets the document and
/// its location; unreadable documents are recorded and skipped.
fn for_each_document(
    old: &OldIndex,
    kind: &'static str,
    report: &mut SkipReport,
    stats: &mut PhaseStats,
    mut visit: impl FnMut(
        TantivyDocument,
        Location,
        &mut SkipReport,
        &mut PhaseStats,
    ) -> MailboxdResult<()>,
) -> MailboxdResult<()> {
    let total = old.searcher.num_docs();
    let mut seen = 0u64;
    for segment in old.searcher.segment_readers() {
        // An unreadable store is not a per-document problem: abort rather
        // than lose the segment.
        let store = segment.get_store_reader(1).map_err(|e| {
            raise_error!(
                format!("cannot read the {kind} document store: {e:#?}"),
                ErrorCode::InternalError
            )
        })?;
        let segment_id = segment.segment_id().uuid_string();
        for doc_id in 0..segment.max_doc() {
            if segment
                .alive_bitset()
                .is_some_and(|alive| !alive.is_alive(doc_id))
            {
                continue;
            }
            let location = Location {
                kind,
                segment: segment_id.clone(),
                doc_id,
            };
            match store.get::<TantivyDocument>(doc_id) {
                Ok(doc) => visit(doc, location, report, stats)?,
                Err(e) => {
                    report.record(
                        &location,
                        "unreadable_document",
                        Some(format!("{e:#?}")),
                        None,
                    );
                    stats.skipped += 1;
                }
            }
            seen += 1;
            if seen % COMMIT_EVERY == 0 {
                info!("Rebuilding the {kind} index: {seen}/{total} documents");
            }
        }
    }
    Ok(())
}

fn rebuild_emails(
    old: &OldIndex,
    mut target: NewIndex,
    blobs: &Engine,
    report: &mut SkipReport,
) -> MailboxdResult<PhaseStats> {
    let f = SchemaTools::email_fields();
    let mut stats = PhaseStats::default();
    let mut dropped = BTreeSet::new();
    // Duplicates share a content hash; keep the last EML.
    let mut last_eml: Option<(String, Option<Vec<u8>>)> = None;

    for_each_document(
        old,
        "email",
        report,
        &mut stats,
        |old_doc, location, report, stats| {
            let doc = old.remap(&old_doc, &target.schema, &mut dropped);
            let tags = document_tags(&doc, f.f_tags);
            let hash = doc.get_first(f.f_content_hash).and_then(|v| v.as_str());

            let eml = match (&last_eml, hash) {
                (Some((last, eml)), Some(hash)) if last == hash => eml.clone(),
                (_, Some(hash)) => {
                    let eml = read_blob(blobs, hash);
                    last_eml = Some((hash.to_string(), eml.clone()));
                    eml
                }
                (_, None) => None,
            };
            if eml.is_none() {
                stats.missing += 1;
                report.record(
                    &location,
                    "missing_body",
                    None,
                    Some(old.describe(&old_doc)),
                );
            }

            let new_doc = rebuild_envelope_document(&doc, &tags, eml.as_deref());
            add(
                &mut target,
                new_doc,
                &doc,
                f.f_id,
                &location,
                old,
                &old_doc,
                report,
                stats,
            )
        },
    )?;

    target.finish()?;
    warn_dropped("email", &dropped);
    Ok(stats)
}

fn rebuild_attachments(
    old: &OldIndex,
    old_email: Option<&OldIndex>,
    mut target: NewIndex,
    report: &mut SkipReport,
) -> MailboxdResult<PhaseStats> {
    let f = SchemaTools::attachment_fields();
    let email_schema = SchemaTools::email_schema();
    let old_email_id = old_email.and_then(|old| old.schema.get_field("id").ok());
    let mut stats = PhaseStats::default();
    let mut dropped = BTreeSet::new();
    // Attachments of one envelope are usually next to each other.
    let mut last_parent: Option<(String, Vec<AttachmentInfo>)> = None;

    for_each_document(
        old,
        "attachment",
        report,
        &mut stats,
        |old_doc, location, report, stats| {
            let doc = old.remap(&old_doc, &target.schema, &mut dropped);
            let tags = document_tags(&doc, f.f_tags);
            let envelope_id = doc.get_first(f.f_envelope_id).and_then(|v| v.as_str());
            let content_hash = doc.get_first(f.f_content_hash).and_then(|v| v.as_str());

            let text = match (old_email.zip(old_email_id), envelope_id, content_hash) {
                (Some((email, id_field)), Some(envelope_id), Some(content_hash)) => {
                    if last_parent.as_ref().map(|(id, _)| id.as_str()) != Some(envelope_id) {
                        let attachments =
                            parent_attachments(email, id_field, envelope_id, &email_schema)?;
                        last_parent = Some((envelope_id.to_string(), attachments));
                    }
                    last_parent
                        .as_ref()
                        .and_then(|(_, attachments)| extracted_text_of(attachments, content_hash))
                }
                _ => None,
            };
            let had_text = doc
                .get_first(f.f_has_text)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if text.is_none() && had_text {
                stats.missing += 1;
                report.record(
                    &location,
                    "missing_attachment_text",
                    None,
                    Some(old.describe(&old_doc)),
                );
            }

            let new_doc = rebuild_attachment_document(&doc, &tags, text.as_deref());
            add(
                &mut target,
                new_doc,
                &doc,
                f.f_id,
                &location,
                old,
                &old_doc,
                report,
                stats,
            )
        },
    )?;

    target.finish()?;
    warn_dropped("attachment", &dropped);
    Ok(stats)
}

/// Attachment metadata of the envelope `envelope_id` in the old email index;
/// empty when the envelope is missing.
fn parent_attachments(
    email: &OldIndex,
    id_field: Field,
    envelope_id: &str,
    email_schema: &Schema,
) -> MailboxdResult<Vec<AttachmentInfo>> {
    let query = TermQuery::new(
        Term::from_field_text(id_field, envelope_id),
        IndexRecordOption::Basic,
    );
    let hits = email
        .searcher
        .search(&query, &TopDocs::with_limit(1).order_by_score())
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
    let Some((_, address)) = hits.first() else {
        return Ok(Vec::new());
    };
    let doc: TantivyDocument = email
        .searcher
        .doc(*address)
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
    let doc = email.remap(&doc, email_schema, &mut BTreeSet::new());
    Ok(stored_attachments(&doc).unwrap_or_default())
}

#[allow(clippy::too_many_arguments)]
fn add(
    target: &mut NewIndex,
    new_doc: TantivyDocument,
    doc: &TantivyDocument,
    id_field: Field,
    location: &Location,
    old: &OldIndex,
    old_doc: &TantivyDocument,
    report: &mut SkipReport,
    stats: &mut PhaseStats,
) -> MailboxdResult<()> {
    if let Err(e) = target.writer.add_document(new_doc) {
        report.record(
            location,
            "skipped_document",
            Some(format!("{e:#?}")),
            Some(old.describe(old_doc)),
        );
        stats.skipped += 1;
        return Ok(());
    }
    stats.docs += 1;
    if stats.sample_id.is_none() {
        stats.sample_id = doc
            .get_first(id_field)
            .and_then(|v| v.as_str())
            .map(String::from);
    }
    if stats.docs % COMMIT_EVERY == 0 {
        target.commit()?;
    }
    Ok(())
}

fn read_blob(blobs: &Engine, content_hash: &str) -> Option<Vec<u8>> {
    let mut key = [0u8; 32];
    if let Err(error) = hex::decode_to_slice(content_hash, &mut key) {
        warn!(%error, "Invalid content hash {content_hash:?}");
        return None;
    }
    match blobs.get(&key) {
        Ok(eml) => eml,
        Err(error) => {
            warn!(%error, "Cannot read blob {content_hash}");
            None
        }
    }
}

fn warn_dropped(kind: &str, dropped: &BTreeSet<String>) {
    if !dropped.is_empty() {
        warn!(
            "Dropped {kind} index fields the current schema no longer has: {:?}",
            dropped
        );
    }
}

// ─── validation and swap ──────────────────────────────────────────────────────

/// Check a rebuilt index before it replaces the live one: current format
/// marker, `expected` documents, a query over the whole index, and the
/// sample document found by its id.
fn validate(
    dir: &Path,
    expected: u64,
    sample_id: Option<&str>,
    id_field: Field,
    kind: &str,
) -> MailboxdResult<()> {
    let fail = |detail: String| {
        raise_error!(
            format!(
                "validation of the rebuilt {kind} index {:?} failed: {detail}",
                dir
            ),
            ErrorCode::InternalError
        )
    };
    if check_index_format(dir) != IndexFormatStatus::Current {
        return Err(fail(format!(
            "not marked as format v{CURRENT_INDEX_FORMAT_VERSION}"
        )));
    }
    let index = Index::open_in_dir(dir).map_err(|e| fail(format!("{e:#?}")))?;
    register_tokenizers(&index);
    let searcher = index
        .reader()
        .map_err(|e| fail(format!("{e:#?}")))?
        .searcher();

    let docs = searcher.num_docs();
    if docs != expected {
        return Err(fail(format!("{docs} documents, expected {expected}")));
    }
    let matched = searcher
        .search(&AllQuery, &Count)
        .map_err(|e| fail(format!("{e:#?}")))? as u64;
    if matched != expected {
        return Err(fail(format!(
            "{matched} documents match, expected {expected}"
        )));
    }
    if let Some(id) = sample_id {
        let query = TermQuery::new(
            Term::from_field_text(id_field, id),
            IndexRecordOption::Basic,
        );
        let hits = searcher
            .search(&query, &Count)
            .map_err(|e| fail(format!("{e:#?}")))?;
        if hits == 0 {
            return Err(fail(format!("document {id:?} not found")));
        }
    }
    info!("Rebuilt {kind} index validated: {docs} documents");
    Ok(())
}

/// Replace `index_root` with `new_root`, keeping the previous root as
/// `backup`. On failure the previous root is put back.
fn swap(index_root: &Path, new_root: &Path, backup: &Path) -> MailboxdResult<()> {
    if let Err(e) = rename_with_retry(index_root, backup) {
        let _ = std::fs::remove_dir_all(new_root);
        return Err(raise_error!(
            format!("cannot move {:?} to {:?}: {e:#?}", index_root, backup),
            ErrorCode::InternalError
        ));
    }
    if let Err(e) = rename_with_retry(new_root, index_root) {
        let restored = rename_with_retry(backup, index_root);
        let detail = match restored {
            Ok(()) => {
                let _ = std::fs::remove_dir_all(new_root);
                "the previous index was put back".to_string()
            }
            Err(_) => format!(
                "the previous index is in {:?}; rename it back to {:?}",
                backup, index_root
            ),
        };
        return Err(raise_error!(
            format!(
                "cannot move {:?} to {:?}: {e:#?}; {detail}",
                new_root, index_root
            ),
            ErrorCode::InternalError
        ));
    }
    Ok(())
}

fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut attempt = 0;
    loop {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if attempt >= 2 => return Err(e),
            Err(_) => {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
}

fn sibling(dir: &Path, suffix: &str) -> PathBuf {
    let mut name = dir.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

fn leaf(path: &Path) -> MailboxdResult<&std::ffi::OsStr> {
    path.file_name().ok_or_else(|| {
        raise_error!(
            format!("{:?} has no final component", path),
            ErrorCode::InternalError
        )
    })
}

/// `<index root>.old-<timestamp>`, with a counter if that name is taken.
fn backup_path(index_root: &Path) -> PathBuf {
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let mut candidate = sibling(index_root, &format!(".old-{stamp}"));
    let mut n = 1;
    while candidate.exists() {
        candidate = sibling(index_root, &format!(".old-{stamp}-{n}"));
        n += 1;
    }
    candidate
}

fn dir_size(dir: &Path) -> u64 {
    let mut total = 0;
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => pending.push(entry.path()),
                Ok(_) => total += entry.metadata().map(|m| m.len()).unwrap_or(0),
                Err(_) => {}
            }
        }
    }
    total
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

// ─── blob store ───────────────────────────────────────────────────────────────

/// The blob store, held for the whole run so a server cannot start on the
/// same data meanwhile; released on drop.
struct BlobGuard(Engine);

impl BlobGuard {
    fn open(dir: &Path) -> MailboxdResult<Self> {
        std::fs::create_dir_all(dir).map_err(|e| {
            raise_error!(
                format!("cannot create {:?}: {e:#?}", dir),
                ErrorCode::InternalError
            )
        })?;
        let mut config = Config::default();
        config.default_codec = Codec::Zstd;
        config.compress_threshold = 1024;
        config.flush_interval_secs = 0;
        config.gc_interval_secs = 0;
        match Engine::open(dir, config) {
            Ok(engine) => Ok(Self(engine)),
            Err(BlobError::AlreadyOpen { .. }) => Err(raise_error!(
                format!(
                    "cannot rebuild the search index: the blob store {:?} is in use; stop mailboxd and retry",
                    dir
                ),
                ErrorCode::InternalError
            )),
            Err(e) => Err(raise_error!(
                format!("cannot open the blob store {:?}: {e:#?}", dir),
                ErrorCode::InternalError
            )),
        }
    }

    fn engine(&self) -> &Engine {
        &self.0
    }
}

impl Drop for BlobGuard {
    fn drop(&mut self) {
        if let Err(error) = self.0.shutdown() {
            warn!(%error, "Failed to close the blob store");
        }
    }
}

// ─── skip report ──────────────────────────────────────────────────────────────

struct Location {
    kind: &'static str,
    segment: String,
    doc_id: u32,
}

/// NDJSON list of documents the reindex could not carry over completely:
/// - `unreadable_document`: the stored document cannot be read; it is lost
/// - `skipped_document`: the document could not be written; it is lost
/// - `missing_body`: the email is kept, but its blob is missing, so its body
///   is not searchable
/// - `missing_attachment_text`: the attachment is kept, but its text is not
///   in the envelope's attachment metadata, so it is not searchable by text
///
/// Records carry every stored field, so a message can be fetched again from
/// its mailbox. The file holds plain-text message metadata and is created
/// readable by the owner only.
struct SkipReport {
    path: PathBuf,
    file: Option<std::fs::File>,
    count: u64,
}

impl SkipReport {
    fn new(path: PathBuf) -> Self {
        // A report of an earlier run must not pass for this one's.
        if let Err(error) = std::fs::remove_file(&path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                warn!(%error, "Cannot remove the old reindex report {:?}", path);
            }
        }
        Self {
            path,
            file: None,
            count: 0,
        }
    }

    fn record(
        &mut self,
        location: &Location,
        reason: &str,
        error: Option<String>,
        fields: Option<serde_json::Value>,
    ) {
        let mut record = serde_json::json!({
            "reason": reason,
            "index": location.kind,
            "segment": location.segment,
            "doc_id": location.doc_id,
        });
        if let Some(error) = error {
            record["error"] = error.into();
        }
        if let Some(fields) = fields {
            record["fields"] = fields;
        }
        warn!(reason, index = location.kind, "Reindex: {}", record);

        if self.file.is_none() {
            match create_private_file(&self.path) {
                Ok(file) => self.file = Some(file),
                Err(error) => {
                    warn!(%error, "Cannot create the reindex report {:?}", self.path);
                    return;
                }
            }
        }
        if let Some(file) = self.file.as_mut() {
            match writeln!(file, "{record}") {
                Ok(()) => self.count += 1,
                Err(error) => warn!(%error, "Cannot write the reindex report {:?}", self.path),
            }
        }
    }
}

fn create_private_file(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mailboxd_core::migrate::write_storage_version;
    use tantivy::{
        query::QueryParser,
        schema::{Facet, OwnedValue},
    };
    use tempfile::TempDir;

    const EML: &[u8] = b"From: u1@example.com\r\n\
        Subject: Report rv1234\r\n\
        Date: Thu, 01 Jan 2023 00:00:00 +0000\r\n\
        \r\n\
        lorem qx7731 ipsum\r\n";

    fn hash(byte: &str) -> String {
        byte.repeat(32)
    }

    /// The current schema without the trigram fields, as format 1 had it.
    fn legacy_schema(current: Schema) -> Schema {
        let mut builder = Schema::builder();
        for (_, entry) in current.fields() {
            if !entry.name().ends_with("_tri") {
                builder.add_field(entry.clone());
            }
        }
        builder.build()
    }

    fn legacy_doc(schema: &Schema, values: &[(&str, OwnedValue)]) -> TantivyDocument {
        let mut doc = TantivyDocument::new();
        for (name, value) in values {
            doc.add_field_value(schema.get_field(name).unwrap(), value);
        }
        doc
    }

    fn write_legacy_index(dir: &Path, schema: Schema, docs: Vec<TantivyDocument>) {
        std::fs::create_dir_all(dir).unwrap();
        let index = Index::create_in_dir(dir, schema).unwrap();
        register_tokenizers(&index);
        let mut writer: IndexWriter = index.writer_with_num_threads(1, 15_000_000).unwrap();
        for doc in docs {
            writer.add_document(doc).unwrap();
        }
        writer.commit().unwrap();
    }

    fn text(s: &str) -> OwnedValue {
        OwnedValue::Str(s.to_string())
    }

    fn facet(path: &str) -> OwnedValue {
        OwnedValue::Facet(Facet::from(path))
    }

    /// A v2 data root with a format 1 index: one email with its blob, one
    /// without, one attachment with text in its envelope's metadata and one
    /// whose envelope is gone.
    fn seed_legacy_root() -> (TempDir, AutoMigratePaths) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        write_storage_version(&root, CURRENT_STORAGE_VERSION).unwrap();
        let dirs = resolve_dirs(&root, None, None);

        let email = legacy_schema(SchemaTools::email_schema());
        let attachments = serde_json::json!([
            {"filename": "inv9921.pdf", "file_type": "application/pdf", "inline": false,
             "size": 5, "content_id": null, "content_hash": hash("cd"), "is_message": false,
             "extracted_text": "total zq4420 due", "extracted_page_count": 1}
        ]);
        let emails = vec![
            legacy_doc(
                &email,
                &[
                    ("id", text("eid-1")),
                    ("account_id", OwnedValue::U64(1)),
                    ("mailbox_id", OwnedValue::U64(2)),
                    ("uid", OwnedValue::U64(3)),
                    ("subject", text("Report rv1234")),
                    ("content_hash", text(&hash("ab"))),
                    ("from", text("u1@example.com")),
                    ("attachments", text(&attachments.to_string())),
                    ("tags", facet("/inbox")),
                ],
            ),
            legacy_doc(
                &email,
                &[
                    ("id", text("eid-2")),
                    ("account_id", OwnedValue::U64(1)),
                    ("subject", text("Lost body")),
                    ("content_hash", text(&hash("ef"))),
                ],
            ),
        ];
        write_legacy_index(&dirs.envelope_dir, email, emails);

        let attachment = legacy_schema(SchemaTools::attachment_schema());
        let attachments = vec![
            legacy_doc(
                &attachment,
                &[
                    ("id", text("aid-1")),
                    ("eid", text("eid-1")),
                    ("account_id", OwnedValue::U64(1)),
                    ("subject", text("Report rv1234")),
                    ("content_hash", text(&hash("cd"))),
                    ("name_exact", text("inv9921.pdf")),
                    ("has_text", OwnedValue::Bool(true)),
                    ("is_indexed", OwnedValue::Bool(true)),
                    ("tags", facet("/old")),
                ],
            ),
            legacy_doc(
                &attachment,
                &[
                    ("id", text("aid-2")),
                    ("eid", text("eid-gone")),
                    ("account_id", OwnedValue::U64(1)),
                    ("content_hash", text(&hash("12"))),
                    ("has_text", OwnedValue::Bool(true)),
                    ("is_indexed", OwnedValue::Bool(true)),
                ],
            ),
        ];
        write_legacy_index(&dirs.attachment_dir, attachment, attachments);

        let blobs = BlobGuard::open(&dirs.blobs_dir).unwrap();
        let mut key = [0u8; 32];
        hex::decode_to_slice(hash("ab"), &mut key).unwrap();
        blobs.engine().put(key, EML, Codec::Zstd).unwrap();
        drop(blobs);

        let paths = AutoMigratePaths {
            root_dir: root,
            index_dir: None,
            data_dir: None,
        };
        (tmp, paths)
    }

    fn searcher(dir: &Path) -> (Index, Searcher) {
        let index = Index::open_in_dir(dir).unwrap();
        register_tokenizers(&index);
        let searcher = index.reader().unwrap().searcher();
        (index, searcher)
    }

    fn hits(index: &Index, searcher: &Searcher, field: Field, query: &str) -> usize {
        let query = QueryParser::for_index(index, vec![field])
            .parse_query(query)
            .unwrap();
        searcher.search(&query, &Count).unwrap()
    }

    fn term_hits(searcher: &Searcher, term: Term) -> usize {
        searcher
            .search(&TermQuery::new(term, IndexRecordOption::Basic), &Count)
            .unwrap()
    }

    fn stored_doc(searcher: &Searcher, id_field: Field, id: &str) -> TantivyDocument {
        let query = TermQuery::new(
            Term::from_field_text(id_field, id),
            IndexRecordOption::Basic,
        );
        let top = searcher
            .search(&query, &TopDocs::with_limit(1).order_by_score())
            .unwrap();
        searcher.doc(top[0].1).unwrap()
    }

    #[test]
    fn rebuilds_a_legacy_index_and_keeps_the_old_one() {
        let (_tmp, paths) = seed_legacy_root();
        let dirs = resolve(&paths);
        assert!(reindex_required(&paths));

        let outcome = reindex(&paths, false)
            .unwrap()
            .expect("legacy index is rebuilt");
        assert_eq!(outcome.emails, 2);
        assert_eq!(outcome.attachments, 2);
        assert_eq!(outcome.skipped, 0);
        assert_eq!(outcome.missing_bodies, 1);
        assert_eq!(outcome.missing_texts, 1);

        // Swapped in, in the current format, the previous root kept.
        assert!(!reindex_required(&paths));
        for dir in [&dirs.envelope_dir, &dirs.attachment_dir] {
            assert_eq!(check_index_format(dir), IndexFormatStatus::Current);
        }
        assert!(!sibling(&dirs.index_root, NEW_SUFFIX).exists());
        assert!(is_index(&outcome.backup_dir.join("mail_metadata")));
        assert!(is_index(&outcome.backup_dir.join("attachment_metadata")));

        // Emails: stored values carried over, body and trigrams derived.
        let f = SchemaTools::email_fields();
        let (index, s) = searcher(&dirs.envelope_dir);
        assert_eq!(hits(&index, &s, f.f_body, "qx7731"), 1);
        assert_eq!(hits(&index, &s, f.f_subject, "rv1234"), 1);
        assert_eq!(hits(&index, &s, f.f_from_text, "u1"), 1);
        assert_eq!(
            s.search(
                &mailboxd_core::store::tantivy::tokenizers::tri_query(&[f.f_subject_tri], "rv123"),
                &Count
            )
            .unwrap(),
            1
        );
        assert_eq!(
            term_hits(&s, Term::from_facet(f.f_tags, &Facet::from("/inbox"))),
            1
        );
        for (field, value) in [(f.f_account_id, 1), (f.f_mailbox_id, 2), (f.f_uid, 3)] {
            assert!(term_hits(&s, Term::from_field_u64(field, value)) >= 1);
        }
        assert_eq!(hits(&index, &s, f.f_subject, "lost"), 1);

        // Attachments: text from the envelope metadata, flags reset without it.
        let af = SchemaTools::attachment_fields();
        let (index, s) = searcher(&dirs.attachment_dir);
        assert_eq!(hits(&index, &s, af.f_text, "zq4420"), 1);
        assert_eq!(hits(&index, &s, af.f_name_text, "inv9921"), 1);
        assert_eq!(
            term_hits(&s, Term::from_facet(af.f_tags, &Facet::from("/old"))),
            1
        );
        let orphan = stored_doc(&s, af.f_id, "aid-2");
        assert_eq!(
            orphan.get_first(af.f_has_text).and_then(|v| v.as_bool()),
            Some(false)
        );

        // The skip report lists the two incomplete documents, owner-only.
        let report = outcome.report.expect("incomplete documents are reported");
        assert_eq!(report, paths.root_dir.join(SKIP_REPORT_FILE));
        let lines: Vec<serde_json::Value> = std::fs::read_to_string(&report)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let reasons: BTreeSet<_> = lines
            .iter()
            .map(|l| l["reason"].as_str().unwrap())
            .collect();
        assert_eq!(
            reasons,
            BTreeSet::from(["missing_body", "missing_attachment_text"])
        );
        assert!(lines.iter().any(|l| l["fields"]["id"] == "eid-2"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&report).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn a_current_index_is_only_rebuilt_when_forced() {
        let (_tmp, paths) = seed_legacy_root();
        let first = reindex(&paths, false).unwrap().unwrap();

        assert_eq!(reindex(&paths, false).unwrap(), None);

        let forced = reindex(&paths, true).unwrap().expect("force rebuilds");
        assert_eq!(forced.emails, 2);
        assert_ne!(forced.backup_dir, first.backup_dir);
        assert!(first.backup_dir.is_dir());
        assert!(forced.backup_dir.is_dir());
    }

    #[test]
    fn nothing_to_do_without_an_index() {
        let tmp = TempDir::new().unwrap();
        let paths = AutoMigratePaths {
            root_dir: tmp.path().to_path_buf(),
            index_dir: None,
            data_dir: None,
        };
        assert!(!reindex_required(&paths));
        assert_eq!(reindex(&paths, true).unwrap(), None);
    }

    #[test]
    fn a_locked_blob_store_leaves_the_live_index_alone() {
        let (_tmp, paths) = seed_legacy_root();
        let dirs = resolve(&paths);
        let held = BlobGuard::open(&dirs.blobs_dir).unwrap();

        let error = reindex(&paths, false).unwrap_err();
        assert!(format!("{error:?}").contains("in use"), "{error:?}");

        drop(held);
        assert!(reindex_required(&paths));
        assert!(!sibling(&dirs.index_root, NEW_SUFFIX).exists());
        assert!(!paths.root_dir.join(SKIP_REPORT_FILE).exists());
    }

    #[test]
    fn an_unmigrated_storage_is_refused() {
        let (_tmp, paths) = seed_legacy_root();
        write_storage_version(&paths.root_dir, CURRENT_STORAGE_VERSION - 1).unwrap();

        assert!(reindex(&paths, false).is_err());
        assert!(reindex_required(&paths));
    }
}
