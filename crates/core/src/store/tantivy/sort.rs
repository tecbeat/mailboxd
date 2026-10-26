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

//! The page of matches for a search, in the requested order.

use tantivy::{
    collector::TopDocs, fastfield::FastValue, query::Query, DocAddress, Order, Searcher,
};

use crate::{
    error::{code::ErrorCode, MailboxdResult},
    message::search::SortBy,
    raise_error,
    store::tantivy::fields::{F_DATE, F_INGEST_AT, F_SIZE},
};

/// Addresses of the matches on one page. `internal_date_field` is the fast
/// field used for [`SortBy::InternalDate`]; indexes without an IMAP
/// INTERNALDATE pass their own date field. [`SortBy::Relevance`] orders by
/// tantivy's BM25 score, always best match first, so `desc` is ignored.
pub fn top_docs_page(
    searcher: &Searcher,
    query: &dyn Query,
    sort_by: SortBy,
    desc: bool,
    offset: usize,
    limit: usize,
    internal_date_field: &str,
) -> MailboxdResult<Vec<DocAddress>> {
    let order = if desc { Order::Desc } else { Order::Asc };
    match sort_by {
        SortBy::DATE => by_fast_field::<i64>(searcher, query, F_DATE, order, offset, limit),
        SortBy::SIZE => by_fast_field::<u64>(searcher, query, F_SIZE, order, offset, limit),
        SortBy::InternalDate => {
            by_fast_field::<i64>(searcher, query, internal_date_field, order, offset, limit)
        }
        SortBy::IngestAt => {
            by_fast_field::<i64>(searcher, query, F_INGEST_AT, order, offset, limit)
        }
        SortBy::Relevance => {
            let collector = TopDocs::with_limit(limit)
                .and_offset(offset)
                .order_by_score();
            let docs: Vec<(f32, DocAddress)> = searcher
                .search(query, &collector)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            Ok(docs.into_iter().map(|(_, addr)| addr).collect())
        }
    }
}

fn by_fast_field<T: FastValue>(
    searcher: &Searcher,
    query: &dyn Query,
    field: &str,
    order: Order,
    offset: usize,
    limit: usize,
) -> MailboxdResult<Vec<DocAddress>> {
    let collector = TopDocs::with_limit(limit)
        .and_offset(offset)
        .order_by_fast_field::<T>(field, order);
    let docs: Vec<(Option<T>, DocAddress)> = searcher
        .search(query, &collector)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    Ok(docs.into_iter().map(|(_, addr)| addr).collect())
}

#[cfg(test)]
mod tests {
    use tantivy::{schema::Value, Index, TantivyDocument};

    use super::*;
    use crate::store::tantivy::{
        fields::F_INTERNAL_DATE,
        schema::SchemaTools,
        tokenizers::{register_tokenizers, text_parser, word_or_tri},
    };

    /// (id, subject, body, date); the id is returned to make assertions easy.
    fn build_index(docs: &[(&str, &str, &str, i64)]) -> Index {
        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);
        let f = SchemaTools::email_fields();
        let mut writer = index.writer_with_num_threads(1, 15_000_000).unwrap();
        for (id, subject, body, date) in docs {
            let mut doc = TantivyDocument::new();
            doc.add_text(f.f_id, *id);
            doc.add_text(f.f_subject, *subject);
            doc.add_text(f.f_subject_tri, *subject);
            doc.add_text(f.f_body, *body);
            doc.add_i64(f.f_date, *date);
            writer.add_document(doc).unwrap();
        }
        writer.commit().unwrap();
        index
    }

    fn ordered_ids(index: &Index, text: &str, sort_by: SortBy) -> Vec<String> {
        let f = SchemaTools::email_fields();
        // The message search's text query: words, plus the trigram layer.
        let word = text_parser(index, SchemaTools::email_default_fields())
            .parse_query(text)
            .unwrap();
        let query = word_or_tri(word, &[f.f_subject_tri], text);
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        top_docs_page(
            &searcher,
            query.as_ref(),
            sort_by,
            true,
            0,
            10,
            F_INTERNAL_DATE,
        )
        .unwrap()
        .into_iter()
        .map(|addr| {
            let doc: TantivyDocument = searcher.doc(addr).unwrap();
            doc.get_first(f.f_id).unwrap().as_str().unwrap().to_string()
        })
        .collect()
    }

    /// A newer mail that only mentions the term in passing buries the best
    /// match under date order; relevance puts the best match first.
    #[test]
    fn relevance_ranks_the_best_match_first_where_date_buries_it() {
        let index = build_index(&[
            (
                "best",
                "Welcome to Primagaz",
                "Your Primagaz account is ready.",
                1_736_985_600_000,
            ),
            (
                "passing",
                "September newsletter",
                "News from our partners, offers, events, a note about Primagaz, \
                 the weather, recipes, travel tips and much more for the season.",
                1_755_705_600_000,
            ),
        ]);

        assert_eq!(
            ordered_ids(&index, "primagaz", SortBy::DATE),
            ["passing", "best"]
        );
        assert_eq!(
            ordered_ids(&index, "primagaz", SortBy::Relevance),
            ["best", "passing"]
        );
    }

    #[test]
    fn relevance_ignores_desc_and_pages_by_offset() {
        let index = build_index(&[
            ("best", "Primagaz Primagaz", "Primagaz", 1),
            (
                "passing",
                "Newsletter",
                "a long body that mentions primagaz once",
                2,
            ),
        ]);
        let f = SchemaTools::email_fields();
        let query = text_parser(&index, SchemaTools::email_default_fields())
            .parse_query("primagaz")
            .unwrap();
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let id = |addr: DocAddress| {
            let doc: TantivyDocument = searcher.doc(addr).unwrap();
            doc.get_first(f.f_id).unwrap().as_str().unwrap().to_string()
        };

        let ascending = top_docs_page(
            &searcher,
            query.as_ref(),
            SortBy::Relevance,
            false,
            0,
            1,
            F_DATE,
        )
        .unwrap();
        assert_eq!(ascending.into_iter().map(id).collect::<Vec<_>>(), ["best"]);
        let second = top_docs_page(
            &searcher,
            query.as_ref(),
            SortBy::Relevance,
            true,
            1,
            1,
            F_DATE,
        )
        .unwrap();
        assert_eq!(second.into_iter().map(id).collect::<Vec<_>>(), ["passing"]);
    }
}
