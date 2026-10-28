//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
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

use std::{
    collections::{HashMap, HashSet},
    ops::Bound,
    path::PathBuf,
    sync::{Arc, LazyLock},
    time::Duration,
};

use crate::{
    account::{migration::AccountModel, stats::AccountStats},
    common::{paginated::DataPage, signal::SIGNAL_MANAGER},
    dashboard::{DashboardStats, Group, LargestEmail, TimeBucket},
    error::{code::ErrorCode, MailboxdResult},
    message::{
        content::AttachmentInfo,
        search::{EmailSearchFilter, SortBy},
        tags::{TagAction, TagCount, TagsRequest},
    },
    raise_error,
    settings::dir::DATA_DIR_MANAGER,
    store::{
        blob::BLOB_MANAGER,
        envelope::Envelope,
        tantivy::{
            attachment::ATTACHMENT_MANAGER,
            dedup_cache::DEDUP_CACHE,
            fatal_commit,
            fields::{
                F_ACCOUNT_ID, F_DATE, F_FROM, F_ID, F_INTERNAL_DATE, F_REGULAR_ATTACHMENT_COUNT,
                F_SIZE, F_TAGS, F_THREAD_ID, F_UID,
            },
            format::{
                check_index_format, heal_index_format_marker, write_index_format_version,
                IndexFormatStatus,
            },
            model::{extract_contacts, EnvelopeWithAttachments},
            rebuild::{document_tags, rebuild_envelope_document, stored_attachments},
            schema::SchemaTools,
            sort::top_docs_page,
            tokenizers::{register_tokenizers, text_parser, word_or_tri},
        },
    },
    utc_now,
};

use chrono::Utc;
use serde_json::json;
use tantivy::{
    aggregation::{
        agg_req::Aggregations,
        agg_result::{
            AggregationResult, AggregationResults, BucketEntries, BucketResult, MetricResult,
        },
        AggregationCollector, Key,
    },
    collector::{Count, DocSetCollector, FacetCollector, TopDocs},
    indexer::{LogMergePolicy, UserOperation},
    query::{AllQuery, BooleanQuery, EmptyQuery, Occur, Query, RangeQuery, TermQuery},
    schema::{IndexRecordOption, Value},
    DocAddress, Index, IndexReader, IndexWriter, Order, TantivyDocument, Term,
};
use tantivy::{schema::Facet, Searcher};
use tokio::{
    sync::{mpsc, Mutex},
    task::{self, JoinHandle},
};
use tracing::{info, warn};

pub static ENVELOPE_MANAGER: LazyLock<IndexManager> = LazyLock::new(IndexManager::new);

/// Lightweight snapshot of a single envelope, used as the local side of
/// the gap-fill diff without materializing full `Envelope` structs.
///
/// Loads every document of the mailbox into memory at once — the caller
/// must not use this for mailboxes too large to hold in a full in-memory
/// pass. Documents without a message-id are excluded by
/// `get_envelope_snapshots_for_mailbox` since the diff relies on
/// message-id / fingerprint matching.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnvelopeSnapshot {
    pub message_id: String,
    pub uid: u64,
    pub size: u64,
    /// Epoch millis (internal date).
    pub internal_date: i64,
    //pub subject: String,
}

