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

use std::collections::HashSet;

use tantivy::{
    schema::{Facet, Field, Value},
    TantivyDocument,
};

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
    use tantivy::{collector::TopDocs, query::AllQuery, schema::Schema, Index};

    /// `doc` after a round trip through an index, as tag updates read it.
    fn stored(schema: Schema, doc: TantivyDocument) -> TantivyDocument {
        let index = Index::create_in_ram(schema);
        index.tokenizers().register("euro", EuroTokenizer::new());
        let mut writer = index.writer_with_num_threads(1, 15_000_000).unwrap();
        writer.add_document(doc).unwrap();
        writer.commit().unwrap();
        let searcher = index.reader().unwrap().searcher();
        let top = searcher
            .search(&AllQuery, &TopDocs::with_limit(1).order_by_score())
            .unwrap();
        searcher.doc(top[0].1).unwrap()
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
