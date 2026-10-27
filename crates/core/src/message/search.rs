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

//use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::{
    common::paginated::DataPage,
    error::{code::ErrorCode, MailboxdResult},
    raise_error,
    store::{
        envelope::Envelope,
        tantivy::{
            attachment::ATTACHMENT_MANAGER, envelope::ENVELOPE_MANAGER, model::AttachmentModel,
        },
    },
};

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct EmailSearchFilter {
    pub text: Option<String>,
    pub subject: Option<String>,
    pub id: Option<String>,
    pub body: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub cc: Option<String>,
    pub bcc: Option<String>,
    /// Matches if the address appears in `to`, `cc`, or `bcc` (OR semantics).
    pub any_recipient: Option<String>,
    /// Matches if the address appears in `from`, `to`, `cc`, or `bcc` (OR semantics).
    pub any_participant: Option<String>,
    pub since: Option<i64>,
    pub before: Option<i64>,
    /// Lower bound (inclusive) on the IMAP server INTERNALDATE timestamp.
    pub internal_date_since: Option<i64>,
    /// Upper bound (inclusive) on the IMAP server INTERNALDATE timestamp.
    pub internal_date_before: Option<i64>,
    /// Lower bound (inclusive) on Mailboxd's archival (ingest) timestamp.
    pub ingest_since: Option<i64>,
    /// Upper bound (inclusive) on Mailboxd's archival (ingest) timestamp.
    pub ingest_before: Option<i64>,
    pub account_ids: Option<HashSet<u64>>,
    pub mailbox_ids: Option<HashSet<u64>>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub message_id: Option<String>,
    pub has_attachment: Option<bool>,
    pub attachment_name: Option<String>,
    pub tags: Option<HashSet<String>>,
    pub attachment_extension: Option<String>,
    pub attachment_category: Option<String>,
    pub attachment_content_type: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum SortBy {
    #[default]
    DATE,
    SIZE,
    /// Sort by the IMAP server INTERNALDATE timestamp.
    #[serde(rename = "INTERNAL_DATE")]
    #[cfg_attr(feature = "web-api", oai(rename = "INTERNAL_DATE"))]
    InternalDate,
    /// Sort by Mailboxd's archival (ingest) timestamp.
    #[serde(rename = "INGEST_AT")]
    #[cfg_attr(feature = "web-api", oai(rename = "INGEST_AT"))]
    IngestAt,
    /// Sort by full-text relevance (tantivy's BM25 score), best match first;
    /// `desc` is ignored. Falls back to `DATE` without a text query.
    #[serde(rename = "RELEVANCE")]
    #[cfg_attr(feature = "web-api", oai(rename = "RELEVANCE"))]
    Relevance,
}

/// Date stays the default. Relevance needs a text query to score; without
/// one, every match scores the same and the order would be arbitrary.
fn resolve_sort_by(requested: Option<SortBy>, has_text: bool) -> SortBy {
    match requested {
        Some(SortBy::Relevance) if !has_text => SortBy::DATE,
        Some(sort_by) => sort_by,
        None => SortBy::DATE,
    }
}

impl EmailSearchFilter {
    fn has_text(&self) -> bool {
        self.text.is_some() || self.subject.is_some() || self.body.is_some()
    }
}

impl AttachmentSearchFilter {
    fn has_text(&self) -> bool {
        self.text.is_some() || self.subject.is_some()
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct EmailSearchRequest {
    pub filter: EmailSearchFilter,
    pub page: u64,
    pub page_size: u64,
    pub sort_by: Option<SortBy>,
    pub desc: Option<bool>,
}
impl EmailSearchRequest {
    pub fn validate(&self) -> MailboxdResult<()> {
        if self.page == 0 || self.page_size == 0 {
            return Err(raise_error!(
                "Both page and page_size must be greater than 0.".into(),
                ErrorCode::InvalidParameter
            ));
        }
        if self.page_size > 500 {
            return Err(raise_error!(
                "The page_size exceeds the maximum allowed limit of 500.".into(),
                ErrorCode::InvalidParameter
            ));
        }

        Ok(())
    }
}

pub fn search_messages_impl(
    accounts: Option<HashSet<u64>>,
    request: EmailSearchRequest,
) -> MailboxdResult<DataPage<Envelope>> {
    request.validate()?;
    let sort_by = resolve_sort_by(request.sort_by, request.filter.has_text());
    ENVELOPE_MANAGER.search(
        accounts,
        request.filter,
        request.page,
        request.page_size,
        request.desc.unwrap_or(true),
        sort_by,
    )
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct AttachmentSearchFilter {
    pub id: Option<String>,
    pub text: Option<String>,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub since: Option<i64>,
    pub before: Option<i64>,
    pub account_ids: Option<HashSet<u64>>,
    pub mailbox_ids: Option<HashSet<u64>>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,

    pub attachment_name: Option<String>,
    pub content_hash: Option<String>,

    pub tags: Option<HashSet<String>>,
    pub attachment_extension: Option<String>,
    pub attachment_category: Option<String>,
    pub attachment_content_type: Option<String>,

    pub is_ocr: Option<bool>,
    pub is_message: Option<bool>,
    pub has_text: Option<bool>,

    pub min_page_count: Option<u64>,
    pub max_page_count: Option<u64>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct AttachmentSearchRequest {
    filter: AttachmentSearchFilter,
    page: u64,
    page_size: u64,
    sort_by: Option<SortBy>,
    desc: Option<bool>,
}
impl AttachmentSearchRequest {
    pub fn filter(&self) -> &AttachmentSearchFilter {
        &self.filter
    }

    pub fn validate(&self) -> MailboxdResult<()> {
        if self.page == 0 || self.page_size == 0 {
            return Err(raise_error!(
                "Both page and page_size must be greater than 0.".into(),
                ErrorCode::InvalidParameter
            ));
        }
        if self.page_size > 500 {
            return Err(raise_error!(
                "The page_size exceeds the maximum allowed limit of 500.".into(),
                ErrorCode::InvalidParameter
            ));
        }

        Ok(())
    }
}

pub fn search_attachment_impl(
    accounts: Option<HashSet<u64>>,
    request: AttachmentSearchRequest,
) -> MailboxdResult<DataPage<AttachmentModel>> {
    request.validate()?;
    let sort_by = resolve_sort_by(request.sort_by, request.filter.has_text());
    ATTACHMENT_MANAGER.search(
        accounts,
        request.filter,
        request.page,
        request.page_size,
        request.desc.unwrap_or(true),
        sort_by,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_stays_the_default_even_with_text() {
        assert_eq!(resolve_sort_by(None, true), SortBy::DATE);
        assert_eq!(resolve_sort_by(None, false), SortBy::DATE);
    }

    #[test]
    fn relevance_is_kept_with_text_and_falls_back_to_date_without() {
        assert_eq!(
            resolve_sort_by(Some(SortBy::Relevance), true),
            SortBy::Relevance
        );
        assert_eq!(
            resolve_sort_by(Some(SortBy::Relevance), false),
            SortBy::DATE
        );
    }

    #[test]
    fn other_explicit_sorts_are_kept() {
        for sort_by in [
            SortBy::DATE,
            SortBy::SIZE,
            SortBy::InternalDate,
            SortBy::IngestAt,
        ] {
            assert_eq!(resolve_sort_by(Some(sort_by.clone()), true), sort_by);
        }
    }

    #[test]
    fn text_means_a_scored_query_field() {
        assert!(!EmailSearchFilter::default().has_text());
        for filter in [
            EmailSearchFilter {
                text: Some("x".into()),
                ..Default::default()
            },
            EmailSearchFilter {
                subject: Some("x".into()),
                ..Default::default()
            },
            EmailSearchFilter {
                body: Some("x".into()),
                ..Default::default()
            },
        ] {
            assert!(filter.has_text());
        }
        assert!(!EmailSearchFilter {
            from: Some("x".into()),
            ..Default::default()
        }
        .has_text());

        assert!(!AttachmentSearchFilter::default().has_text());
        assert!(AttachmentSearchFilter {
            text: Some("x".into()),
            ..Default::default()
        }
        .has_text());
        assert!(AttachmentSearchFilter {
            subject: Some("x".into()),
            ..Default::default()
        }
        .has_text());
    }

    mod against_the_global_indexes {
        use super::*;
        use crate::store::tantivy::envelope::test_support::{
            commit_documents, envelope, envelope_document, init_globals, register_mailbox,
            unique_id,
        };

        const PASSING: &str = "News from our partners, offers, events, a note about \
                               Primagaz, the weather, recipes, travel tips and much \
                               more for the season.";

        fn registered_mailbox() -> (u64, u64) {
            let (account_id, mailbox_id) = (unique_id(), unique_id());
            register_mailbox(account_id, mailbox_id, "owner@example.com");
            (account_id, mailbox_id)
        }

        /// The best match is older than a mail mentioning the term in passing.
        #[tokio::test(flavor = "multi_thread")]
        async fn message_search_sorts_by_relevance_only_when_asked() {
            init_globals();
            let (account_id, mailbox_id) = registered_mailbox();
            let mut best = envelope(account_id, mailbox_id, "best");
            best.subject = "Welcome to Primagaz".into();
            best.date = 1_000;
            let mut passing = envelope(account_id, mailbox_id, "passing");
            passing.subject = "September newsletter".into();
            passing.date = 2_000;
            commit_documents(
                &ENVELOPE_MANAGER,
                vec![
                    envelope_document(best, None, "Your Primagaz account is ready."),
                    envelope_document(passing, None, PASSING),
                ],
            )
            .await;

            let ids = |sort_by: Option<SortBy>| -> Vec<String> {
                let request = EmailSearchRequest {
                    filter: EmailSearchFilter {
                        text: Some("primagaz".into()),
                        ..Default::default()
                    },
                    page: 1,
                    page_size: 10,
                    sort_by,
                    desc: None,
                };
                search_messages_impl(Some(HashSet::from([account_id])), request)
                    .unwrap()
                    .items
                    .into_iter()
                    .map(|e| e.id)
                    .collect()
            };
            assert_eq!(ids(None), ["passing", "best"]);
            assert_eq!(ids(Some(SortBy::Relevance)), ["best", "passing"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn attachment_search_sorts_by_relevance_only_when_asked() {
            init_globals();
            let (account_id, mailbox_id) = registered_mailbox();
            let attachment = |id: &str, subject: &str, text: &str, date: i64| AttachmentModel {
                id: id.to_string(),
                envelope_id: id.to_string(),
                account_id,
                mailbox_id,
                subject: subject.to_string(),
                content_hash: format!("{:064x}", unique_id()),
                from: "sender@example.com".to_string(),
                date,
                category: "document".to_string(),
                content_type: "application/pdf".to_string(),
                text: Some(text.to_string()),
                ..Default::default()
            };
            {
                let mut writer = ATTACHMENT_MANAGER.index_writer().lock().await;
                for doc in [
                    attachment("best", "Primagaz contract", "Primagaz terms", 1_000),
                    attachment("passing", "September newsletter", PASSING, 2_000),
                ] {
                    writer.add_document(doc.into_document()).unwrap();
                }
                writer.commit().unwrap();
            }

            let ids = |sort_by: Option<SortBy>| -> Vec<String> {
                let request = AttachmentSearchRequest {
                    filter: AttachmentSearchFilter {
                        text: Some("primagaz".into()),
                        ..Default::default()
                    },
                    page: 1,
                    page_size: 10,
                    sort_by,
                    desc: None,
                };
                search_attachment_impl(Some(HashSet::from([account_id])), request)
                    .unwrap()
                    .items
                    .into_iter()
                    .map(|a| a.id)
                    .collect()
            };
            assert_eq!(ids(None), ["passing", "best"]);
            assert_eq!(ids(Some(SortBy::Relevance)), ["best", "passing"]);
        }
    }
}