pub struct IndexManager {
    index: Arc<Index>,
    index_writer: Arc<Mutex<IndexWriter>>,
    sender: mpsc::Sender<TantivyDocument>,
    reader: IndexReader,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl IndexManager {
    pub(crate) fn index_writer(&self) -> &Arc<Mutex<IndexWriter>> {
        &self.index_writer
    }

    pub(crate) fn create_reader(&self) -> MailboxdResult<IndexReader> {
        self.index
            .reader()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
    }

    pub async fn shutdown(&self) {
        let mut guard = self.handle.lock().await;
        if let Some(handle) = guard.take() {
            let _ = handle.await;
        }
    }
    pub fn new() -> Self {
        let index = Self::open_or_create_index(&DATA_DIR_MANAGER.envelope_dir);
        register_tokenizers(&index);
        let mut merge_policy = LogMergePolicy::default();
        merge_policy.set_min_num_segments(25);
        merge_policy.set_min_layer_size(10_000);
        merge_policy.set_max_docs_before_merge(100_000);

        let index_writer = index
            .writer_with_num_threads(4, 67_108_864)
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to create IndexWriter with 4 threads and 64MB buffer for {:?}: {}",
                    &DATA_DIR_MANAGER.envelope_dir, e
                )
            });
        index_writer.set_merge_policy(Box::new(merge_policy));
        let index_writer = Arc::new(Mutex::new(index_writer));
        let reader = index.reader().unwrap_or_else(|e| {
            panic!(
                "Failed to create IndexReader for {:?}: {}",
                &DATA_DIR_MANAGER.envelope_dir, e
            )
        });

        let (sender, mut receiver) = mpsc::channel::<TantivyDocument>(100);

        let writer = index_writer.clone();
        let handler = task::spawn(async move {
            let mut shutdown = SIGNAL_MANAGER.subscribe();
            let mut commit_interval = tokio::time::interval(Duration::from_secs(60));
            let mut pending_count = 0;
            let commit_threshold = 1000;
            loop {
                tokio::select! {
                    maybe_msg = receiver.recv() => {
                        match maybe_msg {
                            Some(doc) => {
                                let mut writer = writer.lock().await;
                                let mut batch_count = 0;
                                match writer.add_document(doc) {
                                    Ok(_) => {
                                        batch_count += 1;
                                    }
                                    Err(e) => {
                                        eprintln!("[ERROR] Failed to add document: {e:?}");
                                        tracing::error!("Tantivy: Failed to add document: {e:?}");
                                    }
                                }
                                while let Ok(next_doc) = receiver.try_recv() {
                                    match writer.add_document(next_doc) {
                                        Ok(_) => batch_count += 1,
                                        Err(e) => {
                                            eprintln!("[ERROR] Failed to add document: {e:?}");
                                            tracing::error!("Tantivy: Failed to add document: {e:?}");
                                        }
                                    }
                                }
                                if batch_count > 0 {
                                    pending_count += batch_count;
                                }
                                if pending_count >= commit_threshold {
                                    tracing::info!(
                                        "Tantivy: Reached threshold ({} docs), committing...",
                                        pending_count
                                    );
                                    tokio::task::block_in_place(|| fatal_commit(&mut writer));
                                    tracing::debug!(
                                        "Tantivy: committed {} docs, pending reset to 0",
                                        pending_count
                                    );
                                    pending_count = 0;
                                    commit_interval.reset();
                                }
                            }
                            None => {
                                tracing::info!("Tantivy: Receiver closed. Finalizing...");
                                if pending_count > 0 {
                                    let mut writer = writer.lock().await;
                                    tokio::task::block_in_place(|| fatal_commit(&mut writer));
                                }
                                break;
                            },
                        }
                    }
                    _ = commit_interval.tick() => {
                        if pending_count > 0 {
                            let mut writer = writer.lock().await;
                            tracing::debug!(
                                "Tantivy: periodic commit ({} docs pending)",
                                pending_count
                            );
                            tokio::task::block_in_place(|| fatal_commit(&mut writer));
                            pending_count = 0;
                        }
                    }
                    _ = shutdown.recv() => {
                        tracing::info!("Tantivy: Shutdown signal received. Performing final commit...");
                        if pending_count > 0 {
                            let mut writer = writer.lock().await;
                            tokio::task::block_in_place(|| fatal_commit(&mut writer));
                        }
                        tracing::info!("Tantivy: Shutdown cleanup complete.");
                        break;
                    }
                }
            }
        });
        Self {
            index: Arc::new(index),
            index_writer,
            sender,
            reader,
            handle: Mutex::new(Some(handler)),
        }
    }

    pub async fn queue(&self, doc: TantivyDocument) {
        if let Err(e) = self.sender.send(doc).await {
            tracing::warn!(error = %e, "Failed to queue document into Tantivy writer channel");
        }
    }

    fn open_or_create_index(index_dir: &PathBuf) -> Index {
        let need_create = !index_dir.exists()
            || index_dir
                .read_dir()
                .map(|mut d| d.next().is_none())
                .unwrap_or(true);
        if need_create {
            info!(
                "Email index not found or empty, creating new index at {}",
                index_dir.display()
            );
            std::fs::create_dir_all(&index_dir).unwrap_or_else(|e| {
                panic!("Failed to create index directory {:?}: {}", index_dir, e)
            });
            let index = Index::create_in_dir(index_dir, SchemaTools::email_schema())
                .unwrap_or_else(|e| panic!("Failed to create index in {:?}: {}", index_dir, e));
            if let Err(e) = write_index_format_version(index_dir) {
                warn!(
                    "Failed to write INDEX_FORMAT marker for {:?}: {}",
                    index_dir, e
                );
            }
            index
        } else {
            info!("Opening existing email index at {}", index_dir.display());
            heal_index_format_marker(index_dir);
            if check_index_format(index_dir) == IndexFormatStatus::NeedsUpgrade {
                panic!(
                    "The email index in {:?} has an outdated format and must be rebuilt; \
                     restart mailboxd to rebuild it automatically",
                    index_dir
                );
            }
            Self::open(&index_dir)
        }
    }

    fn open(index_dir: &PathBuf) -> Index {
        Index::open_in_dir(index_dir)
            .unwrap_or_else(|e| panic!("Failed to open index in {:?}: {}", index_dir, e))
    }

    fn account_query(&self, account_id: u64) -> Box<TermQuery> {
        let account_term =
            Term::from_field_u64(SchemaTools::email_fields().f_account_id, account_id);
        Box::new(TermQuery::new(account_term, IndexRecordOption::Basic))
    }

    pub(crate) fn mailbox_query(&self, account_id: u64, mailbox_id: u64) -> Box<dyn Query> {
        let account_query = TermQuery::new(
            Term::from_field_u64(SchemaTools::email_fields().f_account_id, account_id),
            IndexRecordOption::Basic,
        );
        let mailbox_query = TermQuery::new(
            Term::from_field_u64(SchemaTools::email_fields().f_mailbox_id, mailbox_id),
            IndexRecordOption::Basic,
        );
        let boolean_query = BooleanQuery::new(vec![
            (Occur::Must, Box::new(account_query)),
            (Occur::Must, Box::new(mailbox_query)),
        ]);
        Box::new(boolean_query)
    }

    /// Return all Message-IDs stored in Tantivy for a given mailbox.
    /// Prefer `mailbox_contains_message_id` for existence checks on large
    /// mailboxes — this method loads everything into a HashSet.
    pub fn get_message_ids_for_mailbox(
        &self,
        account_id: u64,
        mailbox_id: u64,
    ) -> MailboxdResult<HashSet<String>> {
        let query = self.mailbox_query(account_id, mailbox_id);
        let fields = SchemaTools::email_fields();
        let searcher = self.create_searcher()?;

        let docs = searcher
            .search(&query, &DocSetCollector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        let mut result = HashSet::new();
        for doc_address in docs {
            let doc = searcher
                .doc::<TantivyDocument>(doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

            if let Some(v) = doc.get_first(fields.f_message_id) {
                if let Some(s) = v.as_str() {
                    if !s.is_empty() {
                        result.insert(s.to_string());
                    }
                }
            }
        }
        Ok(result)
    }

    /// Returns lightweight snapshots of every envelope stored for a mailbox:
    /// message-id, uid, size, internal date (epoch millis), subject.
    /// Used by gap-fill to compute the local side of the diff without
    /// materializing full `Envelope` structs.
    pub fn get_envelope_snapshots_for_mailbox(
        &self,
        account_id: u64,
        mailbox_id: u64,
    ) -> MailboxdResult<Vec<EnvelopeSnapshot>> {
        let query = self.mailbox_query(account_id, mailbox_id);
        let fields = SchemaTools::email_fields();
        let searcher = self.create_searcher()?;

        let docs = searcher
            .search(&query, &DocSetCollector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        let mut snapshots = Vec::with_capacity(docs.len());
        for doc_address in docs {
            let doc = searcher
                .doc::<TantivyDocument>(doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            let message_id = doc
                .get_first(fields.f_message_id)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            // Skip documents without a message-id: they are useless for
            // gap-fill (the diff matches on message-id / fingerprint) and
            // would otherwise surface as spurious "missing" entries in the
            // difference set. Other fields keep their fallback defaults.
            if message_id.is_empty() {
                continue;
            }
            let uid = doc
                .get_first(fields.f_uid)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let size = doc
                .get_first(fields.f_size)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let internal_date = doc
                .get_first(fields.f_internal_date)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            // let subject = doc
            //     .get_first(fields.f_subject)
            //     .and_then(|v| v.as_str())
            //     .unwrap_or("")
            //     .to_string();
            snapshots.push(EnvelopeSnapshot {
                message_id,
                uid,
                size,
                internal_date,
                //subject,
            });
        }
        Ok(snapshots)
    }

    /// Check whether a specific Message-ID exists in a mailbox.
    /// Uses a TermQuery — O(1) per call, no allocation proportional to
    /// mailbox size. Suitable for large mailboxes where
    /// `get_message_ids_for_mailbox` would allocate too much memory.
    pub fn mailbox_contains_message_id(
        &self,
        account_id: u64,
        mailbox_id: u64,
        message_id: &str,
    ) -> MailboxdResult<bool> {
        let fields = SchemaTools::email_fields();
        let query = BooleanQuery::new(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(fields.f_account_id, account_id),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(fields.f_mailbox_id, mailbox_id),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(fields.f_message_id, message_id),
                    IndexRecordOption::Basic,
                )),
            ),
        ]);
        let searcher = self.create_searcher()?;
        let count = searcher
            .search(&query, &Count)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        Ok(count > 0)
    }

    fn envelope_query(&self, account_id: u64, eid: &str) -> Box<dyn Query> {
        envelope_query(account_id, eid)
    }

    fn filter_query(
        &self,
        accounts: Option<HashSet<u64>>,
        filter: EmailSearchFilter,
    ) -> MailboxdResult<Box<dyn Query>> {
        let f = SchemaTools::email_fields();
        let mut subqueries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        if let Some(authorized_ids) = accounts {
            if authorized_ids.is_empty() {
                let term = Term::from_field_u64(f.f_account_id, u64::MAX);
                subqueries.push((
                    Occur::Must,
                    Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                ));
            } else {
                let mut account_must_queries = Vec::new();
                for id in authorized_ids {
                    let term = Term::from_field_u64(f.f_account_id, id);
                    account_must_queries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>,
                    ));
                }
                subqueries.push((
                    Occur::Must,
                    Box::new(BooleanQuery::new(account_must_queries)),
                ));
            }
        }

        if let Some(ref text) = filter.text {
            let query_parser = text_parser(&self.index, SchemaTools::email_default_fields());

            let word = query_parser
                .parse_query(text)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InvalidParameter))?;
            let tri_fields = [
                f.f_subject_tri,
                f.f_from_text_tri,
                f.f_to_text_tri,
                f.f_cc_text_tri,
                f.f_bcc_text_tri,
                f.f_attachment_name_text_tri,
            ];
            subqueries.push((Occur::Must, word_or_tri(word, &tri_fields, text)));
        }

        if let Some(ref subject_val) = filter.subject {
            let query_parser = text_parser(&self.index, vec![f.f_subject]);
            let word = query_parser
                .parse_query(subject_val)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InvalidParameter))?;
            subqueries.push((
                Occur::Must,
                word_or_tri(word, &[f.f_subject_tri], subject_val),
            ));
        }

        if let Some(ref body_val) = filter.body {
            let query_parser = text_parser(&self.index, vec![f.f_body]);

            let q = query_parser
                .parse_query(body_val)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InvalidParameter))?;
            subqueries.push((Occur::Must, q));
        }

        if let Some(ref tags) = filter.tags {
            if !tags.is_empty() {
                let mut should_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

                for tag in tags {
                    let facet = Facet::from_text(tag).map_err(|e| {
                        raise_error!(format!("{:#?}", e), ErrorCode::InvalidParameter)
                    })?;

                    let term = Term::from_facet(f.f_tags, &facet);

                    should_queries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                    ));
                }
                subqueries.push((Occur::Must, Box::new(BooleanQuery::new(should_queries))));
            }
        }

        for (field, tri_field, opt_value) in [
            (f.f_from_text, f.f_from_text_tri, &filter.from),
            (f.f_to_text, f.f_to_text_tri, &filter.to),
            (f.f_cc_text, f.f_cc_text_tri, &filter.cc),
            (f.f_bcc_text, f.f_bcc_text_tri, &filter.bcc),
        ] {
            if let Some(ref v) = opt_value {
                let query_parser = text_parser(&self.index, vec![field]);
                let word = query_parser
                    .parse_query(v)
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InvalidParameter))?;
                subqueries.push((Occur::Must, word_or_tri(word, &[tri_field], v)));
            }
        }

        // any_recipient: OR across to, cc, bcc
        if let Some(ref v) = filter.any_recipient {
            let mut recipient_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
            for (field, tri_field) in [
                (f.f_to_text, f.f_to_text_tri),
                (f.f_cc_text, f.f_cc_text_tri),
                (f.f_bcc_text, f.f_bcc_text_tri),
            ] {
                let query_parser = text_parser(&self.index, vec![field]);
                if let Ok(word) = query_parser.parse_query(v) {
                    recipient_queries.push((Occur::Should, word_or_tri(word, &[tri_field], v)));
                }
            }
            if !recipient_queries.is_empty() {
                subqueries.push((Occur::Must, Box::new(BooleanQuery::new(recipient_queries))));
            }
        }

        // any_participant: OR across from, to, cc, bcc
        if let Some(ref v) = filter.any_participant {
            let mut participant_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
            for (field, tri_field) in [
                (f.f_from_text, f.f_from_text_tri),
                (f.f_to_text, f.f_to_text_tri),
                (f.f_cc_text, f.f_cc_text_tri),
                (f.f_bcc_text, f.f_bcc_text_tri),
            ] {
                let query_parser = text_parser(&self.index, vec![field]);
                if let Ok(word) = query_parser.parse_query(v) {
                    participant_queries.push((Occur::Should, word_or_tri(word, &[tri_field], v)));
                }
            }
            if !participant_queries.is_empty() {
                subqueries.push((
                    Occur::Must,
                    Box::new(BooleanQuery::new(participant_queries)),
                ));
            }
        }

        if let Some(has) = filter.has_attachment {
            let lower: Bound<Term>;
            let upper: Bound<Term>;

            if has {
                lower = Bound::Included(Term::from_field_u64(f.f_regular_attachment_count, 1));
                upper =
                    Bound::Included(Term::from_field_u64(f.f_regular_attachment_count, u64::MAX));
            } else {
                lower = Bound::Included(Term::from_field_u64(f.f_regular_attachment_count, 0));
                upper = Bound::Included(Term::from_field_u64(f.f_regular_attachment_count, 0));
            }

            subqueries.push((Occur::Must, Box::new(RangeQuery::new(lower, upper))));
        }

        if let Some(ref name) = filter.attachment_name {
            if name.contains('.') {
                let term = Term::from_field_text(f.f_attachment_name_exact, name);
                let exact_query = TermQuery::new(term, IndexRecordOption::Basic);

                let query_parser = text_parser(&self.index, vec![f.f_attachment_name_text]);
                let word: Box<dyn Query> = query_parser
                    .parse_query(name)
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InvalidParameter))?;
                let query = BooleanQuery::new(vec![
                    (Occur::Should, Box::new(exact_query)),
                    (
                        Occur::Should,
                        word_or_tri(word, &[f.f_attachment_name_text_tri], name),
                    ),
                ]);
                subqueries.push((Occur::Must, Box::new(query)));
            } else {
                let query_parser = text_parser(&self.index, vec![f.f_attachment_name_text]);
                let word = query_parser
                    .parse_query(name)
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InvalidParameter))?;
                subqueries.push((
                    Occur::Must,
                    word_or_tri(word, &[f.f_attachment_name_text_tri], name),
                ));
            }
        }

        if let Some(ref id) = filter.id {
            let term = Term::from_field_text(f.f_id, id);
            let query = TermQuery::new(term, IndexRecordOption::Basic);
            subqueries.push((Occur::Must, Box::new(query)));
        }

        if let Some(ref extension) = filter.attachment_extension {
            let term = Term::from_field_text(f.f_attachment_ext, extension);
            let query = TermQuery::new(term, IndexRecordOption::Basic);
            subqueries.push((Occur::Must, Box::new(query)));
        }

        if let Some(ref category) = filter.attachment_category {
            let term = Term::from_field_text(f.f_attachment_category, category);
            let query = TermQuery::new(term, IndexRecordOption::Basic);
            subqueries.push((Occur::Must, Box::new(query)));
        }

        if let Some(ref content_type) = filter.attachment_content_type {
            let term = Term::from_field_text(f.f_attachment_content_type, content_type);
            let query = TermQuery::new(term, IndexRecordOption::Basic);
            subqueries.push((Occur::Must, Box::new(query)));
        }

        let start_bound = if let Some(from) = filter.since {
            Bound::Included(Term::from_field_i64(f.f_date, from))
        } else {
            Bound::Unbounded
        };

        let end_bound = if let Some(to) = filter.before {
            Bound::Included(Term::from_field_i64(f.f_date, to))
        } else {
            Bound::Unbounded
        };

        if start_bound != Bound::Unbounded || end_bound != Bound::Unbounded {
            let q = RangeQuery::new(start_bound, end_bound);
            subqueries.push((Occur::Must, Box::new(q)));
        }

        let start_bound = if let Some(from) = filter.internal_date_since {
            Bound::Included(Term::from_field_i64(f.f_internal_date, from))
        } else {
            Bound::Unbounded
        };

        let end_bound = if let Some(to) = filter.internal_date_before {
            Bound::Included(Term::from_field_i64(f.f_internal_date, to))
        } else {
            Bound::Unbounded
        };

        if start_bound != Bound::Unbounded || end_bound != Bound::Unbounded {
            let q = RangeQuery::new(start_bound, end_bound);
            subqueries.push((Occur::Must, Box::new(q)));
        }

        let start_bound = if let Some(from) = filter.ingest_since {
            Bound::Included(Term::from_field_i64(f.f_ingest_at, from))
        } else {
            Bound::Unbounded
        };

        let end_bound = if let Some(to) = filter.ingest_before {
            Bound::Included(Term::from_field_i64(f.f_ingest_at, to))
        } else {
            Bound::Unbounded
        };

        if start_bound != Bound::Unbounded || end_bound != Bound::Unbounded {
            let q = RangeQuery::new(start_bound, end_bound);
            subqueries.push((Occur::Must, Box::new(q)));
        }

        if let Some(account_ids) = filter.account_ids {
            let mut should_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
            for id in account_ids {
                let term = Term::from_field_u64(f.f_account_id, id);
                should_queries.push((
                    Occur::Should,
                    Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                ));
            }
            subqueries.push((Occur::Must, Box::new(BooleanQuery::new(should_queries))));
        }

        if let Some(mailbox_ids) = filter.mailbox_ids {
            let mut should_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
            for id in mailbox_ids {
                let term = Term::from_field_u64(f.f_mailbox_id, id);
                should_queries.push((
                    Occur::Should,
                    Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                ));
            }
            subqueries.push((Occur::Must, Box::new(BooleanQuery::new(should_queries))));
        }

        let start_bound = if let Some(from) = filter.min_size {
            Bound::Included(Term::from_field_u64(f.f_size, from))
        } else {
            Bound::Unbounded
        };

        let end_bound = if let Some(to) = filter.max_size {
            Bound::Included(Term::from_field_u64(f.f_size, to))
        } else {
            Bound::Unbounded
        };

        if start_bound != Bound::Unbounded || end_bound != Bound::Unbounded {
            let q = RangeQuery::new(start_bound, end_bound);
            subqueries.push((Occur::Must, Box::new(q)));
        }

        if let Some(ref msg_id) = filter.message_id {
            let term = Term::from_field_text(f.f_message_id, msg_id);
            subqueries.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }

        if subqueries.is_empty() {
            return Ok(Box::new(AllQuery));
        }

        Ok(Box::new(BooleanQuery::new(subqueries)))
    }

    fn thread_query(&self, account_id: u64, thread_id: &str) -> Box<dyn Query> {
        let account_query = TermQuery::new(
            Term::from_field_u64(SchemaTools::email_fields().f_account_id, account_id),
            IndexRecordOption::Basic,
        );
        let thread_query = TermQuery::new(
            Term::from_field_text(SchemaTools::email_fields().f_thread_id, thread_id),
            IndexRecordOption::Basic,
        );
        let boolean_query = BooleanQuery::new(vec![
            (Occur::Must, Box::new(account_query)),
            (Occur::Must, Box::new(thread_query)),
        ]);
        Box::new(boolean_query)
    }

    pub fn get_envelope_by_id(
        &self,
        account_id: u64,
        envelope_id: &str,
    ) -> MailboxdResult<Option<EnvelopeWithAttachments>> {
        let searcher = self.create_searcher()?;
        let f = SchemaTools::email_fields();

        let query = BooleanQuery::new(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(f.f_account_id, account_id),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f.f_id, envelope_id),
                    IndexRecordOption::Basic,
                )),
            ),
        ]);

        let docs: Vec<(f32, DocAddress)> = searcher
            .search(&query, &TopDocs::with_limit(1).order_by_score())
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        if let Some((_, doc_address)) = docs.first() {
            let doc: TantivyDocument = searcher
                .doc(*doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            let envelope = EnvelopeWithAttachments::from_tantivy_doc(&doc)?;
            Ok(Some(envelope))
        } else {
            Ok(None)
        }
    }

    /// Attachment metadata stored with an envelope, or `None` if the
    /// envelope doesn't exist.
    pub fn stored_attachments_of(
        &self,
        account_id: u64,
        envelope_id: &str,
    ) -> MailboxdResult<Option<Vec<AttachmentInfo>>> {
        let searcher = self.create_searcher()?;
        let docs = searcher
            .search(
                self.envelope_query(account_id, envelope_id).as_ref(),
                &TopDocs::with_limit(1).order_by_score(),
            )
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        let Some((_, doc_address)) = docs.first() else {
            return Ok(None);
        };
        let doc: TantivyDocument = searcher
            .doc(*doc_address)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        Ok(Some(stored_attachments(&doc).unwrap_or_default()))
    }

    pub fn top_10_largest_emails(
        &self,
        accounts: &Option<HashSet<u64>>,
    ) -> MailboxdResult<Vec<LargestEmail>> {
        self.reader
            .reload()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        let searcher = self.reader.searcher();

        let query: Box<dyn Query> = match accounts {
            Some(ref ids) if !ids.is_empty() => {
                let mut subqueries = Vec::new();
                for &id in ids {
                    let term = Term::from_field_u64(SchemaTools::email_fields().f_account_id, id);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>,
                    ));
                }
                Box::new(BooleanQuery::new(subqueries))
            }
            Some(_) => Box::new(EmptyQuery),
            None => Box::new(AllQuery),
        };

        let mailbox_docs: Vec<(Option<u64>, DocAddress)> = searcher
            .search(
                &query,
                &TopDocs::with_limit(10).order_by_fast_field(F_SIZE, Order::Desc),
            )
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        let mut result = Vec::new();

        for (_, doc_address) in mailbox_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            let envelope = LargestEmail::from_tantivy_doc(&doc)?;
            result.push(envelope);
        }
        Ok(result)
    }

    pub fn total_emails(&self, accounts: &Option<HashSet<u64>>) -> MailboxdResult<u64> {
        let searcher = self.create_searcher()?;

        match accounts {
            Some(ref ids) if !ids.is_empty() => {
                let mut subqueries = Vec::new();
                for &id in ids {
                    let term = Term::from_field_u64(SchemaTools::email_fields().f_account_id, id);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>,
                    ));
                }
                let query = Box::new(BooleanQuery::new(subqueries)) as Box<dyn Query>;
                let count = searcher
                    .search(&query, &Count)
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
                Ok(count as u64)
            }
            Some(_) => Ok(0),
            None => Ok(searcher.num_docs()),
        }
    }

    pub fn get_max_uid(&self, account_id: u64, mailbox_id: u64) -> MailboxdResult<Option<u64>> {
        let searcher = self.create_searcher()?;

        let query = self.mailbox_query(account_id, mailbox_id);
        let agg_req: Aggregations = serde_json::from_value(json!({
            "max_uid": {
                "max": {
                    "field": F_UID
                }
            }
        }))
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        let collector = AggregationCollector::from_aggs(agg_req, Default::default());

        let agg_res = searcher
            .search(query.as_ref(), &collector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        let result = Self::extract_max_uid(&agg_res);
        tracing::debug!(
            "[account {}][mailbox {}] get_max_uid = {:?} (num_docs in searcher = {})",
            account_id,
            mailbox_id,
            result,
            searcher.num_docs()
        );
        Ok(result)
    }

    pub fn get_account_stats(&self, account_id: u64) -> MailboxdResult<AccountStats> {
        let searcher = self.create_searcher()?;
        let query = self.account_query(account_id);

        let agg_req: Aggregations = serde_json::from_value(json!({
            "total_count": {
                "value_count": {
                    "field": F_ID
                }
            },
            "total_size": {
                "sum": {
                    "field": F_SIZE
                }
            }
        }))
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        let collector = AggregationCollector::from_aggs(agg_req, Default::default());
        let agg_res = searcher
            .search(query.as_ref(), &collector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        let mut stats = AccountStats::default();

        stats.total_count = Self::extract_value_count(&agg_res, "total_count")?;

        let result = agg_res.0.get("total_size").ok_or_else(|| {
            raise_error!(
                "missing 'total_size' aggregation result".into(),
                ErrorCode::InternalError
            )
        })?;

        if let AggregationResult::MetricResult(MetricResult::Sum(v)) = result {
            stats.total_size = v.value.map(|v| v as u64).ok_or_else(|| {
                raise_error!(
                    "'total_size' sum metric has no value".into(),
                    ErrorCode::InternalError
                )
            })?;
        }
        Ok(stats)
    }

    fn extract_max_uid(agg_res: &AggregationResults) -> Option<u64> {
        agg_res.0.get("max_uid").and_then(|result| match result {
            AggregationResult::MetricResult(MetricResult::Max(max)) => {
                max.value.and_then(|value| {
                    (value >= 0.0 && value <= u64::MAX as f64).then(|| value.trunc() as u64)
                })
            }
            _ => None,
        })
    }

    pub async fn delete_account_envelopes(&self, account_id: u64) -> MailboxdResult<()> {
        let query = self.account_query(account_id);
        let (eml_content_hashes, attachments_content_hashes) =
            self.collect_content_hashes(query)?;

        let query = self.account_query(account_id);

        let mut writer = self.index_writer.lock().await;
        writer
            .delete_query(query)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        writer
            .commit()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        ATTACHMENT_MANAGER
            .delete_account_attachments(account_id)
            .await?;

        if !eml_content_hashes.is_empty() || !attachments_content_hashes.is_empty() {
            self.cleanup_unused_content(
                &mut writer,
                eml_content_hashes,
                attachments_content_hashes,
            )?;
        }

        DEDUP_CACHE.remove_by_account(account_id);
        Ok(())
    }

    pub async fn delete_mailbox_envelopes(
        &self,
        account_id: u64,
        mailbox_ids: Vec<u64>,
    ) -> MailboxdResult<()> {
        if mailbox_ids.is_empty() {
            return Ok(());
        }

        let mut eml_content_hashes: HashSet<String> = HashSet::new();
        let mut attachments_content_hashes: HashSet<String> = HashSet::new();

        for mailbox_id in &mailbox_ids {
            let query = self.mailbox_query(account_id, *mailbox_id);
            let (eml_hashes, attachment_hashes) = self.collect_content_hashes(query)?;
            eml_content_hashes.extend(eml_hashes);
            attachments_content_hashes.extend(attachment_hashes);
        }

        let mut queries: Vec<Box<dyn Query>> = Vec::with_capacity(mailbox_ids.len());
        for mailbox_id in &mailbox_ids {
            queries.push(self.mailbox_query(account_id, *mailbox_id));
        }
        let mut writer = self.index_writer.lock().await;
        for query in queries {
            writer
                .delete_query(query)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        }
        writer
            .commit()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        if !eml_content_hashes.is_empty() || !attachments_content_hashes.is_empty() {
            self.cleanup_unused_content(
                &mut writer,
                eml_content_hashes,
                attachments_content_hashes,
            )?;
        }

        for mailbox_id in mailbox_ids {
            DEDUP_CACHE.remove_by_mailbox(mailbox_id);
        }

        Ok(())
    }

    fn collect_content_hashes(
        &self,
        query: Box<dyn Query>,
    ) -> MailboxdResult<(HashSet<String>, HashSet<String>)> {
        let (eml_with_mailbox, attachments_content_hashes) =
            self.collect_content_hashes_with_mailbox(query)?;

        let eml_content_hashes = eml_with_mailbox
            .into_iter()
            .map(|(hash, _mailbox_id)| hash)
            .collect();

        Ok((eml_content_hashes, attachments_content_hashes))
    }

    fn collect_content_hashes_with_mailbox(
        &self,
        query: Box<dyn Query>,
    ) -> MailboxdResult<(HashSet<(String, u64)>, HashSet<String>)> {
        let mut eml_content_hashes = HashSet::new();
        let mut attachments_content_hashes = HashSet::new();

        let fields = SchemaTools::email_fields();
        let searcher = self.create_searcher()?;

        let docs = searcher
            .search(&query, &DocSetCollector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        for doc_address in docs {
            let doc = searcher
                .doc::<TantivyDocument>(doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

            let mailbox_id = doc.get_first(fields.f_mailbox_id).and_then(|v| v.as_u64());

            // Extract content_hash
            if let Some(content_hash_value) = doc.get_first(fields.f_content_hash) {
                if let (Some(hash_str), Some(mailbox_id)) =
                    (content_hash_value.as_str(), mailbox_id)
                {
                    eml_content_hashes.insert((hash_str.to_string(), mailbox_id));
                }
            }

            // Extract attachment_content_hash
            let attachment_hash_values = doc.get_all(fields.f_attachment_content_hash);
            for hash_value in attachment_hash_values {
                if let Some(str) = hash_value.as_str() {
                    attachments_content_hashes.insert(str.to_string());
                }
            }
        }

        Ok((eml_content_hashes, attachments_content_hashes))
    }

    fn cleanup_unused_content(
        &self,
        writer: &mut IndexWriter,
        eml_content_hashes: HashSet<String>,
        attachments_content_hashes: HashSet<String>,
    ) -> MailboxdResult<()> {
        // Reference-count barrier: commit the writer and reload the reader so the
        // `Count` below is evaluated against a fully committed, freshly-reloaded
        // index state. Without this, an envelope that shares a content hash but
        // is still sitting uncommitted in the writer buffer (e.g. added by the
        // background ingest task before this delete acquired the writer lock)
        // would be invisible to the searcher, the count would read 0, and a
        // still-referenced blob would be deleted.
        fatal_commit(writer);
        let searcher = self.create_searcher()?;
        let to_delete =
            unreferenced_hashes(&searcher, eml_content_hashes, attachments_content_hashes)?;
        BLOB_MANAGER.delete(&to_delete, std::iter::empty::<&str>())
    }

    /// Deletes the given envelopes, keyed by account id, and returns how many
    /// of them existed. Nothing is written when none of them exist.
    pub async fn delete_envelopes_multi_account(
        &self,
        deletes: HashMap<u64, Vec<String>>,
    ) -> MailboxdResult<u64> {
        if deletes.is_empty() {
            tracing::warn!("delete_envelopes_multi_account: deletes is empty, nothing to delete");
            return Ok(0);
        }
        let matched = count_matching_envelopes(&self.create_searcher()?, &deletes)?;
        if matched == 0 {
            return Ok(0);
        }

        let mut eml_content_hash_triples: HashSet<(u64, u64, String)> = HashSet::new();
        let mut attachments_content_hashes: HashSet<String> = HashSet::new();

        for (account_id, envelope_ids) in &deletes {
            let unique_ids: HashSet<&String> = envelope_ids.iter().collect();
            if unique_ids.is_empty() {
                continue;
            }

            for eid in unique_ids {
                let query = self.envelope_query(*account_id, eid);
                let (eml_hashes_with_mailbox, attachment_hashes) =
                    self.collect_content_hashes_with_mailbox(query)?;

                eml_content_hash_triples.extend(
                    eml_hashes_with_mailbox
                        .into_iter()
                        .map(|(hash, mailbox_id)| (*account_id, mailbox_id, hash)),
                );
                attachments_content_hashes.extend(attachment_hashes);
            }
        }

        let mut writer = self.index_writer.lock().await;

        for (account_id, envelope_ids) in deletes {
            let unique_ids: HashSet<&String> = envelope_ids.iter().collect();
            if unique_ids.is_empty() {
                continue;
            }
            for eid in unique_ids {
                let query = self.envelope_query(account_id, eid);
                writer
                    .delete_query(query)
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            }
        }
        writer
            .commit()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        if !eml_content_hash_triples.is_empty() || !attachments_content_hashes.is_empty() {
            let eml_content_hashes: HashSet<String> = eml_content_hash_triples
                .iter()
                .map(|(_, _, hash)| hash.clone())
                .collect();

            self.cleanup_unused_content(
                &mut writer,
                eml_content_hashes,
                attachments_content_hashes,
            )?;
        }

        for (aid, mid, hash) in eml_content_hash_triples {
            DEDUP_CACHE.remove(aid, mid, &hash);
        }

        Ok(matched)
    }

    /// Ids of up to `limit` envelopes of `account_id` that are older than
    /// `cutoff_ms` (see [`retention_expired_query`]).
    pub fn retention_expired_ids(
        &self,
        account_id: u64,
        cutoff_ms: i64,
        limit: usize,
    ) -> MailboxdResult<Vec<String>> {
        expired_envelope_ids(&self.create_searcher()?, account_id, cutoff_ms, limit)
    }

    fn collect_facets_recursive(
        query: &dyn Query,
        searcher: &Searcher,
        parent_facet: &str,
        all_facets: &mut Vec<TagCount>,
    ) -> MailboxdResult<()> {
        let mut facet_collector = FacetCollector::for_field(F_TAGS);
        facet_collector.add_facet(parent_facet);

        let facet_counts = searcher
            .search(query, &facet_collector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        for (facet, count) in facet_counts.get(parent_facet) {
            all_facets.push(TagCount {
                tag: facet.to_string(),
                count,
            });
            Self::collect_facets_recursive(query, searcher, &facet.to_string(), all_facets)?;
        }

        Ok(())
    }

    pub fn get_all_tags(&self, accounts: Option<HashSet<u64>>) -> MailboxdResult<Vec<TagCount>> {
        let searcher = self.reader.searcher();

        let query: Box<dyn Query> = match accounts {
            Some(ref ids) if !ids.is_empty() => {
                let mut subqueries = Vec::new();
                for &id in ids {
                    let term = Term::from_field_u64(SchemaTools::email_fields().f_account_id, id);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>,
                    ));
                }
                Box::new(BooleanQuery::new(subqueries))
            }
            Some(_) => Box::new(EmptyQuery),
            None => Box::new(AllQuery),
        };

        let mut all_facets = Vec::new();
        Self::collect_facets_recursive(&query, &searcher, "/", &mut all_facets)?;
        Ok(all_facets)
    }

    pub fn get_all_contacts(
        &self,
        accounts: Option<HashSet<u64>>,
    ) -> MailboxdResult<HashSet<String>> {
        let searcher = self.create_searcher()?;

        let query: Box<dyn Query> = match accounts {
            Some(ref ids) if !ids.is_empty() => {
                let mut subqueries = Vec::new();
                for &id in ids {
                    let term = Term::from_field_u64(SchemaTools::email_fields().f_account_id, id);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>,
                    ));
                }
                Box::new(BooleanQuery::new(subqueries))
            }
            Some(_) => Box::new(EmptyQuery),
            None => Box::new(AllQuery),
        };

        let mut contacts_set: HashSet<String> = HashSet::new();

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(1_000_000).order_by_score())
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        for (_score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            let contacts = extract_contacts(&doc)?;
            for value in contacts {
                contacts_set.insert(value);
            }
        }
        Ok(contacts_set)
    }

    pub async fn update_envelope_tags(&self, request: TagsRequest) -> MailboxdResult<()> {
        if request.updates.is_empty() {
            tracing::warn!("update_envelope_tags: request is empty, nothing to update");
            return Ok(());
        }
        let searcher = self.create_searcher()?;
        let mut writer = self.index_writer.lock().await;

        let f = SchemaTools::email_fields();
        let f_tags = f.f_tags;
        let f_id = f.f_id;
        let deduplicated_updates: HashMap<u64, HashSet<String>> = request
            .updates
            .into_iter()
            .map(|(account_id, envelope_ids)| (account_id, envelope_ids.into_iter().collect()))
            .collect();

        let mut operations = Vec::new();

        for (account_id, envelope_ids) in &deduplicated_updates {
            for eid in envelope_ids {
                let query = self.envelope_query(*account_id, eid);
                let docs = searcher
                    .search(query.as_ref(), &TopDocs::with_limit(1).order_by_score())
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

                if let Some((_, doc_address)) = docs.first() {
                    let old_doc: TantivyDocument = searcher
                        .doc(*doc_address)
                        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

                    let mut current_tags = document_tags(&old_doc, f_tags);

                    match request.action {
                        TagAction::Add => {
                            for tag in &request.tags {
                                current_tags.insert(tag.clone());
                            }
                        }
                        TagAction::Remove => {
                            for tag in &request.tags {
                                current_tags.remove(tag);
                            }
                        }
                        TagAction::Overwrite => {
                            current_tags = request.tags.iter().cloned().collect();
                        }
                    }

                    // f_body is not stored; take it from the EML in the blob store.
                    let eml = match old_doc.get_first(f.f_content_hash).and_then(|v| v.as_str()) {
                        Some(content_hash) => match BLOB_MANAGER.get_email(content_hash) {
                            Ok(Some(eml)) => Some(eml),
                            Ok(None) => {
                                tracing::warn!(
                                    content_hash,
                                    "EML not found in blob store during tag update"
                                );
                                None
                            }
                            Err(e) => {
                                tracing::warn!(
                                    content_hash,
                                    error = %e,
                                    "Failed to fetch EML during tag update"
                                );
                                None
                            }
                        },
                        None => None,
                    };
                    let new_doc =
                        rebuild_envelope_document(&old_doc, &current_tags, eml.as_deref());

                    let delete_term = Term::from_field_text(f_id, eid);
                    operations.push(UserOperation::Delete(delete_term));
                    operations.push(UserOperation::Add(new_doc));
                }
            }
        }

        writer
            .run(operations)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        // commit
        writer
            .commit()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        Ok(())
    }

    pub fn search(
        &self,
        accounts: Option<HashSet<u64>>,
        filter: EmailSearchFilter,
        page: u64,
        page_size: u64,
        desc: bool,
        sort_by: SortBy,
    ) -> MailboxdResult<DataPage<Envelope>> {
        assert!(page > 0, "Page number must be greater than 0");
        assert!(page_size > 0, "Page size must be greater than 0");
        let query = self.filter_query(accounts, filter)?;
        let searcher = self.create_searcher()?;
        let total = searcher
            .search(&query, &Count)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
            as u64;

        if total == 0 {
            return Ok(DataPage {
                current_page: Some(page),
                page_size: Some(page_size),
                total_items: 0,
                items: vec![],
                total_pages: Some(0),
            });
        }
        let offset = (page - 1) * page_size;
        let total_pages = total.div_ceil(page_size);
        if offset > total {
            return Ok(DataPage {
                current_page: Some(page),
                page_size: Some(page_size),
                total_items: total,
                items: vec![],
                total_pages: Some(total_pages),
            });
        }

        let mailbox_docs = top_docs_page(
            &searcher,
            &query,
            sort_by,
            desc,
            offset as usize,
            page_size as usize,
            F_INTERNAL_DATE,
        )?;

        let mut result = Vec::new();

        for doc_address in mailbox_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            let envelope = EnvelopeWithAttachments::from_tantivy_doc(&doc)?.envelope;
            result.push(envelope);
        }
        Ok(DataPage {
            current_page: Some(page),
            page_size: Some(page_size),
            total_items: total,
            items: result,
            total_pages: Some(total_pages),
        })
    }

    fn create_searcher(&self) -> MailboxdResult<Searcher> {
        self.reader
            .reload()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        Ok(self.reader.searcher())
    }

    pub fn num_messages_in_thread(
        &self,
        searcher: &Searcher,
        account_id: u64,
        thread_id: &str,
    ) -> MailboxdResult<u64> {
        let query = self.thread_query(account_id, thread_id);

        let agg_req: Aggregations = serde_json::from_value(json!({
            "thread_count": {
                "value_count": {
                    "field": F_THREAD_ID
                }
            }
        }))
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        let collector = AggregationCollector::from_aggs(agg_req, Default::default());

        let agg_res = searcher
            .search(query.as_ref(), &collector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        Self::extract_value_count(&agg_res, "thread_count")
    }

    fn extract_value_count(agg_res: &AggregationResults, name: &str) -> MailboxdResult<u64> {
        let Some(result) = agg_res.0.get(name) else {
            return Err(raise_error!(
                format!("Missing aggregation result: '{}'", name),
                ErrorCode::InternalError
            ));
        };

        match result {
            AggregationResult::MetricResult(MetricResult::Count(count)) => {
                Ok(count.value.map(|v| v as u64).ok_or_else(|| {
                    raise_error!(
                        "Failed to get count value from aggregation result: value is None".into(),
                        ErrorCode::InternalError
                    )
                })?)
            }
            other => Err(raise_error!(
                format!("Unexpected aggregation result type: {other:?}"),
                ErrorCode::InternalError
            )),
        }
    }

    pub fn list_thread_envelopes(
        &self,
        account_id: u64,
        thread_id: &str,
        page: u64,
        page_size: u64,
        desc: bool,
    ) -> MailboxdResult<DataPage<Envelope>> {
        assert!(page > 0, "Page number must be greater than 0");
        assert!(page_size > 0, "Page size must be greater than 0");
        let searcher = self.create_searcher()?;
        let total = self.num_messages_in_thread(&searcher, account_id, thread_id)?;
        if total == 0 {
            return Ok(DataPage {
                current_page: Some(page),
                page_size: Some(page_size),
                total_items: 0,
                items: vec![],
                total_pages: Some(0),
            });
        }
        let offset = (page - 1) * page_size;
        let total_pages = total.div_ceil(page_size);
        if offset > total {
            return Ok(DataPage {
                current_page: Some(page),
                page_size: Some(page_size),
                total_items: total,
                items: vec![],
                total_pages: Some(total_pages),
            });
        }

        let query = self.thread_query(account_id, thread_id);

        let order = if desc { Order::Desc } else { Order::Asc };
        let thread_docs: Vec<(Option<i64>, DocAddress)> = searcher
            .search(
                query.as_ref(),
                &TopDocs::with_limit(page_size as usize)
                    .and_offset(offset as usize)
                    .order_by_fast_field(F_DATE, order),
            )
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        let mut result = Vec::new();

        for (_, doc_address) in thread_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            let envelope = EnvelopeWithAttachments::from_tantivy_doc(&doc)?.envelope;
            result.push(envelope);
        }
        Ok(DataPage {
            current_page: Some(page),
            page_size: Some(page_size),
            total_items: total,
            items: result,
            total_pages: Some(total_pages),
        })
    }

    pub fn get_dashboard_stats(
        &self,
        accounts: &Option<HashSet<u64>>,
    ) -> MailboxdResult<DashboardStats> {
        let searcher = self.create_searcher()?;
        let now_ms = utc_now!();
        let week_ago_ms = (Utc::now() - Duration::from_secs(60 * 60 * 24 * 30)).timestamp_millis();

        let aggregations: Aggregations = serde_json::from_value(json!({
            "total_size": {
                "sum": { "field": F_SIZE }
            },
            "recent_30d_histogram": {
                "histogram": {
                    "field": F_DATE,
                    "interval": 86400000,
                    "hard_bounds": {
                        "min": week_ago_ms,
                        "max": now_ms
                    }
                }
            },
            "top_from_values": {
                "terms": {
                    "field": F_FROM,
                    "size": 10
                }
            },
            "top_account_values": {
                "terms": {
                    "field": F_ACCOUNT_ID,
                    "size": 10
                }
            },
            "attachment_stats": {
                "range": {
                    "field": F_REGULAR_ATTACHMENT_COUNT,
                    "ranges": [
                        { "to": 1, "key": "no_attachment" },
                        { "from": 1, "key": "has_attachment" }
                    ]
                }
            }
        }))
        .unwrap();

        let query: Box<dyn Query> = match accounts {
            Some(ref ids) if !ids.is_empty() => {
                let mut subqueries = Vec::new();
                for &id in ids {
                    let term = Term::from_field_u64(SchemaTools::email_fields().f_account_id, id);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>,
                    ));
                }
                Box::new(BooleanQuery::new(subqueries))
            }
            Some(_) => Box::new(EmptyQuery),
            None => Box::new(AllQuery),
        };

        let agg_collector = AggregationCollector::from_aggs(aggregations, Default::default());
        let agg_results = searcher
            .search(&query, &agg_collector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        let mut stats = DashboardStats::default();
        let total_size = agg_results.0.get("total_size").ok_or_else(|| {
            raise_error!(
                "missing 'total_size' aggregation result".into(),
                ErrorCode::InternalError
            )
        })?;

        if let AggregationResult::MetricResult(MetricResult::Sum(v)) = total_size {
            let total_size = v.value.map(|v| v as u64).ok_or_else(|| {
                raise_error!(
                    "'total_size' sum metric has no value".into(),
                    ErrorCode::InternalError
                )
            })?;
            stats.total_size_bytes = total_size;
        }

        let recent_30d_histogram = agg_results.0.get("recent_30d_histogram").ok_or_else(|| {
            raise_error!(
                "missing 'recent_30d_histogram' aggregation result".into(),
                ErrorCode::InternalError
            )
        })?;

        let mut recent_activity = Vec::with_capacity(31);
        if let AggregationResult::BucketResult(BucketResult::Histogram { buckets, .. }) =
            recent_30d_histogram
        {
            if let BucketEntries::Vec(bucket_list) = buckets {
                for entry in bucket_list {
                    if let Key::F64(ms) = entry.key {
                        recent_activity.push(TimeBucket {
                            timestamp_ms: ms as i64,
                            count: entry.doc_count,
                        });
                    }
                }
            }
        }
        stats.recent_activity = recent_activity;
        let mut top_senders = Vec::with_capacity(11);
        let top_from_values = agg_results.0.get("top_from_values").unwrap();
        if let AggregationResult::BucketResult(BucketResult::Terms { buckets, .. }) =
            top_from_values
        {
            for entry in buckets {
                if let Key::Str(sender) = &entry.key {
                    top_senders.push(Group {
                        key: sender.clone(),
                        count: entry.doc_count,
                        account_id: None,
                    });
                }
            }
        }
        stats.top_senders = top_senders;

        let mut top_accounts = Vec::with_capacity(11);
        let top_account_values = agg_results.0.get("top_account_values").unwrap();
        if let AggregationResult::BucketResult(BucketResult::Terms { buckets, .. }) =
            top_account_values
        {
            for entry in buckets {
                if let Some(account_id) = account_id_of(&entry.key) {
                    match AccountModel::get(account_id) {
                        Ok(account) => {
                            top_accounts.push(Group {
                                key: account.email,
                                account_id: Some(account_id),
                                count: entry.doc_count,
                            });
                        }
                        Err(e) => {
                            warn!(
                                account_id = account_id,
                                error = %e,
                                "orphaned account index detected, scheduling cleanup"
                            );
                            tokio::spawn(async move {
                                if let Err(e) =
                                    ENVELOPE_MANAGER.delete_account_envelopes(account_id).await
                                {
                                    tracing::error!(
                                        account_id = account_id,
                                        error = %e,
                                        "failed to cleanup envelope index"
                                    );
                                }
                            });
                        }
                    }
                }
            }
        }
        stats.top_accounts = top_accounts;

        let attachment_stats = agg_results.0.get("attachment_stats").unwrap();
        if let AggregationResult::BucketResult(BucketResult::Range { buckets, .. }) =
            attachment_stats
        {
            match buckets {
                BucketEntries::Vec(bucket_vec) => {
                    for entry in bucket_vec {
                        match entry.key.to_string().as_str() {
                            "no_attachment" => {
                                stats.without_attachment_count = entry.doc_count;
                            }
                            "has_attachment" => {
                                stats.with_attachment_count = entry.doc_count;
                            }
                            _ => {}
                        }
                    }
                }
                BucketEntries::HashMap(bucket_map) => {
                    if let Some(entry) = bucket_map.get("no_attachment") {
                        stats.without_attachment_count = entry.doc_count;
                    }
                    if let Some(entry) = bucket_map.get("has_attachment") {
                        stats.with_attachment_count = entry.doc_count;
                    }
                }
            }
        }
        Ok(stats)
    }
}

