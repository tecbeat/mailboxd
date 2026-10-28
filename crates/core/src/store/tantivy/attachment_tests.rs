//
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

//! `IndexManager` behaviour on its own index directory: opening, the search
//! filters, tag updates and attachment metadata.

use std::path::Path;

use super::*;
use crate::store::tantivy::{
    envelope::{
        test_support::{
            attachment, commit_documents, envelope, envelope_document, init_globals,
            register_mailbox, unique_id,
        },
        ENVELOPE_MANAGER,
    },
    format::{read_index_format_version, CURRENT_INDEX_FORMAT_VERSION},
};

/// An attachment index manager on `dir`, opened (or created) the way the
/// global one opens its directory. It has no background ingest task.
fn attachment_manager_at(dir: &Path) -> IndexManager {
    let index = IndexManager::open_or_create_index(&dir.to_path_buf());
    register_tokenizers(&index);
    let index_writer = index.writer_with_num_threads(1, 15_000_000).unwrap();
    let reader = index.reader().unwrap();
    let (sender, _receiver) = mpsc::channel(1);
    IndexManager {
        index: Arc::new(index),
        index_writer: Arc::new(Mutex::new(index_writer)),
        sender,
        reader,
        handle: Mutex::new(None),
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    manager: IndexManager,
    account_id: u64,
    mailbox_id: u64,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let manager = attachment_manager_at(dir.path());
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

    /// An extracted, text-indexed PDF of envelope `envelope_id`; the id is
    /// the attachment's too.
    fn attachment(&self, envelope_id: &str) -> AttachmentModel {
        AttachmentModel {
            id: envelope_id.to_string(),
            envelope_id: envelope_id.to_string(),
            account_id: self.account_id,
            mailbox_id: self.mailbox_id,
            subject: String::new(),
            content_hash: format!("{:064x}", unique_id()),
            from: "sender@example.com".to_string(),
            date: 1_700_000_000_000,
            ingest_at: 1_700_000_000_000,
            size: 10,
            ext: Some("pdf".to_string()),
            category: "document".to_string(),
            content_type: "application/pdf".to_string(),
            text: Some(String::new()),
            has_text: true,
            is_indexed: true,
            page_count: Some(1),
            name: Some(format!("{envelope_id}.pdf")),
            ..Default::default()
        }
    }

    async fn add(&self, attachments: Vec<AttachmentModel>) {
        self.add_documents(
            attachments
                .into_iter()
                .map(AttachmentModel::into_document)
                .collect(),
        )
        .await;
    }

    async fn add_documents(&self, docs: Vec<TantivyDocument>) {
        let mut writer = self.manager.index_writer.lock().await;
        for doc in docs {
            writer.add_document(doc).unwrap();
        }
        writer.commit().unwrap();
    }

    fn search_page(&self, filter: AttachmentSearchFilter, sort_by: SortBy) -> Vec<String> {
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
            .into_iter()
            .map(|a| a.id)
            .collect()
    }

    /// Ids of the matches, sorted.
    fn ids(&self, filter: AttachmentSearchFilter) -> Vec<String> {
        let mut ids = self.search_page(filter, SortBy::DATE);
        ids.sort();
        ids
    }

    async fn tag(&self, envelope_ids: &[&str], tag: &str) {
        self.manager
            .update_attachment_tags(TagsRequest {
                updates: HashMap::from([(
                    self.account_id,
                    envelope_ids.iter().map(|id| id.to_string()).collect(),
                )]),
                tags: vec![tag.to_string()],
                action: TagAction::Add,
            })
            .await
            .unwrap();
    }

    fn tagged(&self, tag: &str) -> Vec<String> {
        self.ids(AttachmentSearchFilter {
            tags: Some(HashSet::from([tag.to_string()])),
            ..Default::default()
        })
    }
}

fn text(value: &str) -> AttachmentSearchFilter {
    AttachmentSearchFilter {
        text: Some(value.into()),
        ..Default::default()
    }
}

fn has_text() -> AttachmentSearchFilter {
    AttachmentSearchFilter {
        has_text: Some(true),
        ..Default::default()
    }
}

#[test]
fn reopening_an_index_heals_a_lost_format_marker() {
    let dir = tempfile::tempdir().unwrap();
    drop(attachment_manager_at(dir.path()));
    std::fs::remove_file(dir.path().join("INDEX_FORMAT")).unwrap();

    drop(attachment_manager_at(dir.path()));

    assert_eq!(
        read_index_format_version(dir.path()),
        Some(CURRENT_INDEX_FORMAT_VERSION)
    );
}

#[test]
#[should_panic(expected = "has an outdated format and must be rebuilt")]
fn reopening_an_outdated_index_panics() {
    let dir = tempfile::tempdir().unwrap();
    drop(attachment_manager_at(dir.path()));
    std::fs::write(dir.path().join("INDEX_FORMAT"), "1\n").unwrap();

    attachment_manager_at(dir.path());
}

#[tokio::test]
async fn text_search_requires_every_word() {
    let fx = Fixture::new();
    let mut a = fx.attachment("a");
    a.subject = "Quarterly report".into();
    a.text = Some("revenue grew".into());
    let mut b = fx.attachment("b");
    b.subject = "Quarterly plan".into();
    b.text = Some("costs".into());
    fx.add(vec![a, b]).await;

    assert_eq!(fx.ids(text("quarterly")), ["a", "b"]);
    assert_eq!(fx.ids(text("quarterly revenue")), ["a"]);
    assert!(fx.ids(text("quarterly budget")).is_empty());
}

/// Words are not stemmed; the trigram fields find fragments and
/// inflections in subjects, senders and file names.
#[tokio::test]
async fn text_search_finds_fragments_through_trigram_fields() {
    let fx = Fixture::new();
    let mut a = fx.attachment("a");
    a.subject = "Sejlerferie i Kroatien".into();
    let mut b = fx.attachment("b");
    b.from = "zelda@wonderland.test".into();
    let mut c = fx.attachment("c");
    c.name = Some("blueprint.pdf".into());
    fx.add(vec![a, b, c]).await;

    assert_eq!(fx.ids(text("kroatiens")), ["a"]);
    assert_eq!(fx.ids(text("wonderla")), ["b"]);
    assert_eq!(fx.ids(text("bluepri")), ["c"]);
}

/// CJK words are found inside unspaced attachment text, subjects and file
/// names, as exact substrings.
#[tokio::test]
async fn search_finds_cjk_words_in_text_subject_and_name() {
    let fx = Fixture::new();
    let mut t = fx.attachment("t");
    t.text = Some("洗澡间灯泡更换&对讲机故障沟通".into());
    let mut s = fx.attachment("s");
    s.subject = "回复：维修报价".into();
    let mut n = fx.attachment("n");
    n.name = Some("灯泡发票2024.pdf".into());
    fx.add(vec![t, s, n]).await;

    let subject = |v: &str| AttachmentSearchFilter {
        subject: Some(v.into()),
        ..Default::default()
    };
    let name = |v: &str| AttachmentSearchFilter {
        attachment_name: Some(v.into()),
        ..Default::default()
    };
    assert_eq!(fx.ids(text("对讲机")), ["t"]);
    assert_eq!(fx.ids(text("灯泡")), ["n", "t"]);
    assert_eq!(fx.ids(text("报价")), ["s"]);
    assert!(fx.ids(text("灯泡沟通")).is_empty(), "not a substring");
    assert_eq!(fx.ids(subject("维修")), ["s"]);
    assert_eq!(fx.ids(name("发票")), ["n"]);
}

#[tokio::test]
async fn subject_from_and_name_filters_look_only_at_their_field() {
    let fx = Fixture::new();
    let mut s = fx.attachment("s");
    s.subject = "Newsletter September".into();
    let mut f = fx.attachment("f");
    f.from = "newsletter@shop.test".into();
    let mut n = fx.attachment("n");
    n.name = Some("newsletter-2024.pdf".into());
    fx.add(vec![s, f, n]).await;

    let subject = |v: &str| AttachmentSearchFilter {
        subject: Some(v.into()),
        ..Default::default()
    };
    let from = |v: &str| AttachmentSearchFilter {
        from: Some(v.into()),
        ..Default::default()
    };
    let name = |v: &str| AttachmentSearchFilter {
        attachment_name: Some(v.into()),
        ..Default::default()
    };
    assert_eq!(fx.ids(subject("newsletter")), ["s"]);
    assert_eq!(fx.ids(subject("newslett")), ["s"], "fragment");
    assert_eq!(fx.ids(from("newsletter")), ["f"]);
    assert_eq!(fx.ids(from("shop")), ["f"]);
    assert_eq!(fx.ids(name("newsletter-2024.pdf")), ["n"]);
    assert_eq!(fx.ids(name("newslett")), ["n"], "fragment");
}

#[tokio::test]
async fn search_by_relevance_puts_the_best_match_first() {
    let fx = Fixture::new();
    let mut best = fx.attachment("best");
    best.subject = "Primagaz contract".into();
    best.text = Some("Primagaz terms for your Primagaz tank".into());
    best.date = 1_000;
    let mut passing = fx.attachment("passing");
    passing.subject = "September newsletter".into();
    passing.text = Some(
        "News from our partners, offers, events, a note about Primagaz, \
         the weather, recipes, travel tips and much more for the season."
            .into(),
    );
    passing.date = 2_000;
    fx.add(vec![best, passing]).await;

    assert_eq!(
        fx.search_page(text("primagaz"), SortBy::DATE),
        ["passing", "best"]
    );
    assert_eq!(
        fx.search_page(text("primagaz"), SortBy::Relevance),
        ["best", "passing"]
    );
}

/// The text is not stored; a tag update takes it from the owning
/// envelope's attachment metadata, so the attachment stays searchable.
#[tokio::test(flavor = "multi_thread")]
async fn tagged_attachment_stays_searchable_by_its_text() {
    init_globals();
    let fx = Fixture::new();
    let envelope_id = format!("env-{}", unique_id());
    let mut a = fx.attachment(&envelope_id);
    a.text = Some("the platypus manual".into());
    let mut info = attachment("manual.pdf", &a.content_hash);
    info.extracted_text = Some("the platypus manual".into());
    commit_documents(
        &ENVELOPE_MANAGER,
        vec![envelope_document(
            envelope(fx.account_id, fx.mailbox_id, &envelope_id),
            Some(vec![info]),
            "",
        )],
    )
    .await;
    fx.add(vec![a]).await;

    fx.tag(&[&envelope_id], "/project").await;

    assert_eq!(fx.tagged("/project"), [envelope_id.clone()]);
    assert_eq!(fx.ids(text("platypus")), [envelope_id.clone()]);
    assert_eq!(fx.ids(has_text()), [envelope_id]);
}

/// Without the envelope's attachment metadata the text can't be restored,
/// so the rebuilt document no longer claims to have text.
#[tokio::test(flavor = "multi_thread")]
async fn tagging_an_attachment_without_its_envelope_resets_the_text_flags() {
    init_globals();
    let fx = Fixture::new();
    let envelope_id = format!("env-{}", unique_id());
    let mut a = fx.attachment(&envelope_id);
    a.text = Some("the platypus manual".into());
    fx.add(vec![a]).await;
    assert_eq!(fx.ids(has_text()), [envelope_id.clone()]);

    fx.tag(&[&envelope_id], "/project").await;

    assert_eq!(fx.tagged("/project"), [envelope_id.clone()]);
    assert!(fx.ids(text("platypus")).is_empty());
    assert!(fx.ids(has_text()).is_empty());
    let not_indexed = AttachmentSearchFilter {
        has_text: Some(false),
        ..Default::default()
    };
    assert_eq!(fx.ids(not_indexed), [envelope_id]);
}

/// A document without a content hash can't be matched to its envelope's
/// metadata; it is still tagged.
#[tokio::test(flavor = "multi_thread")]
async fn tagging_an_attachment_without_content_hash_still_applies_the_tag() {
    init_globals();
    let fx = Fixture::new();
    let f = SchemaTools::attachment_fields();
    let mut doc = TantivyDocument::new();
    doc.add_text(f.f_id, "no-hash");
    doc.add_text(f.f_envelope_id, "no-hash");
    doc.add_u64(f.f_account_id, fx.account_id);
    fx.add_documents(vec![doc]).await;

    fx.tag(&["no-hash"], "/project").await;

    let searcher = fx.manager.create_searcher().unwrap();
    let tagged = searcher
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

#[tokio::test]
async fn metadata_groups_extensions_categories_and_content_types() {
    let fx = Fixture::new();
    let pdf1 = fx.attachment("pdf1");
    let pdf2 = fx.attachment("pdf2");
    let mut png = fx.attachment("png");
    png.ext = Some("png".into());
    png.category = "image".into();
    png.content_type = "image/png".into();
    fx.add(vec![pdf1, pdf2, png]).await;

    let metadata = fx
        .manager
        .collect_attachment_metadata(Some(HashSet::from([fx.account_id])))
        .unwrap();

    let groups = |groups: Vec<Group>| -> Vec<(String, u64, Option<u64>)> {
        groups
            .into_iter()
            .map(|g| (g.key, g.count, g.account_id))
            .collect()
    };
    assert_eq!(
        groups(metadata.extensions),
        [("pdf".to_string(), 2, None), ("png".to_string(), 1, None)]
    );
    assert_eq!(
        groups(metadata.categories),
        [
            ("document".to_string(), 2, None),
            ("image".to_string(), 1, None)
        ]
    );
    assert_eq!(
        groups(metadata.content_types),
        [
            ("application/pdf".to_string(), 2, None),
            ("image/png".to_string(), 1, None)
        ]
    );
}
