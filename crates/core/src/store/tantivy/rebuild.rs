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

//! Rebuild index documents from their stored fields.
//!
//! Body, address, attachment-name and attachment text are indexed but not
//! stored. Rewriting a document (tag updates) copies the stored fields and
//! must re-derive these, otherwise the rewritten document drops out of
//! full-text search.

use std::collections::HashSet;

use mail_parser::{Message, MessageParser};
use tantivy::{
    schema::{Facet, Field, Value},
    TantivyDocument,
};

use crate::{
    message::content::AttachmentInfo, store::tantivy::schema::SchemaTools,
    utils::html::extract_text,
};

/// Plain body text of a message as indexed in `f_body`: the text part, or
/// the text of the HTML part, with whitespace collapsed.
pub fn message_body_text(message: &Message<'_>) -> String {
    let text = if let Some(text) = message.body_text(0) {
        text.into_owned()
    } else if let Some(html) = message.body_html(0) {
        extract_text(html.into_owned())
    } else {
        String::new()
    };
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Attachment metadata stored as JSON in an envelope document's
/// `f_attachments`, or `None` if absent or unreadable.
pub fn stored_attachments(doc: &TantivyDocument) -> Option<Vec<AttachmentInfo>> {
    let json = doc
        .get_first(SchemaTools::email_fields().f_attachments)?
        .as_str()?;
    match serde_json::from_str(json) {
        Ok(attachments) => Some(attachments),
        Err(error) => {
            tracing::warn!(%error, "Unreadable attachment metadata in envelope document");
            None
        }
    }
}

/// Rebuild an envelope document with `tags`. `eml` is the stored EML the
/// body text is taken from; without it the document has no body text.
pub fn rebuild_envelope_document(
    old: &TantivyDocument,
    tags: &HashSet<String>,
    eml: Option<&[u8]>,
) -> TantivyDocument {
    let f = SchemaTools::email_fields();
    let mut doc = copy_stored_fields(old, f.f_tags);

    for (stored, text) in [
        (f.f_from, f.f_from_text),
        (f.f_to, f.f_to_text),
        (f.f_cc, f.f_cc_text),
        (f.f_bcc, f.f_bcc_text),
    ] {
        for value in old.get_all(stored) {
            if let Some(address) = value.as_str() {
                doc.add_text(text, address);
            }
        }
    }

    // Same rule as `EnvelopeWithAttachments::to_document`.
    for att in stored_attachments(old).unwrap_or_default() {
        if att.is_inline() {
            continue;
        }
        if let Some(filename) = &att.filename {
            doc.add_text(f.f_attachment_name_text, filename);
            doc.add_text(f.f_attachment_name_exact, filename);
        }
    }

    if let Some(message) = eml.and_then(|eml| MessageParser::new().parse(eml)) {
        let body = message_body_text(&message);
        if !body.is_empty() {
            doc.add_text(f.f_body, &body);
        }
    }

    add_tags(&mut doc, f.f_tags, tags);
    doc
}

/// Rebuild an attachment document with `tags`. `extracted_text` is the text
/// stored for this attachment in its envelope's `f_attachments`.
pub fn rebuild_attachment_document(
    old: &TantivyDocument,
    tags: &HashSet<String>,
    extracted_text: Option<&str>,
) -> TantivyDocument {
    let f = SchemaTools::attachment_fields();
    let mut doc = copy_stored_fields(old, f.f_tags);

    if let Some(from) = old.get_first(f.f_from).and_then(|v| v.as_str()) {
        doc.add_text(f.f_from_text, from);
    }
    if let Some(name) = old.get_first(f.f_name_exact).and_then(|v| v.as_str()) {
        doc.add_text(f.f_name_text, name);
    }
    if let Some(text) = extracted_text {
        doc.add_text(f.f_text, text);
    }

    add_tags(&mut doc, f.f_tags, tags);
    doc
}

/// Text extracted from the attachment with `content_hash`, as stored in its
/// envelope's attachment metadata.
pub fn extracted_text_of(attachments: &[AttachmentInfo], content_hash: &str) -> Option<String> {
    attachments
        .iter()
        .find(|att| att.content_hash == content_hash)
        .and_then(|att| att.extracted_text.clone())
}

fn copy_stored_fields(old: &TantivyDocument, tags_field: Field) -> TantivyDocument {
    let mut doc = TantivyDocument::new();
    for (field, value) in old.field_values() {
        if field != tags_field {
            doc.add_field_value(field, value);
        }
    }
    doc
}

fn add_tags(doc: &mut TantivyDocument, tags_field: Field, tags: &HashSet<String>) {
    for tag in tags {
        doc.add_facet(tags_field, tag.as_str());
    }
}

/// Tags of a document's facet field, as paths like `/inbox`. Stored facets
/// come back encoded (`inbox`, `a\0b`) and must be decoded before they can
/// be added to a document again.
pub fn document_tags(doc: &TantivyDocument, tags_field: Field) -> HashSet<String> {
    doc.get_all(tags_field)
        .filter_map(|value| value.as_facet())
        .filter_map(|encoded| Facet::from_encoded(encoded.as_bytes().to_vec()).ok())
        .map(|facet| facet.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tantivy::{schema::SchemaTools, tokenizers::EuroTokenizer};
    use serde_json::json;
    use tantivy::{
        collector::{Count, TopDocs},
        query::{AllQuery, QueryParser, TermQuery},
        schema::{IndexRecordOption, Schema},
        Index, Searcher, Term,
    };

    // Tokens no stemmer changes, so these tests don't depend on the analyzer.
    const EML: &[u8] = b"From: u1@example.com\r\n\
        Subject: Report rv1234\r\n\
        Date: Thu, 01 Jan 2023 00:00:00 +0000\r\n\
        \r\n\
        lorem  qx7731\r\n  ipsum\r\n";

    fn index_one(schema: Schema, doc: TantivyDocument) -> (Index, Searcher) {
        let index = Index::create_in_ram(schema);
        index.tokenizers().register("euro", EuroTokenizer::new());
        let mut writer = index.writer_with_num_threads(1, 15_000_000).unwrap();
        writer.add_document(doc).unwrap();
        writer.commit().unwrap();
        let searcher = index.reader().unwrap().searcher();
        (index, searcher)
    }

    /// `doc` after a round trip through an index, as tag updates read it.
    fn stored(schema: Schema, doc: TantivyDocument) -> TantivyDocument {
        let (_, searcher) = index_one(schema, doc);
        let top = searcher
            .search(&AllQuery, &TopDocs::with_limit(1).order_by_score())
            .unwrap();
        searcher.doc(top[0].1).unwrap()
    }

    fn hits(index: &Index, searcher: &Searcher, field: Field, query: &str) -> usize {
        let parser = QueryParser::for_index(index, vec![field]);
        searcher
            .search(&parser.parse_query(query).unwrap(), &Count)
            .unwrap()
    }

    fn term_hits(searcher: &Searcher, term: Term) -> usize {
        searcher
            .search(&TermQuery::new(term, IndexRecordOption::Basic), &Count)
            .unwrap()
    }

    fn tag_term(field: Field, tag: &str) -> Term {
        Term::from_facet(field, &Facet::from_text(tag).unwrap())
    }

    /// An envelope as originally indexed, including the non-stored fields.
    fn indexed_envelope() -> TantivyDocument {
        let f = SchemaTools::email_fields();
        let mut doc = TantivyDocument::new();
        doc.add_text(f.f_id, "eid-1");
        doc.add_u64(f.f_account_id, 1);
        doc.add_text(f.f_subject, "Report rv1234");
        doc.add_text(f.f_content_hash, "hash-1");
        for (stored, text, address) in [
            (f.f_from, f.f_from_text, "u1@example.com"),
            (f.f_to, f.f_to_text, "u2@example.com"),
            (f.f_cc, f.f_cc_text, "u3@example.com"),
            (f.f_bcc, f.f_bcc_text, "u4@example.com"),
        ] {
            doc.add_text(stored, address);
            doc.add_text(text, address);
        }
        doc.add_text(f.f_body, "lorem qx7731 ipsum");
        let atts = json!([
            {"filename": "inv9921.pdf", "file_type": "application/pdf", "inline": false,
             "size": 5, "content_id": null, "content_hash": "att-pdf", "is_message": false,
             "extracted_text": "total zq4420 due", "extracted_page_count": 1},
            {"filename": "lg5512.png", "file_type": "image/png", "inline": true,
             "size": 5, "content_id": "logo", "content_hash": "att-png", "is_message": false,
             "extracted_text": null, "extracted_page_count": null}
        ]);
        doc.add_text(f.f_attachments, atts.to_string());
        doc.add_text(f.f_attachment_name_text, "inv9921.pdf");
        doc.add_text(f.f_attachment_name_exact, "inv9921.pdf");
        doc.add_facet(f.f_tags, "/inbox");
        doc
    }

    fn stored_envelope() -> TantivyDocument {
        stored(SchemaTools::email_schema(), indexed_envelope())
    }

    fn indexed_attachment() -> TantivyDocument {
        let f = SchemaTools::attachment_fields();
        let mut doc = TantivyDocument::new();
        doc.add_text(f.f_id, "aid-1");
        doc.add_text(f.f_envelope_id, "eid-1");
        doc.add_u64(f.f_account_id, 1);
        doc.add_text(f.f_subject, "Report rv1234");
        doc.add_text(f.f_content_hash, "att-pdf");
        doc.add_text(f.f_from, "u1@example.com");
        doc.add_text(f.f_from_text, "u1@example.com");
        doc.add_text(f.f_name_exact, "inv9921.pdf");
        doc.add_text(f.f_name_text, "inv9921.pdf");
        doc.add_text(f.f_text, "total zq4420 due");
        doc.add_bool(f.f_has_text, true);
        doc.add_facet(f.f_tags, "/old");
        doc
    }

    #[test]
    fn attachment_rebuild_restores_all_searchable_fields() {
        let f = SchemaTools::attachment_fields();
        let old = stored(SchemaTools::attachment_schema(), indexed_attachment());
        let text = extracted_text_of(&stored_attachments(&stored_envelope()).unwrap(), "att-pdf");
        assert_eq!(text.as_deref(), Some("total zq4420 due"));

        let tags = HashSet::from(["/new".to_string()]);
        let doc = rebuild_attachment_document(&old, &tags, text.as_deref());
        let (index, searcher) = index_one(SchemaTools::attachment_schema(), doc);

        assert_eq!(hits(&index, &searcher, f.f_text, "zq4420"), 1);
        assert_eq!(hits(&index, &searcher, f.f_name_text, "inv9921"), 1);
        assert_eq!(hits(&index, &searcher, f.f_from_text, "u1"), 1);
        assert_eq!(hits(&index, &searcher, f.f_subject, "rv1234"), 1);
        assert_eq!(term_hits(&searcher, tag_term(f.f_tags, "/new")), 1);
        assert_eq!(term_hits(&searcher, tag_term(f.f_tags, "/old")), 0);
    }

    #[test]
    fn extracted_text_of_unknown_attachment_is_none() {
        let attachments = stored_attachments(&stored_envelope()).unwrap();
        assert_eq!(extracted_text_of(&attachments, "att-png"), None);
        assert_eq!(extracted_text_of(&attachments, "missing"), None);
    }

    #[test]
    fn envelope_rebuild_restores_all_searchable_fields() {
        let f = SchemaTools::email_fields();
        let tags = HashSet::from(["/important".to_string()]);
        let doc = rebuild_envelope_document(&stored_envelope(), &tags, Some(EML));
        let (index, searcher) = index_one(SchemaTools::email_schema(), doc);

        assert_eq!(hits(&index, &searcher, f.f_body, "qx7731"), 1);
        assert_eq!(hits(&index, &searcher, f.f_subject, "rv1234"), 1);
        for (field, address) in [
            (f.f_from_text, "u1"),
            (f.f_to_text, "u2"),
            (f.f_cc_text, "u3"),
            (f.f_bcc_text, "u4"),
        ] {
            assert_eq!(hits(&index, &searcher, field, address), 1, "{address}");
        }
        assert_eq!(
            hits(&index, &searcher, f.f_attachment_name_text, "inv9921"),
            1
        );
        assert_eq!(
            term_hits(
                &searcher,
                Term::from_field_text(f.f_attachment_name_exact, "inv9921.pdf")
            ),
            1
        );
        // Inline images are not indexed by name, as in `to_document`.
        assert_eq!(
            hits(&index, &searcher, f.f_attachment_name_text, "lg5512"),
            0
        );

        assert_eq!(term_hits(&searcher, tag_term(f.f_tags, "/important")), 1);
        assert_eq!(term_hits(&searcher, tag_term(f.f_tags, "/inbox")), 0);
    }

    #[test]
    fn envelope_rebuild_without_eml_has_no_body() {
        let f = SchemaTools::email_fields();
        let doc = rebuild_envelope_document(&stored_envelope(), &HashSet::new(), None);
        assert_eq!(doc.get_all(f.f_body).count(), 0);
        assert_eq!(doc.get_all(f.f_from_text).count(), 1);
    }

    #[test]
    fn body_text_collapses_whitespace() {
        let message = MessageParser::new().parse(EML).unwrap();
        assert_eq!(message_body_text(&message), "lorem qx7731 ipsum");
    }

    #[test]
    fn body_text_falls_back_to_html() {
        let eml = b"Content-Type: text/html\r\n\r\n<p>lorem <b>qx7731</b></p>\r\n";
        let message = MessageParser::new().parse(&eml[..]).unwrap();
        assert_eq!(message_body_text(&message), "lorem qx7731");
    }

    #[test]
    fn stored_tags_can_be_added_again() {
        for (schema, f_tags) in [
            (
                SchemaTools::email_schema(),
                SchemaTools::email_fields().f_tags,
            ),
            (
                SchemaTools::attachment_schema(),
                SchemaTools::attachment_fields().f_tags,
            ),
        ] {
            let mut doc = TantivyDocument::new();
            doc.add_facet(f_tags, "/inbox");
            doc.add_facet(f_tags, "/project/alpha");

            let tags = document_tags(&stored(schema, doc), f_tags);
            let expected = HashSet::from(["/inbox".to_string(), "/project/alpha".to_string()]);
            assert_eq!(tags, expected);

            let mut again = TantivyDocument::new();
            for tag in &tags {
                again.add_facet(f_tags, tag.as_str());
            }
            assert_eq!(document_tags(&again, f_tags), expected);
        }
    }
}