/// The account id of a terms-aggregation bucket. tantivy reports a u64 key
/// that fits into an i64 as `Key::I64`.
fn account_id_of(key: &Key) -> Option<u64> {
    match key {
        Key::U64(id) => Some(*id),
        Key::I64(id) => u64::try_from(*id).ok(),
        _ => None,
    }
}

/// Return the candidate blob keys that no indexed email references anymore.
///
/// Email and attachment blobs share one key space, so a key can be referenced
/// both as an email content hash and as an attachment content hash (e.g. a
/// nested `.eml` attachment that was also archived standalone). A key is only
/// unreferenced when neither field still uses it.
fn unreferenced_hashes(
    searcher: &Searcher,
    eml_content_hashes: HashSet<String>,
    attachments_content_hashes: HashSet<String>,
) -> MailboxdResult<HashSet<String>> {
    let fields = SchemaTools::email_fields();
    let count = |field, hash: &str| {
        let query = TermQuery::new(Term::from_field_text(field, hash), IndexRecordOption::Basic);
        searcher
            .search(&query, &Count)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
    };

    let mut unreferenced = HashSet::new();
    for hash in eml_content_hashes
        .into_iter()
        .chain(attachments_content_hashes)
    {
        if count(fields.f_content_hash, &hash)? == 0
            && count(fields.f_attachment_content_hash, &hash)? == 0
        {
            unreferenced.insert(hash);
        }
    }
    Ok(unreferenced)
}

fn envelope_query(account_id: u64, eid: &str) -> Box<dyn Query> {
    let account_id_query = TermQuery::new(
        Term::from_field_u64(SchemaTools::email_fields().f_account_id, account_id),
        IndexRecordOption::Basic,
    );
    let envelope_id_query = TermQuery::new(
        Term::from_field_text(SchemaTools::email_fields().f_id, eid),
        IndexRecordOption::Basic,
    );
    Box::new(BooleanQuery::new(vec![
        (Occur::Must, Box::new(account_id_query)),
        (Occur::Must, Box::new(envelope_id_query)),
    ]))
}

/// Envelopes of `account_id` whose effective date, the older of `date` and
/// `internal_date`, is before `cutoff_ms`. A timestamp of 0 is unknown and
/// never counts as old, so mail without any timestamp is kept.
fn retention_expired_query(account_id: u64, cutoff_ms: i64) -> Box<dyn Query> {
    let f = SchemaTools::email_fields();
    let before = |field| -> Box<dyn Query> {
        Box::new(RangeQuery::new(
            Bound::Excluded(Term::from_field_i64(field, 0)),
            Bound::Excluded(Term::from_field_i64(field, cutoff_ms)),
        ))
    };
    let older = BooleanQuery::new(vec![
        (Occur::Should, before(f.f_date)),
        (Occur::Should, before(f.f_internal_date)),
    ]);
    Box::new(BooleanQuery::new(vec![
        (
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_u64(f.f_account_id, account_id),
                IndexRecordOption::Basic,
            )),
        ),
        (Occur::Must, Box::new(older)),
    ]))
}

fn expired_envelope_ids(
    searcher: &Searcher,
    account_id: u64,
    cutoff_ms: i64,
    limit: usize,
) -> MailboxdResult<Vec<String>> {
    let f = SchemaTools::email_fields();
    let docs = searcher
        .search(
            &retention_expired_query(account_id, cutoff_ms),
            &TopDocs::with_limit(limit).order_by_score(),
        )
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    let mut ids = Vec::with_capacity(docs.len());
    for (_, address) in docs {
        let doc: TantivyDocument = searcher
            .doc(address)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        if let Some(id) = doc.get_first(f.f_id).and_then(|v| v.as_str()) {
            ids.push(id.to_string());
        }
    }
    Ok(ids)
}

/// Number of distinct envelopes in `deletes` (keyed by account id) that exist.
fn count_matching_envelopes(
    searcher: &Searcher,
    deletes: &HashMap<u64, Vec<String>>,
) -> MailboxdResult<u64> {
    let mut matched = 0u64;
    for (account_id, envelope_ids) in deletes {
        let unique_ids: HashSet<&String> = envelope_ids.iter().collect();
        for eid in unique_ids {
            let count = searcher
                .search(&envelope_query(*account_id, eid), &Count)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            if count > 0 {
                matched += 1;
            }
        }
    }
    Ok(matched)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tantivy::fields::F_INGEST_AT;
    use serde_json::json;
    use tantivy::{
        collector::Count,
        query::{QueryParser, TermQuery},
        schema::IndexRecordOption,
        Index, Term,
    };

    /// Build a complete test document with known values across all
    /// stored and non-stored fields so we can verify reconstruction.
    fn build_test_doc() -> TantivyDocument {
        let f = SchemaTools::email_fields();
        let mut doc = TantivyDocument::new();

        doc.add_text(f.f_id, "test-eid-001");
        doc.add_text(f.f_message_id, "<test@msg.id>");
        doc.add_u64(f.f_account_id, 1);
        doc.add_u64(f.f_mailbox_id, 10);
        doc.add_u64(f.f_uid, 100);
        doc.add_text(f.f_subject, "Test Subject Line");
        doc.add_text(f.f_body, "the quick brown fox jumps over the lazy dog");
        doc.add_text(f.f_preview, "the quick brown fox...");
        doc.add_text(f.f_content_hash, "test-content-hash-001");
        // f_from / f_from_text carry the same data
        doc.add_text(f.f_from, "alice@example.com");
        doc.add_text(f.f_from_text, "alice@example.com");
        doc.add_text(f.f_to, "bob@example.com");
        doc.add_text(f.f_to_text, "bob@example.com");
        doc.add_text(f.f_cc, "carol@example.com");
        doc.add_text(f.f_cc_text, "carol@example.com");
        doc.add_text(f.f_bcc, "dave@example.com");
        doc.add_text(f.f_bcc_text, "dave@example.com");
        doc.add_i64(f.f_date, 1_700_000_000_000);
        doc.add_i64(f.f_internal_date, 1_700_000_000_000);
        doc.add_i64(f.f_ingest_at, 1_700_000_000_000);
        doc.add_u64(f.f_size, 999);
        doc.add_text(f.f_thread_id, "thread-xyz");

        // Attachment metadata (stored as JSON).
        let atts = json!([{
            "filename": "invoice.pdf",
            "file_type": "application/pdf",
            "inline": false,
            "size": 5000,
            "content_id": null,
            "content_hash": "att-hash-pdf",
            "is_message": false
        }]);
        doc.add_text(f.f_attachments, atts.to_string());
        doc.add_text(f.f_attachment_name_text, "invoice.pdf");
        doc.add_text(f.f_attachment_name_exact, "invoice.pdf");
        doc.add_text(f.f_attachment_ext, "pdf");
        doc.add_text(f.f_attachment_category, "document");
        doc.add_text(f.f_attachment_content_type, "application/pdf");
        doc.add_text(f.f_attachment_content_hash, "att-hash-pdf");
        doc.add_u64(f.f_attachment_count, 1);
        doc.add_u64(f.f_regular_attachment_count, 1);
        doc.add_u64(f.f_shard_id, 0);

        // Initial tags.
        doc.add_facet(f.f_tags, "/inbox");
        doc.add_facet(f.f_tags, "/unread");

        doc
    }

    /// `rebuild_envelope_document` with the EML looked up in `eml_cache`,
    /// a stand-in for the blob store.
    fn reconstruct_for_test(
        old_doc: &TantivyDocument,
        new_tags: &HashSet<String>,
        eml_cache: &HashMap<String, Vec<u8>>,
    ) -> TantivyDocument {
        let f = SchemaTools::email_fields();
        let eml = old_doc
            .get_first(f.f_content_hash)
            .and_then(|v| v.as_str())
            .and_then(|hash| eml_cache.get(hash));
        rebuild_envelope_document(old_doc, new_tags, eml.map(Vec::as_slice))
    }

    #[test]
    fn update_tags_preserves_non_stored_fields() {
        let f = SchemaTools::email_fields();

        // ---- setup: in-memory index + document --------------------
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);

        {
            let mut writer = index
                .writer_with_num_threads(1, 15_000_000)
                .expect("writer");
            let doc = build_test_doc();
            writer.add_document(doc).unwrap();
            writer.commit().unwrap();
        } // drop writer so the next one can acquire the lock

        // ---- build a minimal EML so body reconstruction works ------
        let eml = b"From: alice@example.com\r\n\
                         To: bob@example.com\r\n\
                         Subject: Test\r\n\
                         Date: Thu, 01 Jan 2023 00:00:00 +0000\r\n\
                         Message-ID: <test@msg.id>\r\n\
                         \r\n\
                         the quick brown fox jumps over the lazy dog\r\n";
        let mut eml_cache = HashMap::new();
        eml_cache.insert("test-content-hash-001".to_string(), eml.to_vec());

        // ---- read old doc, reconstruct, delete + add --------------
        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();
        let query = TermQuery::new(
            Term::from_field_text(f.f_id, "test-eid-001"),
            IndexRecordOption::Basic,
        );
        let hits = searcher
            .search(&query, &TopDocs::with_limit(1).order_by_score())
            .unwrap();
        assert_eq!(hits.len(), 1);

        let old_doc: TantivyDocument = searcher.doc(hits[0].1).unwrap();

        let mut new_tags = HashSet::new();
        new_tags.insert("/important".to_string());
        new_tags.insert("/inbox".to_string());

        let new_doc = reconstruct_for_test(&old_doc, &new_tags, &eml_cache);

        let mut writer2 = index
            .writer_with_num_threads(1, 15_000_000)
            .expect("writer2");
        writer2.delete_term(Term::from_field_text(f.f_id, "test-eid-001"));
        writer2.add_document(new_doc).unwrap();
        writer2.commit().unwrap();

        // ---- verify: search for non-stored fields still works -----
        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();

        // Body text (tokenized via "euro")
        let body_parser = QueryParser::for_index(&index, vec![f.f_body]);
        let body_hits = searcher
            .search(&body_parser.parse_query("quick brown fox").unwrap(), &Count)
            .unwrap();
        assert_eq!(body_hits, 1, "body text should survive tag update");

        // from_text (tokenized via "euro")
        let from_parser = QueryParser::for_index(&index, vec![f.f_from_text]);
        let from_hits = searcher
            .search(
                &from_parser.parse_query("alice@example.com").unwrap(),
                &Count,
            )
            .unwrap();
        assert_eq!(from_hits, 1, "from_text should survive tag update");

        // to_text
        let to_parser = QueryParser::for_index(&index, vec![f.f_to_text]);
        let to_hits = searcher
            .search(&to_parser.parse_query("bob@example.com").unwrap(), &Count)
            .unwrap();
        assert_eq!(to_hits, 1, "to_text should survive tag update");

        // attachment_name_exact (STRING — not tokenized)
        let att_hits = searcher
            .search(
                &TermQuery::new(
                    Term::from_field_text(f.f_attachment_name_exact, "invoice.pdf"),
                    IndexRecordOption::Basic,
                ),
                &Count,
            )
            .unwrap();
        assert_eq!(
            att_hits, 1,
            "attachment_name_exact should survive tag update"
        );

        // Updated tags
        let tags_hits = searcher
            .search(
                &TermQuery::new(
                    Term::from_facet(f.f_tags, &Facet::from_text("/important").unwrap()),
                    IndexRecordOption::Basic,
                ),
                &Count,
            )
            .unwrap();
        assert_eq!(tags_hits, 1, "new tag /important should be present");

        // Old tag /unread should be gone since we overwrote with new_tags
        let old_tag_hits = searcher
            .search(
                &TermQuery::new(
                    Term::from_facet(f.f_tags, &Facet::from_text("/unread").unwrap()),
                    IndexRecordOption::Basic,
                ),
                &Count,
            )
            .unwrap();
        assert_eq!(old_tag_hits, 0, "old tag /unread should have been removed");
    }

    #[test]
    fn body_reconstruction_from_eml_cache() {
        // Verify the EML → body_text extraction used by update_envelope_tags.
        let eml = b"From: x@y\r\n\
                         Subject: testing\r\n\
                         Date: Thu, 01 Jan 2023 00:00:00 +0000\r\n\
                         \r\n\
                         hello world from the test suite\r\n";

        let mut cache = HashMap::new();
        cache.insert("hash-abc".to_string(), eml.to_vec());

        let f = SchemaTools::email_fields();
        let mut old = TantivyDocument::new();
        old.add_text(f.f_content_hash, "hash-abc");

        let reconstructed = reconstruct_for_test(&old, &HashSet::new(), &cache);

        // The body should have been extracted from the EML and added
        // back to the document.  Search for it.
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);
        let mut writer = index
            .writer_with_num_threads(1, 15_000_000)
            .expect("writer");
        writer.add_document(reconstructed).unwrap();
        writer.commit().unwrap();

        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();
        let parser = QueryParser::for_index(&index, vec![f.f_body]);
        let hits = searcher
            .search(&parser.parse_query("hello world").unwrap(), &Count)
            .unwrap();
        assert_eq!(hits, 1, "body text should be reconstructed from EML");
    }

    #[test]
    fn body_reconstruction_missing_eml_is_graceful() {
        // When the EML is not in the cache (simulating a blob-store
        // miss), the document should still be produced without body.
        let f = SchemaTools::email_fields();
        let mut old = TantivyDocument::new();
        old.add_text(f.f_content_hash, "nonexistent-hash");

        let cache = HashMap::new(); // empty
        let reconstructed = reconstruct_for_test(&old, &HashSet::new(), &cache);

        // The document exists but has no body field.
        let body_vals: Vec<_> = reconstructed.get_all(f.f_body).collect();
        assert!(
            body_vals.is_empty(),
            "body should be absent when EML is missing"
        );
    }

    // ── get_message_ids_for_mailbox ─────────────────────────────────

    #[test]
    fn get_message_ids_returns_stored_ids() {
        let f = SchemaTools::email_fields();
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);

        // Insert two docs for mailbox 10, one for mailbox 20
        {
            let mut writer = index
                .writer_with_num_threads(1, 15_000_000)
                .expect("writer");

            let mut doc1 = TantivyDocument::new();
            doc1.add_u64(f.f_account_id, 1);
            doc1.add_u64(f.f_mailbox_id, 10);
            doc1.add_text(f.f_message_id, "<msg-a@test>");
            doc1.add_text(f.f_id, "id-a");
            doc1.add_u64(f.f_uid, 1);
            doc1.add_text(f.f_content_hash, "hash-a");
            writer.add_document(doc1).unwrap();

            let mut doc2 = TantivyDocument::new();
            doc2.add_u64(f.f_account_id, 1);
            doc2.add_u64(f.f_mailbox_id, 10);
            doc2.add_text(f.f_message_id, "<msg-b@test>");
            doc2.add_text(f.f_id, "id-b");
            doc2.add_u64(f.f_uid, 2);
            doc2.add_text(f.f_content_hash, "hash-b");
            writer.add_document(doc2).unwrap();

            let mut doc3 = TantivyDocument::new();
            doc3.add_u64(f.f_account_id, 1);
            doc3.add_u64(f.f_mailbox_id, 20);
            doc3.add_text(f.f_message_id, "<msg-c@test>");
            doc3.add_text(f.f_id, "id-c");
            doc3.add_u64(f.f_uid, 3);
            doc3.add_text(f.f_content_hash, "hash-c");
            writer.add_document(doc3).unwrap();

            writer.commit().unwrap();
        }

        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();

        // We can't easily call ENVELOPE_MANAGER.get_message_ids_for_mailbox
        // because it reads from ENVELOPE_MANAGER's own index, not our in-memory one.
        // Instead, test the query pattern directly.
        let query: Box<dyn Query> = {
            let account_query = TermQuery::new(
                Term::from_field_u64(f.f_account_id, 1),
                IndexRecordOption::Basic,
            );
            let mailbox_query = TermQuery::new(
                Term::from_field_u64(f.f_mailbox_id, 10),
                IndexRecordOption::Basic,
            );
            Box::new(BooleanQuery::new(vec![
                (Occur::Must, Box::new(account_query)),
                (Occur::Must, Box::new(mailbox_query)),
            ]))
        };

        let docs = searcher.search(&query, &DocSetCollector).unwrap();

        let mut ids: Vec<String> = Vec::new();
        for addr in docs {
            let doc: TantivyDocument = searcher.doc(addr).unwrap();
            if let Some(v) = doc.get_first(f.f_message_id) {
                if let Some(s) = v.as_str() {
                    ids.push(s.to_string());
                }
            }
        }
        ids.sort();

        assert_eq!(ids, vec!["<msg-a@test>", "<msg-b@test>"]);
    }

    #[test]
    fn count_matching_envelopes_counts_only_existing_ids_per_account() {
        let f = SchemaTools::email_fields();
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);
        {
            let mut writer = index.writer_with_num_threads(1, 15_000_000).unwrap();
            for (account_id, eid) in [(1, "a1"), (1, "a2"), (2, "b1")] {
                let mut doc = TantivyDocument::new();
                doc.add_u64(f.f_account_id, account_id);
                doc.add_u64(f.f_mailbox_id, 10);
                doc.add_text(f.f_id, eid);
                writer.add_document(doc).unwrap();
            }
            writer.commit().unwrap();
        }
        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();

        let deletes = HashMap::from([
            (
                1,
                vec!["a1".to_string(), "a1".to_string(), "missing".to_string()],
            ),
            (2, vec!["b1".to_string()]),
        ]);
        assert_eq!(count_matching_envelopes(&searcher, &deletes).unwrap(), 2);

        // Keyed by a mailbox id instead of the account id: nothing matches.
        let wrong_key = HashMap::from([(10, vec!["a1".to_string()])]);
        assert_eq!(count_matching_envelopes(&searcher, &wrong_key).unwrap(), 0);
    }

    #[test]
    fn retention_query_matches_mail_whose_older_timestamp_is_before_the_cutoff() {
        let f = SchemaTools::email_fields();
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);
        let cutoff = 1_000;
        {
            let mut writer = index.writer_with_num_threads(1, 15_000_000).unwrap();
            for (account_id, eid, date, internal_date) in [
                (1, "both-old", 10, 20),
                (1, "header-old", 10, 5_000),
                (1, "internal-old", 5_000, 10),
                (1, "header-unknown-internal-old", 0, 10),
                (1, "header-unknown-internal-new", 0, 5_000),
                (1, "both-unknown", 0, 0),
                (1, "both-new", 5_000, 5_000),
                (1, "exactly-at-cutoff", 1_000, 1_000),
                (1, "just-before-cutoff", 999, 1_000),
                (2, "other-account-old", 10, 10),
            ] {
                let mut doc = TantivyDocument::new();
                doc.add_u64(f.f_account_id, account_id);
                doc.add_text(f.f_id, eid);
                doc.add_i64(f.f_date, date);
                doc.add_i64(f.f_internal_date, internal_date);
                writer.add_document(doc).unwrap();
            }
            writer.commit().unwrap();
        }
        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();

        let mut ids = expired_envelope_ids(&searcher, 1, cutoff, 100).unwrap();
        ids.sort();
        assert_eq!(
            ids,
            vec![
                "both-old",
                "header-old",
                "header-unknown-internal-old",
                "internal-old",
                "just-before-cutoff",
            ]
        );

        // The limit caps one batch.
        assert_eq!(
            expired_envelope_ids(&searcher, 1, cutoff, 2).unwrap().len(),
            2
        );
        assert_eq!(
            expired_envelope_ids(&searcher, 2, cutoff, 100).unwrap(),
            vec!["other-account-old"]
        );
        assert!(expired_envelope_ids(&searcher, 3, cutoff, 100)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn get_message_ids_empty_mailbox_returns_empty() {
        let f = SchemaTools::email_fields();
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);

        {
            let mut writer = index
                .writer_with_num_threads(1, 15_000_000)
                .expect("writer");

            // Doc for a different mailbox
            let mut doc = TantivyDocument::new();
            doc.add_u64(f.f_account_id, 1);
            doc.add_u64(f.f_mailbox_id, 99);
            doc.add_text(f.f_message_id, "<other@test>");
            doc.add_text(f.f_id, "id-other");
            doc.add_u64(f.f_uid, 1);
            doc.add_text(f.f_content_hash, "hash-other");
            writer.add_document(doc).unwrap();
            writer.commit().unwrap();
        }

        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();

        let query: Box<dyn Query> = {
            let account_query = TermQuery::new(
                Term::from_field_u64(f.f_account_id, 1),
                IndexRecordOption::Basic,
            );
            let mailbox_query = TermQuery::new(
                Term::from_field_u64(f.f_mailbox_id, 10),
                IndexRecordOption::Basic,
            );
            Box::new(BooleanQuery::new(vec![
                (Occur::Must, Box::new(account_query)),
                (Occur::Must, Box::new(mailbox_query)),
            ]))
        };

        let docs = searcher.search(&query, &DocSetCollector).unwrap();
        assert!(docs.is_empty());
    }

    // ── mailbox_contains_message_id ───────────────────────────────

    #[test]
    fn mailbox_contains_message_id_finds_existing() {
        let f = SchemaTools::email_fields();
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);

        {
            let mut writer = index
                .writer_with_num_threads(1, 15_000_000)
                .expect("writer");

            let mut doc = TantivyDocument::new();
            doc.add_u64(f.f_account_id, 1);
            doc.add_u64(f.f_mailbox_id, 10);
            doc.add_text(f.f_message_id, "abc@example.com");
            doc.add_text(f.f_id, "id-1");
            doc.add_u64(f.f_uid, 1);
            doc.add_text(f.f_content_hash, "hash-1");
            writer.add_document(doc).unwrap();
            writer.commit().unwrap();
        }

        // We test the query pattern directly (can't call ENVELOPE_MANAGER
        // which uses a different index).
        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();

        let query = BooleanQuery::new(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(f.f_account_id, 1),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(f.f_mailbox_id, 10),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f.f_message_id, "abc@example.com"),
                    IndexRecordOption::Basic,
                )),
            ),
        ]);

        let count = searcher.search(&query, &Count).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn mailbox_contains_message_id_returns_zero_for_missing() {
        let f = SchemaTools::email_fields();
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);

        {
            let mut writer = index
                .writer_with_num_threads(1, 15_000_000)
                .expect("writer");

            let mut doc = TantivyDocument::new();
            doc.add_u64(f.f_account_id, 1);
            doc.add_u64(f.f_mailbox_id, 10);
            doc.add_text(f.f_message_id, "existing@example.com");
            doc.add_text(f.f_id, "id-1");
            doc.add_u64(f.f_uid, 1);
            doc.add_text(f.f_content_hash, "hash-1");
            writer.add_document(doc).unwrap();
            writer.commit().unwrap();
        }

        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();

        let query = BooleanQuery::new(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(f.f_account_id, 1),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(f.f_mailbox_id, 10),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f.f_message_id, "nonexistent@example.com"),
                    IndexRecordOption::Basic,
                )),
            ),
        ]);

        let count = searcher.search(&query, &Count).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn mailbox_contains_message_id_respects_mailbox_boundary() {
        let f = SchemaTools::email_fields();
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);

        {
            let mut writer = index
                .writer_with_num_threads(1, 15_000_000)
                .expect("writer");

            // Same Message-ID in mailbox 10
            let mut doc1 = TantivyDocument::new();
            doc1.add_u64(f.f_account_id, 1);
            doc1.add_u64(f.f_mailbox_id, 10);
            doc1.add_text(f.f_message_id, "shared@example.com");
            doc1.add_text(f.f_id, "id-1");
            doc1.add_u64(f.f_uid, 1);
            doc1.add_text(f.f_content_hash, "hash-1");
            writer.add_document(doc1).unwrap();

            // Same Message-ID in mailbox 20 (different mailbox)
            let mut doc2 = TantivyDocument::new();
            doc2.add_u64(f.f_account_id, 1);
            doc2.add_u64(f.f_mailbox_id, 20);
            doc2.add_text(f.f_message_id, "shared@example.com");
            doc2.add_text(f.f_id, "id-2");
            doc2.add_u64(f.f_uid, 2);
            doc2.add_text(f.f_content_hash, "hash-2");
            writer.add_document(doc2).unwrap();
            writer.commit().unwrap();
        }

        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();

        // Query mailbox 10: should find 1
        let q10 = BooleanQuery::new(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(f.f_account_id, 1),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(f.f_mailbox_id, 10),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f.f_message_id, "shared@example.com"),
                    IndexRecordOption::Basic,
                )),
            ),
        ]);
        assert_eq!(searcher.search(&q10, &Count).unwrap(), 1);

        // Query mailbox 20: should find 1
        let q20 = BooleanQuery::new(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(f.f_account_id, 1),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(f.f_mailbox_id, 20),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f.f_message_id, "shared@example.com"),
                    IndexRecordOption::Basic,
                )),
            ),
        ]);
        assert_eq!(searcher.search(&q20, &Count).unwrap(), 1);

        // Query mailbox 99 (no docs): should find 0
        let q99 = BooleanQuery::new(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(f.f_account_id, 1),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(f.f_mailbox_id, 99),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f.f_message_id, "shared@example.com"),
                    IndexRecordOption::Basic,
                )),
            ),
        ]);
        assert_eq!(searcher.search(&q99, &Count).unwrap(), 0);
    }

    // ── IMAP UID concept: order_by_fast_field(ingest_at) ─────────────

    #[test]
    fn stable_uid_ordering_by_ingest_at() {
        let f = SchemaTools::email_fields();
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);

        let mailbox_id = 42u64;

        // 10 emails, some sharing the same ingest_at (simulating batch import)
        #[rustfmt::skip]
        let test_data: Vec<(u64, u64, i64)> = vec![
            // (account_id, uid, ingest_at)
            (1, 101, 1_700_000_000_000), // epoch #1
            (1, 102, 1_700_000_000_000), // epoch #1 — same ms, different uid
            (1, 201, 1_700_000_000_100), // epoch #2
            (1, 202, 1_700_000_000_100), // epoch #2 — same ms
            (1, 203, 1_700_000_000_100), // epoch #2 — 3 in same ms
            (1, 301, 1_700_000_000_200),
            (1, 401, 1_700_000_000_300),
            (2, 501, 1_700_000_000_300), // different account, same ms — different mailbox
            (1, 402, 1_700_000_000_400),
            (1, 501, 1_700_000_000_500),
        ];

        {
            let mut writer = index
                .writer_with_num_threads(1, 15_000_000)
                .expect("writer");

            for (i, (account_id, uid, ingest_at)) in test_data.iter().enumerate() {
                let mut doc = TantivyDocument::new();
                doc.add_text(f.f_id, format!("eid-{}", i));
                doc.add_text(f.f_message_id, format!("<msg-{}@test>", i));
                doc.add_u64(f.f_account_id, *account_id);
                doc.add_u64(f.f_mailbox_id, mailbox_id);
                doc.add_u64(f.f_uid, *uid);
                doc.add_i64(f.f_ingest_at, *ingest_at);
                doc.add_i64(f.f_date, *ingest_at);
                doc.add_i64(f.f_internal_date, *ingest_at);
                doc.add_u64(f.f_size, 100);
                doc.add_text(f.f_subject, "test");
                doc.add_text(f.f_preview, "preview");
                doc.add_text(f.f_content_hash, format!("hash-{}", i));
                doc.add_text(f.f_from, "a@b.com");
                doc.add_text(f.f_thread_id, "t1");
                writer.add_document(doc).unwrap();
            }
            writer.commit().unwrap();
        }

        let reader = index.reader().expect("reader");
        let searcher = reader.searcher();

        let query = TermQuery::new(
            Term::from_field_u64(f.f_mailbox_id, mailbox_id),
            IndexRecordOption::Basic,
        );

        // Primary sort: Tantivy fast field on ingest_at
        let top_docs: Vec<(Option<i64>, DocAddress)> = searcher
            .search(
                &query,
                &tantivy::collector::TopDocs::with_limit(100)
                    .order_by_fast_field::<i64>(F_INGEST_AT, tantivy::Order::Asc),
            )
            .unwrap();

        assert_eq!(top_docs.len(), 10, "all 10 docs should be returned");

        // Read each doc, extract (ingest_at, uid) for app-level tie-break
        let mut results: Vec<(i64, u64)> = Vec::new();
        for (_, addr) in &top_docs {
            let doc: TantivyDocument = searcher.doc(*addr).unwrap();
            let ingest_at = doc
                .get_first(f.f_ingest_at)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let uid = doc.get_first(f.f_uid).and_then(|v| v.as_u64()).unwrap_or(0);
            results.push((ingest_at, uid));
        }

        // Verify primary sort by ingest_at is correct
        for w in results.windows(2) {
            assert!(w[0].0 <= w[1].0, "ingest_at must be non-decreasing");
        }

        // Verify deterministic: run again, same order
        let top_docs2: Vec<DocAddress> = searcher
            .search(
                &query,
                &tantivy::collector::TopDocs::with_limit(100)
                    .order_by_fast_field::<i64>(F_INGEST_AT, tantivy::Order::Asc),
            )
            .unwrap()
            .into_iter()
            .map(|(_, addr)| addr)
            .collect();

        for (i, addr) in top_docs.iter().map(|(_, a)| *a).enumerate() {
            assert_eq!(addr, top_docs2[i], "order must be stable across queries");
        }

        // Demonstrate tie-break: same ingest_at groups MUST be ordered by uid
        let mut current_group: Vec<u64> = Vec::new();
        for w in results.windows(2) {
            current_group.push(w[0].1);
            if w[0].0 != w[1].0 {
                current_group.push(w[0].1);
                let mut sorted = current_group.clone();
                sorted.sort();
                assert_eq!(
                    current_group, sorted,
                    "within same ingest_at, uids must be ascending"
                );
                current_group = Vec::new();
            }
        }

        println!("IMAP UID mapping (position → ingest_at, uid):");
        for (pos, (ingest_at, uid)) in results.iter().enumerate() {
            println!(
                "  UID {} → (ingest_at={}, original_uid={})",
                pos + 1,
                ingest_at,
                uid
            );
        }
    }

    fn index_with_docs(docs: &[(&str, &[&str])]) -> Index {
        let f = SchemaTools::email_fields();
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);
        let mut writer = index
            .writer_with_num_threads(1, 15_000_000)
            .expect("writer");
        for (content_hash, attachment_hashes) in docs {
            let mut doc = TantivyDocument::new();
            doc.add_text(f.f_content_hash, content_hash);
            for h in attachment_hashes.iter() {
                doc.add_text(f.f_attachment_content_hash, h);
            }
            writer.add_document(doc).unwrap();
        }
        writer.commit().unwrap();
        index
    }

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn unreferenced_hashes_keeps_email_key_still_used_as_attachment() {
        // "shared" is the content hash of a deleted email, but a remaining
        // email still carries the same blob as a (nested .eml) attachment.
        let index = index_with_docs(&[("other-email", &["shared"])]);
        let searcher = index.reader().unwrap().searcher();
        let result = unreferenced_hashes(&searcher, set(&["shared", "gone"]), set(&[])).unwrap();
        assert_eq!(result, set(&["gone"]));
    }

    #[test]
    fn unreferenced_hashes_keeps_attachment_key_still_used_as_email() {
        let index = index_with_docs(&[("shared", &[])]);
        let searcher = index.reader().unwrap().searcher();
        let result = unreferenced_hashes(&searcher, set(&[]), set(&["shared", "gone"])).unwrap();
        assert_eq!(result, set(&["gone"]));
    }

    #[test]
    fn unreferenced_hashes_keeps_keys_referenced_in_their_own_field() {
        let index = index_with_docs(&[("email-1", &["att-1"])]);
        let searcher = index.reader().unwrap().searcher();
        let result = unreferenced_hashes(
            &searcher,
            set(&["email-1", "email-2"]),
            set(&["att-1", "att-2"]),
        )
        .unwrap();
        assert_eq!(result, set(&["email-2", "att-2"]));
    }
}

/// Regression cases for the search fix (rustmailer/bichon#361 and #215):
/// deterministic word level plus the thresholded trigram layer.
#[cfg(test)]
mod search_fix_tests {
    use super::*;
    use crate::store::tantivy::tokenizers::{text_parser, tri_query};
    use tantivy::{collector::Count, query::QueryParser, Index};

    /// Build an in-memory index with trigram fields and write (subject, body)
    /// messages; the id is the subject to make assertions easy.
    fn build_index(docs: &[(&str, &str)]) -> Index {
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);
        let f = SchemaTools::email_fields();
        {
            let mut writer = index.writer_with_num_threads(1, 15_000_000).unwrap();
            for (subject, body) in docs {
                let mut d = TantivyDocument::new();
                d.add_text(f.f_id, *subject);
                d.add_text(f.f_subject, *subject);
                d.add_text(f.f_subject_tri, *subject);
                d.add_text(f.f_body, *body);
                writer.add_document(d).unwrap();
            }
            writer.commit().unwrap();
        }
        index
    }

    fn count(index: &Index, query: &dyn Query) -> usize {
        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        reader.searcher().search(query, &Count).unwrap()
    }

    /// Ids (i.e. subjects) of the matching messages, sorted.
    fn matching_ids(index: &Index, query: &dyn Query) -> Vec<String> {
        let f = SchemaTools::email_fields();
        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        let searcher = reader.searcher();
        let mut ids: Vec<String> = searcher
            .search(query, &DocSetCollector)
            .unwrap()
            .into_iter()
            .map(|addr| {
                let doc: TantivyDocument = searcher.doc(addr).unwrap();
                doc.get_first(f.f_id).unwrap().as_str().unwrap().to_string()
            })
            .collect();
        ids.sort();
        ids
    }

    fn word_query(index: &Index, field: tantivy::schema::Field, term: &str) -> Box<dyn Query> {
        QueryParser::for_index(index, vec![field])
            .parse_query(term)
            .unwrap()
    }

    /// rustmailer/bichon#361: words in a Danish subject must be findable no matter what
    /// language the whole string is judged to be. This case necessarily fails
    /// under the old whichlang tokenizer (the whole string was judged Danish and
    /// stemmed, the single word was judged another language, so the index term
    /// differed from the query term).
    #[test]
    fn issue_361_danish_subject_is_findable() {
        let index = build_index(&[("VS: Sejlerferie i Kroatien med skipper", "body here")]);
        let f = SchemaTools::email_fields();

        for term in ["Sejlerferie", "Kroatien", "skipper"] {
            assert_eq!(
                count(&index, word_query(&index, f.f_subject, term).as_ref()),
                1,
                "word-level: {term}"
            );
            let combined = word_or_tri(
                word_query(&index, f.f_subject, term),
                &[f.f_subject_tri],
                term,
            );
            assert_eq!(count(&index, combined.as_ref()), 1, "word+tri: {term}");
        }
    }

    /// rustmailer/bichon#215: a body word hits, and a two-word query (both
    /// words in the body) hits.
    #[test]
    fn issue_215_body_word_and_multiword() {
        let index = build_index(&[("Rechnung", "laserdrucker brother toner")]);
        let f = SchemaTools::email_fields();

        assert_eq!(
            count(
                &index,
                word_query(&index, f.f_body, "laserdrucker").as_ref()
            ),
            1
        );
        assert_eq!(
            count(
                &index,
                word_query(&index, f.f_body, "laserdrucker brother").as_ref()
            ),
            1
        );
    }

    /// Morphology coverage across Nordic and Southern-European languages, plus
    /// the short-term precision boundary of the trigram layer.
    ///
    /// Measured behaviour this pins down:
    /// - the word level is exact and accent-insensitive (deunicode folds `café`,
    ///   so `cafe` finds it);
    /// - the trigram layer recovers one level of morphology: Danish definite
    ///   forms (`bil` → `bilen`), Nordic compounds (`ferie` ⊂ `sejlerferie`),
    ///   Romance conjugation (`mangio` → `mangiare`), German/Latin inflection
    ///   (`Kroatiens` → `Kroatien`);
    /// - a 4-character term must NOT over-match: `haus` must not drag in
    ///   `Mauser`, even though both contain the `aus` gram;
    /// - a 3-character term is a pure substring match (a single gram), which is
    ///   intended.
    #[test]
    fn nordic_and_romance_morphology() {
        let index = build_index(&[
            ("Sejlerferie i Kroatien", ""),
            ("bilen er rod", ""),
            ("café con leche", ""),
            ("mangiare la pasta", ""),
            ("Haus am See", ""),
            ("Mauser Gewehr", ""),
        ]);
        let f = SchemaTools::email_fields();
        let subj_tri = [f.f_subject_tri];

        let word = |term: &str| {
            matching_ids(
                &index,
                QueryParser::for_index(&index, vec![f.f_subject])
                    .parse_query(term)
                    .unwrap()
                    .as_ref(),
            )
        };
        let tri = |term: &str| matching_ids(&index, tri_query(&subj_tri, term).as_ref());

        // ── exact word level, accent-insensitive ──
        assert_eq!(word("Kroatien"), ["Sejlerferie i Kroatien"]);
        assert_eq!(
            word("cafe"),
            ["café con leche"],
            "deunicode folds the accent"
        );
        assert_eq!(word("Haus"), ["Haus am See"]);

        // ── trigram recovers one level of morphology ──
        assert_eq!(
            tri("Kroatiens"),
            ["Sejlerferie i Kroatien"],
            "one-level inflection"
        );
        assert_eq!(tri("bil"), ["bilen er rod"], "Danish definite form");
        assert_eq!(tri("ferie"), ["Sejlerferie i Kroatien"], "Nordic compound");
        assert_eq!(tri("mangio"), ["mangiare la pasta"], "Romance conjugation");

        // ── a 4-character term must not over-match ──
        // Two grams means no leave-one-out variant, so both grams are required —
        // i.e. an exact 4-character substring.
        assert_eq!(tri("haus"), ["Haus am See"], "must not drag in Mauser");
        assert_eq!(
            tri("Haus"),
            ["Haus am See"],
            "case-insensitive, same result"
        );

        // ── a 3-character term is a single gram = pure substring match ──
        assert_eq!(
            tri("aus"),
            ["Haus am See", "Mauser Gewehr"],
            "intended substring semantics for 3-char terms"
        );
    }

    /// §8.3 trigram threshold: tolerate one level of inflection, reject noise
    /// driven by a single common gram.
    #[test]
    fn trigram_recall_and_threshold() {
        let index = build_index(&[
            ("Kroatien", ""),
            ("Laserdrucker", ""),
            ("Versandinformation", ""),
        ]);
        let f = SchemaTools::email_fields();
        let subj_tri = [f.f_subject_tri];

        // Inflection: kroatiens and kroatien share 6/7 grams → hit
        assert_eq!(
            matching_ids(&index, tri_query(&subj_tri, "Kroatiens").as_ref()),
            vec!["Kroatien".to_string()]
        );
        // All-gram substring: laserdruck ⊂ laserdrucker → hit
        assert_eq!(
            matching_ids(&index, tri_query(&subj_tri, "laserdruck").as_ref()),
            vec!["Laserdrucker".to_string()]
        );
        // A single shared gram must not hit: laserdruckers and
        // versandinformation share only "ers", so only the genuine one-level
        // inflection target (laserdrucker) should match.
        assert_eq!(
            matching_ids(&index, tri_query(&subj_tri, "laserdruckers").as_ref()),
            vec!["Laserdrucker".to_string()],
            "a single shared gram must not drag in Versandinformation"
        );
    }

    /// rustmailer/bichon#215: a word of an English subject and body is found
    /// on its own.
    #[test]
    fn issue_215_single_word_of_subject_and_body() {
        let index = build_index(&[(
            "Our 11th anniversary offer",
            "Celebrate our anniversary with 20% off",
        )]);
        let f = SchemaTools::email_fields();

        for field in [f.f_subject, f.f_body] {
            assert_eq!(
                count(&index, word_query(&index, field, "anniversary").as_ref()),
                1
            );
        }
    }

    /// Accented words are found with and without the accents.
    #[test]
    fn accented_words_match_folded_queries() {
        let index = build_index(&[("Étoiles filantes ce soir", "")]);
        let f = SchemaTools::email_fields();

        for term in ["Étoiles", "etoiles", "filantes"] {
            assert_eq!(
                count(&index, word_query(&index, f.f_subject, term).as_ref()),
                1,
                "{term}"
            );
        }
    }

    /// Space-separated terms are AND-ed, as in email clients: another word
    /// narrows the result. Explicit `OR`, phrases and exclusion still work.
    #[test]
    fn multiword_query_is_and() {
        let index = build_index(&[
            ("both", "alert for rustmailer git"),
            ("only alert", "alert only"),
        ]);
        let f = SchemaTools::email_fields();
        let word = |query: &str| {
            matching_ids(
                &index,
                text_parser(&index, vec![f.f_body])
                    .parse_query(query)
                    .unwrap()
                    .as_ref(),
            )
        };

        assert_eq!(word("alert"), ["both", "only alert"]);
        assert_eq!(word("alert rustmailer"), ["both"]);
        assert_eq!(word("rustmailer OR alert"), ["both", "only alert"]);
        assert_eq!(word("\"alert for\""), ["both"]);
        assert_eq!(word("alert -rustmailer"), ["only alert"]);
    }

    /// The trigram layer ANDs terms as well, and steps aside for a term too
    /// short for grams instead of making the conjunction false.
    #[test]
    fn trigram_terms_are_and() {
        let index = build_index(&[("Quarterly report", ""), ("Annual report", "")]);
        let f = SchemaTools::email_fields();
        let tri = |query: &str| matching_ids(&index, tri_query(&[f.f_subject_tri], query).as_ref());

        assert_eq!(tri("report"), ["Annual report", "Quarterly report"]);
        assert_eq!(tri("quarterly report"), ["Quarterly report"]);
        assert!(tri("quarterly annual").is_empty());
        assert!(tri("quarterly q4").is_empty(), "q4 has no grams");
    }
}

#[cfg(test)]
#[path = "test_support.rs"]
pub(crate) mod test_support;

/// `IndexManager` behaviour on its own index directory: opening, the search
/// filters, attachment lookup, deletion, tag updates and dashboard stats.
#[cfg(test)]
mod manager_tests {
    use super::test_support::{
        attachment, blob_key, commit_documents, envelope, envelope_document, envelope_manager_at,
        init_globals, register_mailbox, store_email_blob, unique_id,
    };
    use super::*;
    use crate::store::tantivy::format::read_index_format_version;

    struct Fixture {
        _dir: tempfile::TempDir,
        manager: IndexManager,
        account_id: u64,
        mailbox_id: u64,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let manager = envelope_manager_at(dir.path());
            let account_id = unique_id();
            let mailbox_id = unique_id();
            register_mailbox(account_id, mailbox_id, "owner@example.com");
            Self {
                _dir: dir,
                manager,
                account_id,
                mailbox_id,
            }
        }

        fn envelope(&self, id: &str) -> Envelope {
            envelope(self.account_id, self.mailbox_id, id)
        }

        async fn add(&self, envelopes: Vec<(Envelope, Option<Vec<AttachmentInfo>>, &str)>) {
            let docs = envelopes
                .into_iter()
                .map(|(envelope, attachments, body)| envelope_document(envelope, attachments, body))
                .collect();
            commit_documents(&self.manager, docs).await;
        }

        fn search_page(&self, filter: EmailSearchFilter, sort_by: SortBy) -> Vec<Envelope> {
            self.manager
                .search(
                    Some(HashSet::from([self.account_id])),
                    filter,
                    1,
                    50,
                    true,
                    sort_by,
                )
                .unwrap()
                .items
        }

        /// Ids of the matches, sorted.
        fn ids(&self, filter: EmailSearchFilter) -> Vec<String> {
            let mut ids: Vec<String> = self
                .search_page(filter, SortBy::DATE)
                .into_iter()
                .map(|e| e.id)
                .collect();
            ids.sort();
            ids
        }

        async fn tag(&self, ids: &[&str], tag: &str) {
            self.manager
                .update_envelope_tags(TagsRequest {
                    updates: HashMap::from([(
                        self.account_id,
                        ids.iter().map(|id| id.to_string()).collect(),
                    )]),
                    tags: vec![tag.to_string()],
                    action: TagAction::Add,
                })
                .await
                .unwrap();
        }
    }

    fn text(value: &str) -> EmailSearchFilter {
        EmailSearchFilter {
            text: Some(value.into()),
            ..Default::default()
        }
    }

    /// The schema cannot tell which analyzer built the index, so a lost
    /// marker is healed as version 2 and the index asks for a rebuild.
    #[test]
    fn reopening_an_index_with_a_lost_format_marker_asks_for_a_rebuild() {
        let dir = tempfile::tempdir().unwrap();
        drop(envelope_manager_at(dir.path()));
        std::fs::remove_file(dir.path().join("INDEX_FORMAT")).unwrap();

        let reopened = std::panic::catch_unwind(|| drop(envelope_manager_at(dir.path())));

        assert!(
            reopened.is_err(),
            "an index with a healed marker must not open"
        );
        assert_eq!(read_index_format_version(dir.path()), Some(2));
    }

    #[test]
    #[should_panic(expected = "has an outdated format and must be rebuilt")]
    fn reopening_an_outdated_index_panics() {
        let dir = tempfile::tempdir().unwrap();
        drop(envelope_manager_at(dir.path()));
        std::fs::write(dir.path().join("INDEX_FORMAT"), "1\n").unwrap();

        envelope_manager_at(dir.path());
    }

    #[tokio::test]
    async fn text_search_requires_every_word() {
        let fx = Fixture::new();
        let mut a = fx.envelope("a");
        a.subject = "Quarterly report".into();
        let mut b = fx.envelope("b");
        b.subject = "Quarterly plan".into();
        fx.add(vec![
            (a, None, "the numbers are attached"),
            (b, None, "nothing to see"),
        ])
        .await;

        assert_eq!(fx.ids(text("quarterly")), ["a", "b"]);
        assert_eq!(fx.ids(text("quarterly numbers")), ["a"]);
        assert!(fx.ids(text("quarterly budget")).is_empty());
    }

    /// Words are not stemmed; the trigram fields find fragments and
    /// inflections in subjects, addresses and attachment names.
    #[tokio::test]
    async fn text_search_finds_fragments_through_trigram_fields() {
        let fx = Fixture::new();
        let mut a = fx.envelope("a");
        a.subject = "Sejlerferie i Kroatien".into();
        let mut b = fx.envelope("b");
        b.subject = "Hello".into();
        b.to = vec!["zelda@wonderland.test".into()];
        let mut c = fx.envelope("c");
        c.subject = "Scan".into();
        fx.add(vec![
            (a, None, ""),
            (b, None, ""),
            (c, Some(vec![attachment("blueprint.pdf", &blob_key(1))]), ""),
        ])
        .await;

        assert_eq!(fx.ids(text("kroatiens")), ["a"]);
        assert_eq!(fx.ids(text("wonderla")), ["b"]);
        assert_eq!(fx.ids(text("bluepri")), ["c"]);
    }

    /// CJK text has no word spaces; any run of characters in it is found,
    /// and only as an exact substring. `队奖鸡` is "dui jiang ji" in pinyin,
    /// like `对讲机`, but other characters.
    #[tokio::test]
    async fn text_search_finds_cjk_words_inside_unspaced_text() {
        let fx = Fixture::new();
        let mut a = fx.envelope("a");
        a.subject = "回复：Re: Re:Re: 回复：Re: 回复：Re: 洗澡间灯泡更换&对讲机故障沟通".into();
        let mut b = fx.envelope("b");
        b.subject = "天气".into();
        let mut c = fx.envelope("c");
        c.subject = "队奖鸡".into();
        let mut j = fx.envelope("j");
        j.subject = "会議のお知らせ".into();
        let mut k = fx.envelope("k");
        k.subject = "회의 안내입니다".into();
        fx.add(vec![
            (a, None, ""),
            (b, None, "今天的风简直难以置信的强劲"),
            (c, None, ""),
            (j, None, ""),
            (k, None, ""),
        ])
        .await;

        assert_eq!(fx.ids(text("对讲机")), ["a"]);
        assert_eq!(fx.ids(text("灯泡")), ["a"]);
        assert_eq!(fx.ids(text("故障")), ["a"]);
        assert_eq!(fx.ids(text("对讲机故障沟通")), ["a"]);
        assert!(fx.ids(text("对讲沟通")).is_empty(), "not a substring");
        assert_eq!(fx.ids(text("难以置信")), ["b"], "body");
        assert_eq!(fx.ids(text("风")), ["b"], "single character");
        assert_eq!(fx.ids(text("队奖鸡")), ["c"], "no pinyin collision");
        assert!(fx.ids(text("dui jiang ji")).is_empty(), "no pinyin");
        assert_eq!(fx.ids(text("お知らせ")), ["j"]);
        assert_eq!(fx.ids(text("会議")), ["j"]);
        assert_eq!(fx.ids(text("안내")), ["k"]);
        assert_eq!(fx.ids(text("灯泡 Re")), ["a"], "mixed query");
    }

    #[tokio::test]
    async fn subject_and_body_filters_look_only_at_their_field() {
        let fx = Fixture::new();
        let mut s = fx.envelope("in-subject");
        s.subject = "Newsletter September".into();
        let mut b = fx.envelope("in-body");
        b.subject = "Hello".into();
        fx.add(vec![
            (s, None, "plain text"),
            (b, None, "read our newsletter"),
        ])
        .await;

        let subject = |v: &str| EmailSearchFilter {
            subject: Some(v.into()),
            ..Default::default()
        };
        let body = |v: &str| EmailSearchFilter {
            body: Some(v.into()),
            ..Default::default()
        };
        assert_eq!(fx.ids(subject("newsletter")), ["in-subject"]);
        assert_eq!(fx.ids(subject("newslett")), ["in-subject"], "fragment");
        assert_eq!(fx.ids(body("newsletter")), ["in-body"]);
    }

    #[tokio::test]
    async fn address_filters_match_their_own_fields() {
        let fx = Fixture::new();
        let mut x = fx.envelope("x");
        x.from = "alice@alpha.test".into();
        x.to = vec!["bob@beta.test".into()];
        x.cc = vec!["carol@gamma.test".into()];
        x.bcc = vec!["dave@delta.test".into()];
        let mut y = fx.envelope("y");
        y.from = "dave@delta.test".into();
        y.to = vec!["alice@alpha.test".into()];
        y.bcc = vec!["carol@gamma.test".into()];
        fx.add(vec![(x, None, ""), (y, None, "")]).await;

        let filter = |f: fn(&mut EmailSearchFilter) -> &mut Option<String>, v: &str| {
            let mut filter = EmailSearchFilter::default();
            *f(&mut filter) = Some(v.into());
            fx.ids(filter)
        };
        assert_eq!(filter(|f| &mut f.from, "alice"), ["x"]);
        assert_eq!(filter(|f| &mut f.to, "alice"), ["y"]);
        assert_eq!(filter(|f| &mut f.cc, "gamm"), ["x"], "cc fragment");
        assert_eq!(filter(|f| &mut f.bcc, "carol"), ["y"]);
        assert_eq!(filter(|f| &mut f.bcc, "dave"), ["x"]);
        assert_eq!(filter(|f| &mut f.bcc, "delt"), ["x"], "bcc fragment");

        assert_eq!(filter(|f| &mut f.any_recipient, "alice"), ["y"]);
        assert_eq!(filter(|f| &mut f.any_recipient, "carol"), ["x", "y"]);
        assert_eq!(filter(|f| &mut f.any_recipient, "bob"), ["x"]);
        assert_eq!(filter(|f| &mut f.any_participant, "alice"), ["x", "y"]);
        assert_eq!(filter(|f| &mut f.any_participant, "delt"), ["x", "y"]);
        assert_eq!(filter(|f| &mut f.any_participant, "beta"), ["x"]);
    }

    #[tokio::test]
    async fn attachment_name_filter_matches_whole_names_and_fragments() {
        let fx = Fixture::new();
        let x = fx.envelope("x");
        let y = fx.envelope("y");
        fx.add(vec![
            (
                x,
                Some(vec![attachment("invoice-2024.pdf", &blob_key(1))]),
                "",
            ),
            (y, Some(vec![attachment("holiday.png", &blob_key(2))]), ""),
        ])
        .await;

        let name = |v: &str| EmailSearchFilter {
            attachment_name: Some(v.into()),
            ..Default::default()
        };
        assert_eq!(fx.ids(name("invoice-2024.pdf")), ["x"]);
        assert_eq!(fx.ids(name("holiday.png")), ["y"]);
        assert_eq!(fx.ids(name("invoic")), ["x"]);
        assert_eq!(fx.ids(name("holidays")), ["y"], "inflection");
    }

    #[tokio::test]
    async fn attachment_name_filter_finds_cjk_words_in_names() {
        let fx = Fixture::new();
        let x = fx.envelope("x");
        let y = fx.envelope("y");
        fx.add(vec![
            (
                x,
                Some(vec![attachment("灯泡发票2024.pdf", &blob_key(1))]),
                "",
            ),
            (
                y,
                Some(vec![attachment("对讲机说明书.pdf", &blob_key(2))]),
                "",
            ),
        ])
        .await;

        let name = |v: &str| EmailSearchFilter {
            attachment_name: Some(v.into()),
            ..Default::default()
        };
        assert_eq!(fx.ids(name("发票")), ["x"]);
        assert_eq!(fx.ids(name("说明")), ["y"]);
        assert_eq!(fx.ids(text("对讲机")), ["y"], "free text");
    }

    #[tokio::test]
    async fn search_by_relevance_puts_the_best_match_first() {
        let fx = Fixture::new();
        let mut best = fx.envelope("best");
        best.subject = "Welcome to Primagaz".into();
        best.date = 1_000;
        let mut passing = fx.envelope("passing");
        passing.subject = "September newsletter".into();
        passing.date = 2_000;
        fx.add(vec![
            (best, None, "Your Primagaz account is ready."),
            (
                passing,
                None,
                "News from our partners, offers, events, a note about Primagaz, \
                 the weather, recipes, travel tips and much more for the season.",
            ),
        ])
        .await;

        let order = |sort_by| -> Vec<String> {
            fx.search_page(text("primagaz"), sort_by)
                .into_iter()
                .map(|e| e.id)
                .collect()
        };
        assert_eq!(order(SortBy::DATE), ["passing", "best"]);
        assert_eq!(order(SortBy::Relevance), ["best", "passing"]);
    }

    #[tokio::test]
    async fn stored_attachments_of_returns_the_envelope_attachments() {
        let fx = Fixture::new();
        fx.add(vec![
            (
                fx.envelope("with"),
                Some(vec![attachment("report.pdf", &blob_key(1))]),
                "",
            ),
            (fx.envelope("without"), None, ""),
        ])
        .await;

        let with = fx
            .manager
            .stored_attachments_of(fx.account_id, "with")
            .unwrap()
            .expect("envelope exists");
        assert_eq!(with.len(), 1);
        assert_eq!(with[0].filename.as_deref(), Some("report.pdf"));
        assert_eq!(
            fx.manager
                .stored_attachments_of(fx.account_id, "without")
                .unwrap(),
            Some(vec![])
        );
        assert_eq!(
            fx.manager
                .stored_attachments_of(fx.account_id, "missing")
                .unwrap(),
            None
        );
        assert_eq!(
            fx.manager
                .stored_attachments_of(fx.account_id + 1, "with")
                .unwrap(),
            None,
            "other account"
        );
    }

    #[tokio::test]
    async fn deleting_nothing_reports_zero() {
        let fx = Fixture::new();
        assert_eq!(
            fx.manager
                .delete_envelopes_multi_account(HashMap::new())
                .await
                .unwrap(),
            0
        );
    }

    /// Deleting envelopes reports how many existed and drops the blobs no
    /// remaining envelope references.
    #[tokio::test(flavor = "multi_thread")]
    async fn deleting_envelopes_drops_only_unreferenced_blobs() {
        init_globals();
        let fx = Fixture::new();
        let own = blob_key(unique_id());
        let shared = blob_key(unique_id());
        store_email_blob(&own, b"Subject: own\r\n\r\nown\r\n").await;
        store_email_blob(&shared, b"Subject: shared\r\n\r\nshared\r\n").await;
        let mut d1 = fx.envelope("d1");
        d1.content_hash = own.clone();
        let mut d2 = fx.envelope("d2");
        d2.content_hash = shared.clone();
        let mut d3 = fx.envelope("d3");
        d3.content_hash = shared.clone();
        fx.add(vec![(d1, None, ""), (d2, None, ""), (d3, None, "")])
            .await;

        let deleted = fx
            .manager
            .delete_envelopes_multi_account(HashMap::from([(
                fx.account_id,
                vec!["d1".to_string(), "d2".to_string(), "unknown".to_string()],
            )]))
            .await
            .unwrap();

        assert_eq!(deleted, 2);
        assert_eq!(fx.ids(EmailSearchFilter::default()), ["d3"]);
        assert!(BLOB_MANAGER.get_email(&own).unwrap().is_none());
        assert!(BLOB_MANAGER.get_email(&shared).unwrap().is_some());
    }

    fn tags_of(fx: &Fixture, id: &str) -> Vec<String> {
        let filter = EmailSearchFilter {
            id: Some(id.into()),
            ..Default::default()
        };
        fx.search_page(filter, SortBy::DATE)
            .pop()
            .expect("envelope exists")
            .tags
            .unwrap_or_default()
    }

    fn body_ids(fx: &Fixture, word: &str) -> Vec<String> {
        fx.ids(EmailSearchFilter {
            body: Some(word.into()),
            ..Default::default()
        })
    }

    /// The body is not stored; a tag update takes it from the EML so the
    /// envelope stays findable by its body.
    #[tokio::test(flavor = "multi_thread")]
    async fn tagging_keeps_the_body_searchable_from_the_stored_eml() {
        init_globals();
        let fx = Fixture::new();
        let hash = blob_key(unique_id());
        store_email_blob(&hash, b"Subject: x\r\n\r\nThe zebra crossing\r\n").await;
        let mut e = fx.envelope("e");
        e.content_hash = hash;
        fx.add(vec![(e, None, "The zebra crossing")]).await;

        fx.tag(&["e"], "/project").await;

        assert_eq!(tags_of(&fx, "e"), ["/project"]);
        assert_eq!(body_ids(&fx, "zebra"), ["e"]);
    }

    /// Without its EML, whether missing or under an unreadable key, the
    /// envelope is still tagged; only the body can't be restored.
    #[tokio::test(flavor = "multi_thread")]
    async fn tagging_without_the_eml_still_applies_the_tag() {
        init_globals();
        let fx = Fixture::new();
        let mut missing = fx.envelope("missing");
        missing.content_hash = blob_key(unique_id());
        let mut unreadable = fx.envelope("unreadable");
        unreadable.content_hash = "not-a-blob-key".into();
        fx.add(vec![
            (missing, None, "giraffe"),
            (unreadable, None, "giraffe"),
        ])
        .await;
        assert_eq!(body_ids(&fx, "giraffe"), ["missing", "unreadable"]);

        fx.tag(&["missing", "unreadable"], "/project").await;

        assert_eq!(tags_of(&fx, "missing"), ["/project"]);
        assert_eq!(tags_of(&fx, "unreadable"), ["/project"]);
        assert!(body_ids(&fx, "giraffe").is_empty());
    }

    /// A document without a content hash has no EML to look up; it is still
    /// tagged.
    #[tokio::test]
    async fn tagging_an_envelope_without_content_hash_still_applies_the_tag() {
        let fx = Fixture::new();
        let f = SchemaTools::email_fields();
        let mut doc = TantivyDocument::new();
        doc.add_text(f.f_id, "no-hash");
        doc.add_u64(f.f_account_id, fx.account_id);
        commit_documents(&fx.manager, vec![doc]).await;

        fx.tag(&["no-hash"], "/project").await;

        let tagged = fx
            .manager
            .create_searcher()
            .unwrap()
            .search(
                &TermQuery::new(
                    Term::from_facet(f.f_tags, &Facet::from("/project")),
                    IndexRecordOption::Basic,
                ),
                &Count,
            )
            .unwrap();
        assert_eq!(tagged, 1);
    }

    /// Senders are grouped by address and carry no account id.
    #[tokio::test]
    async fn dashboard_groups_top_senders() {
        let fx = Fixture::new();
        let mut a1 = fx.envelope("a1");
        a1.from = "news@shop.test".into();
        let mut a2 = fx.envelope("a2");
        a2.from = "news@shop.test".into();
        let mut b1 = fx.envelope("b1");
        b1.from = "friend@mail.test".into();
        fx.add(vec![(a1, None, ""), (a2, None, ""), (b1, None, "")])
            .await;

        let stats = fx
            .manager
            .get_dashboard_stats(&Some(HashSet::from([fx.account_id])))
            .unwrap();

        let senders: Vec<(String, u64, Option<u64>)> = stats
            .top_senders
            .into_iter()
            .map(|g| (g.key, g.count, g.account_id))
            .collect();
        assert_eq!(
            senders,
            [
                ("news@shop.test".to_string(), 2, None),
                ("friend@mail.test".to_string(), 1, None),
            ]
        );
    }

    #[tokio::test]
    async fn dashboard_lists_top_accounts_with_their_counts() {
        // An unknown account schedules a cleanup on the global manager.
        init_globals();
        let fx = Fixture::new();
        let second_account = unique_id();
        let second_mailbox = unique_id();
        register_mailbox(second_account, second_mailbox, "second@example.com");
        let unknown_account = unique_id();

        fx.add(vec![
            (fx.envelope("o1"), None, ""),
            (fx.envelope("o2"), None, ""),
            (envelope(second_account, second_mailbox, "s1"), None, ""),
            (envelope(unknown_account, unique_id(), "u1"), None, ""),
        ])
        .await;

        let stats = fx.manager.get_dashboard_stats(&None).unwrap();

        let accounts: Vec<(String, u64, Option<u64>)> = stats
            .top_accounts
            .into_iter()
            .map(|g| (g.key, g.count, g.account_id))
            .collect();
        assert_eq!(
            accounts,
            [
                ("owner@example.com".to_string(), 2, Some(fx.account_id)),
                ("second@example.com".to_string(), 1, Some(second_account)),
            ]
        );
    }

    #[test]
    fn account_ids_are_read_from_signed_and_unsigned_keys() {
        assert_eq!(account_id_of(&Key::I64(42)), Some(42));
        assert_eq!(account_id_of(&Key::U64(u64::MAX)), Some(u64::MAX));
        assert_eq!(account_id_of(&Key::I64(-1)), None);
        assert_eq!(account_id_of(&Key::Str("42".into())), None);
    }
}
