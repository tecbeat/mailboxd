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

use crate::common::auth::WrappedContext;
use crate::rest::api::ApiTags;
use crate::rest::ApiResult;
use mailboxd_core::account::migration::AccountModel;
use mailboxd_core::common::paginated::DataPage;
use mailboxd_core::envelope::extractor::{repair_envelope_blobs, BlobRepairReport};
use mailboxd_core::envelope::repair::BlobRepairRecord;
use mailboxd_core::error::code::ErrorCode;
use mailboxd_core::ext::event_bus::{emit, Event};
use mailboxd_core::message::append::restore_emails;
use mailboxd_core::message::append::RestoreMessagesRequest;
use mailboxd_core::message::attachment::retrieve_attachment_content;
use mailboxd_core::message::attachment::retrieve_nested_attachment_content;
use mailboxd_core::message::content::retrieve_nested_eml_content;
use mailboxd_core::message::content::FullNestedMessageContent;
use mailboxd_core::message::content::{retrieve_email_content, FullMessageContent};
use mailboxd_core::message::delete::{delete_messages_impl, DeleteMessagesResult};
use mailboxd_core::message::list::get_thread_messages;
use mailboxd_core::message::search::{search_messages_impl, EmailSearchRequest};
use mailboxd_core::message::tags::TagCount;
use mailboxd_core::message::tags::TagsRequest;
use mailboxd_core::raise_error;
use mailboxd_core::store::blob::get_reader;
use mailboxd_core::store::envelope::Envelope;
use mailboxd_core::store::tantivy::envelope::ENVELOPE_MANAGER;
use mailboxd_core::store::tantivy::validate_facet;
use mailboxd_core::users::permissions::Permission;
use poem::Body;
use poem_openapi::param::{Path, Query};
use poem_openapi::payload::{Attachment, AttachmentType, Json};
use poem_openapi::OpenApi;
use std::collections::HashMap;
use std::collections::HashSet;

pub struct MessageApi;

#[OpenApi(prefix_path = "/api/v1", tag = "ApiTags::Message")]
impl MessageApi {
    /// Deletes messages and returns how many were deleted.
    ///
    /// The request maps an **account id** (not a mailbox id) to the envelope
    /// ids to delete. Returns a not-found error when none of the messages exist.
    #[oai(
        path = "/delete-messages",
        method = "post",
        operation_id = "delete_messages"
    )]
    async fn delete_messages(
        &self,
        /// Map of account id to the envelope ids to delete.
        payload: Json<HashMap<u64, Vec<String>>>,
        context: WrappedContext,
    ) -> ApiResult<Json<DeleteMessagesResult>> {
        let request = payload.0;
        for account_id in request.keys() {
            context.require_permission(Some(*account_id), Permission::DATA_DELETE)?;
        }
        let user = context.user.username.clone();
        let result = delete_messages_impl(request.clone()).await?;
        for (account_id, envelope_ids) in &request {
            for envelope_id in envelope_ids {
                emit(Event::EmailDeleted {
                    email_id: format!("{account_id}/{envelope_id}"),
                    user: user.clone(),
                });
            }
        }
        Ok(Json(result))
    }

    /// Searches messages across all mailboxes using various filter criteria.
    /// The search filters are provided in the request body.
    #[oai(
        path = "/search-messages",
        method = "post",
        operation_id = "search_messages"
    )]
    async fn search_messages(
        &self,
        payload: Json<EmailSearchRequest>,
        context: WrappedContext,
    ) -> ApiResult<Json<DataPage<Envelope>>> {
        let authorized_ids: Option<HashSet<u64>> =
            if context.has_permission(None, Permission::DATA_READ_ALL) {
                None
            } else {
                Some(context.user.account_access_map.keys().cloned().collect())
            };
        let query = serde_json::to_string(&payload.0).unwrap_or_default();
        let result = search_messages_impl(authorized_ids, payload.0)?;
        emit(Event::SearchPerformed {
            query,
            user: context.user.username.clone(),
        });
        Ok(Json(result))
    }

    /// Retrieves all messages belonging to a specific thread. Requires `thread_id`, `page`, and `page_size` query parameters.
    #[oai(
        path = "/get-thread-messages/:account_id",
        method = "get",
        operation_id = "get_thread_messages"
    )]
    async fn get_thread_messages(
        &self,
        /// The ID of the account owning the mailbox.
        account_id: Path<u64>,
        // Thread ID
        thread_id: Query<String>,
        /// The page number for pagination (1-based).
        page: Query<u64>,
        /// The number of messages per page.
        page_size: Query<u64>,
        context: WrappedContext,
    ) -> ApiResult<Json<DataPage<Envelope>>> {
        let account_id = account_id.0;
        let thread_id = thread_id.0.trim();
        context.require_permission(Some(account_id), Permission::DATA_READ)?;
        Ok(Json(get_thread_messages(
            account_id,
            thread_id,
            page.0,
            page_size.0,
        )?))
    }

    /// Fetches the content of a specific email.
    /// Set `block_remote_content=true` to strip external images, scripts,
    /// and other content loaded from http/https URLs.
    #[oai(
        path = "/message-content/:account_id/:envelope_id",
        method = "get",
        operation_id = "fetch_message_content"
    )]
    async fn fetch_message_content(
        &self,
        /// The ID of the account.
        account_id: Path<u64>,
        /// The ID of the message to fetch.
        envelope_id: Path<String>,
        /// Block remote content (http/https URLs) from email body.
        block_remote_content: Query<Option<bool>>,
        context: WrappedContext,
    ) -> ApiResult<Json<FullMessageContent>> {
        let account_id = account_id.0;
        let block_remote = block_remote_content.0.unwrap_or(false);
        context.require_permission(Some(account_id), Permission::DATA_READ)?;
        let envelope_id = envelope_id.0.trim().to_string();
        let content = retrieve_email_content(account_id, envelope_id.clone(), block_remote)?;
        emit(Event::EmailViewed {
            email_id: format!("{account_id}/{envelope_id}"),
            user: context.user.username.clone(),
            ip: None,
        });
        Ok(Json(content))
    }

    /// Retrieves the content of an email embedded as an attachment.
    #[oai(
        path = "/nested-message-content/:account_id/:envelope_id",
        method = "get",
        operation_id = "fetch_nested_message_content"
    )]
    async fn fetch_nested_message_content(
        &self,
        /// The ID of the account.
        account_id: Path<u64>,
        /// The ID of the message to fetch.
        envelope_id: Path<String>,
        content_hash: Query<String>,
        block_remote_content: Query<Option<bool>>,
        context: WrappedContext,
    ) -> ApiResult<Json<FullNestedMessageContent>> {
        let account_id = account_id.0;
        let block_remote = block_remote_content.0.unwrap_or(false);
        context.require_permission(Some(account_id), Permission::DATA_READ)?;
        let content_hash = content_hash.0.trim();
        Ok(Json(retrieve_nested_eml_content(
            account_id,
            envelope_id.0,
            content_hash,
            block_remote,
        )?))
    }

    /// Retrieves the envelope (metadata) of a specific message.
    #[oai(
        path = "/envelope/:account_id/:envelope_id",
        method = "get",
        operation_id = "get_envelope"
    )]
    async fn get_envelope(
        &self,
        /// The ID of the account.
        account_id: Path<u64>,
        /// The ID of the message.
        envelope_id: Path<String>,
        context: WrappedContext,
    ) -> ApiResult<Json<Envelope>> {
        let account_id = account_id.0;
        context.require_permission(Some(account_id), Permission::DATA_READ)?;
        let envelope_id = envelope_id.0;
        let e = ENVELOPE_MANAGER
            .get_envelope_by_id(account_id, &envelope_id)?
            .ok_or_else(|| {
                raise_error!(
                    format!(
                        "Envelope not found: account_id={} envelope_id={}",
                        account_id, &envelope_id
                    ),
                    ErrorCode::ResourceNotFound
                )
            })?;
        Ok(Json(e.envelope))
    }

    /// Downloads the raw EML file of a specific email.
    #[oai(
        path = "/download-message/:account_id/:envelope_id",
        method = "get",
        operation_id = "download_message"
    )]
    async fn download_message(
        &self,
        /// The ID of the account.
        account_id: Path<u64>,
        /// The ID of the message to download.
        envelope_id: Path<String>,
        context: WrappedContext,
    ) -> ApiResult<Attachment<Body>> {
        let account_id = account_id.0;
        AccountModel::check_account_exists(account_id)?;
        context.require_permission(Some(account_id), Permission::DATA_RAW_DOWNLOAD)?;
        let envelope_id = envelope_id.0;
        let reader = get_reader(account_id, envelope_id.clone()).await?;
        let body = Body::from_async_read(reader);
        let attachment = Attachment::new(body)
            .attachment_type(AttachmentType::Attachment)
            .filename(format!("{envelope_id}.eml"));
        Ok(attachment)
    }

    /// Re-fetches a message from IMAP and restores its missing email or
    /// attachment blobs. Content is only stored when it matches the archived
    /// hash. A repair that could not fix the message still returns 200, with
    /// status `partial` or `failed` in the report.
    #[oai(
        path = "/repair-message/:account_id/:envelope_id",
        method = "post",
        operation_id = "repair_message"
    )]
    async fn repair_message(
        &self,
        /// The ID of the account.
        account_id: Path<u64>,
        /// The ID of the message to repair.
        envelope_id: Path<String>,
        context: WrappedContext,
    ) -> ApiResult<Json<BlobRepairReport>> {
        let account_id = account_id.0;
        AccountModel::check_account_exists(account_id)?;
        context.require_permission(Some(account_id), Permission::DATA_MANAGE)?;
        let report = repair_envelope_blobs(account_id, &envelope_id.0).await?;
        BlobRepairRecord::record_attempt(context.user.id, &context.user.username, &report);
        Ok(Json(report))
    }

    /// Lists the most recent repair attempts, newest first. Repairs that
    /// found nothing to repair are not recorded.
    #[oai(
        path = "/repair-reports",
        method = "get",
        operation_id = "list_repair_reports"
    )]
    async fn list_repair_reports(
        &self,
        /// Maximum number of reports (default 50, at most 200).
        limit: Query<Option<usize>>,
        /// Only reports of this account.
        account_id: Query<Option<u64>>,
        context: WrappedContext,
    ) -> ApiResult<Json<Vec<BlobRepairRecord>>> {
        let limit = repair_report_limit(limit.0);
        let records = BlobRepairRecord::visible(limit, account_id.0, |id| {
            context.has_permission(Some(id), Permission::DATA_MANAGE)
        })?;
        Ok(Json(records))
    }

    /// Restore an email to an account's IMAP server.
    #[oai(
        path = "/restore-messages/:account_id",
        method = "post",
        operation_id = "restore_messages"
    )]
    async fn restore_messages(
        &self,
        account_id: Path<u64>,
        /// Message IDs to restore.
        payload: Json<RestoreMessagesRequest>,
        context: WrappedContext,
    ) -> ApiResult<()> {
        let account_id = account_id.0;
        context.require_permission(Some(account_id), Permission::DATA_EXPORT_BATCH)?;
        Ok(restore_emails(account_id, payload.0.envelope_ids).await?)
    }

    /// Downloads a specific attachment from an email. Requires `content_hash` query parameter.
    #[oai(
        path = "/download-attachment/:account_id/:envelope_id",
        method = "get",
        operation_id = "download_attachment"
    )]
    async fn download_attachment(
        &self,
        /// The ID of the account.
        account_id: Path<u64>,
        /// The ID of the message containing the attachment.
        envelope_id: Path<String>,
        /// The content_hash of the attachment to download.
        content_hash: Query<String>,
        context: WrappedContext,
    ) -> ApiResult<Attachment<Body>> {
        let account_id = account_id.0;
        let envelope_id = envelope_id.0.trim().to_string();
        AccountModel::check_account_exists(account_id)?;
        context.require_permission(Some(account_id), Permission::DATA_READ)?;
        let content_hash = content_hash.0.trim();
        let reader = retrieve_attachment_content(account_id, envelope_id.clone(), content_hash)?;
        emit(Event::AttachmentDownloaded {
            email_id: format!("{account_id}/{envelope_id}"),
            content_hash: content_hash.to_string(),
            user: context.user.username.clone(),
        });
        let body = Body::from_async_read(reader);
        let attachment = Attachment::new(body)
            .attachment_type(AttachmentType::Attachment)
            .filename(content_hash);
        Ok(attachment)
    }

    /// Returns raw attachment content for in-browser preview with
    /// `Content-Disposition: inline` and the correct MIME type.
    #[oai(
        path = "/preview-attachment/:account_id/:envelope_id",
        method = "get",
        operation_id = "preview_attachment"
    )]
    async fn preview_attachment(
        &self,
        /// The ID of the account.
        account_id: Path<u64>,
        /// The ID of the message containing the attachment.
        envelope_id: Path<String>,
        /// The content_hash of the attachment to preview.
        content_hash: Query<String>,
        context: WrappedContext,
    ) -> ApiResult<Attachment<Body>> {
        let account_id = account_id.0;
        let envelope_id = envelope_id.0.trim().to_string();
        AccountModel::check_account_exists(account_id)?;
        context.require_permission(Some(account_id), Permission::DATA_READ)?;
        let content_hash = content_hash.0.trim();
        let reader = retrieve_attachment_content(account_id, envelope_id.clone(), content_hash)?;
        let body = Body::from_async_read(reader);
        Ok(Attachment::new(body).attachment_type(AttachmentType::Inline))
    }

    /// Downloads an attachment from within a nested email (EML file).
    #[oai(
        path = "/download-nested-attachment/:account_id/:envelope_id",
        method = "get",
        operation_id = "download_nested_attachment"
    )]
    async fn download_nested_attachment(
        &self,
        /// The ID of the account.
        account_id: Path<u64>,
        /// The ID of the message containing the attachment.
        envelope_id: Path<String>,
        /// The filename of the attachment to download.
        content_hash: Query<String>,
        nested_content_hash: Query<String>,
        context: WrappedContext,
    ) -> ApiResult<Attachment<Body>> {
        let account_id = account_id.0;
        let envelope_id = envelope_id.0.trim().to_string();
        AccountModel::check_account_exists(account_id)?;
        context.require_permission(Some(account_id), Permission::DATA_READ)?;
        let content_hash = content_hash.0.trim();
        let nested_content_hash = nested_content_hash.0.trim();
        let reader = retrieve_nested_attachment_content(
            account_id,
            envelope_id.clone(),
            content_hash,
            nested_content_hash,
        )?;
        let body = Body::from_async_read(reader);
        let attachment = Attachment::new(body)
            .attachment_type(AttachmentType::Attachment)
            .filename(nested_content_hash);
        Ok(attachment)
    }

    /// Returns all facets in the index along with their document counts.
    #[oai(path = "/all-tags", method = "get", operation_id = "get_all_tags")]
    async fn get_all_tags(&self, context: WrappedContext) -> ApiResult<Json<Vec<TagCount>>> {
        let authorized_ids: Option<HashSet<u64>> =
            if context.has_permission(None, Permission::DATA_READ_ALL) {
                None
            } else {
                Some(context.user.account_access_map.keys().cloned().collect())
            };
        Ok(Json(ENVELOPE_MANAGER.get_all_tags(authorized_ids)?))
    }

    /// Adds or removes facet tags for multiple emails across accounts.
    #[oai(
        path = "/update-tags",
        method = "post",
        operation_id = "update_envelope_tags"
    )]
    async fn update_envelope_tags(
        &self,
        req: Json<TagsRequest>,
        context: WrappedContext,
    ) -> ApiResult<()> {
        let req = req.0;
        for tag in &req.tags {
            validate_facet(tag)?;
        }

        for account_id in req.updates.keys() {
            context.require_permission(Some(*account_id), Permission::DATA_MANAGE)?;
        }

        ENVELOPE_MANAGER.update_envelope_tags(req).await?;
        Ok(())
    }

    /// Retrieves a unique list of all contact email addresses across authorized accounts.
    #[oai(
        path = "/all-contacts",
        method = "get",
        operation_id = "get_all_contacts"
    )]
    async fn get_all_contacts(&self, context: WrappedContext) -> ApiResult<Json<HashSet<String>>> {
        let authorized_ids: Option<HashSet<u64>> =
            if context.has_permission(None, Permission::DATA_READ_ALL) {
                None
            } else {
                Some(context.user.account_access_map.keys().cloned().collect())
            };
        Ok(Json(ENVELOPE_MANAGER.get_all_contacts(authorized_ids)?))
    }
}

/// The number of repair reports to list: 50 unless asked otherwise, at most
/// the size of the history.
fn repair_report_limit(requested: Option<usize>) -> usize {
    requested.unwrap_or(50).min(BlobRepairRecord::HISTORY_LIMIT)
}

#[cfg(test)]
mod tests {
    use super::repair_report_limit;
    use crate::tests::{admin_token, build_api_route, login, setup};
    use mailboxd_core::account::migration::AccountModel;
    use mailboxd_core::database::{manager::DB_MANAGER, upsert_impl};
    use mailboxd_core::envelope::extractor::BlobRepairReport;
    use mailboxd_core::envelope::repair::BlobRepairRecord;
    use poem::test::TestClient;

    /// Ids used by no other test.
    const REPAIR_ACCOUNT_A: u64 = 61_332_003;
    const REPAIR_ACCOUNT_B: u64 = 61_332_004;

    fn client() -> TestClient<impl poem::Endpoint> {
        TestClient::new(build_api_route()).default_header("X-Forwarded-For", "127.0.0.1")
    }

    fn store_account(id: u64) {
        upsert_impl(
            DB_MANAGER.db(),
            AccountModel {
                id,
                email: format!("repair-{id}@example.com"),
                ..Default::default()
            },
        )
        .unwrap();
    }

    /// A member with no access to any account.
    async fn member_token(username: &str) -> String {
        let resp = client()
            .post("/api/v1/users")
            .header("Authorization", format!("Bearer {}", admin_token().await))
            .body_json(&serde_json::json!({
                "username": username,
                "email": format!("{username}@example.com"),
                "password": "member-pass-123",
                "global_roles": [mailboxd_core::users::role::DEFAULT_MEMBER_ROLE_ID],
                "account_access_map": {},
            }))
            .send()
            .await;
        resp.assert_status_is_ok();
        login(username, "member-pass-123").await
    }

    async fn list_reports(token: &str, query: &str) -> Vec<BlobRepairRecord> {
        let resp = client()
            .get(format!("/api/v1/repair-reports{query}"))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.json().await.value().deserialize()
    }

    #[test]
    fn repair_report_limit_defaults_to_50_and_is_capped_at_the_history() {
        assert_eq!(repair_report_limit(None), 50);
        assert_eq!(repair_report_limit(Some(3)), 3);
        assert_eq!(
            repair_report_limit(Some(10_000)),
            BlobRepairRecord::HISTORY_LIMIT
        );
    }

    #[tokio::test]
    async fn repairing_a_message_of_an_unknown_account_is_not_found() {
        setup().await;
        let resp = client()
            .post("/api/v1/repair-message/61332005/some-envelope")
            .header("Authorization", format!("Bearer {}", admin_token().await))
            .send()
            .await;
        resp.assert_status(http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn repairing_an_unknown_message_is_not_found() {
        setup().await;
        store_account(REPAIR_ACCOUNT_A);
        let resp = client()
            .post(format!(
                "/api/v1/repair-message/{REPAIR_ACCOUNT_A}/no-such-envelope"
            ))
            .header("Authorization", format!("Bearer {}", admin_token().await))
            .send()
            .await;
        resp.assert_status(http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn repair_reports_are_listed_per_account_and_permission() {
        setup().await;
        store_account(REPAIR_ACCOUNT_A);
        store_account(REPAIR_ACCOUNT_B);
        for (account_id, envelope_id) in [
            (REPAIR_ACCOUNT_A, "a-old"),
            (REPAIR_ACCOUNT_B, "b"),
            (REPAIR_ACCOUNT_A, "a-new"),
        ] {
            let report = BlobRepairReport {
                envelope_id: envelope_id.into(),
                account_id,
                status: "failed".into(),
                ..Default::default()
            };
            assert!(BlobRepairRecord::persist(1, "admin", &report).unwrap());
            // Distinct millisecond timestamps keep the order deterministic.
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        }
        let admin = admin_token().await;
        let ids = |records: Vec<BlobRepairRecord>| -> Vec<String> {
            records
                .into_iter()
                .filter(|r| [REPAIR_ACCOUNT_A, REPAIR_ACCOUNT_B].contains(&r.account_id))
                .map(|r| r.report.envelope_id)
                .collect()
        };

        assert_eq!(ids(list_reports(&admin, "").await), ["a-new", "b", "a-old"]);
        assert_eq!(
            ids(list_reports(&admin, &format!("?account_id={REPAIR_ACCOUNT_A}")).await),
            ["a-new", "a-old"]
        );
        assert_eq!(
            ids(list_reports(&admin, &format!("?account_id={REPAIR_ACCOUNT_A}&limit=1")).await),
            ["a-new"]
        );

        let member = member_token("member-repair-reports").await;
        assert!(list_reports(&member, "").await.is_empty());
    }

    #[tokio::test]
    async fn a_member_without_data_manage_cannot_repair() {
        setup().await;
        store_account(REPAIR_ACCOUNT_B);
        let member = member_token("member-repair").await;
        let resp = client()
            .post(format!(
                "/api/v1/repair-message/{REPAIR_ACCOUNT_B}/some-envelope"
            ))
            .header("Authorization", format!("Bearer {}", member))
            .send()
            .await;
        resp.assert_status(http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn deleting_unknown_messages_is_reported_as_not_found() {
        setup().await;
        let token = admin_token().await;
        let cli = TestClient::new(build_api_route()).default_header("X-Forwarded-For", "127.0.0.1");

        // Account id used by no other test.
        let resp = cli
            .post("/api/v1/delete-messages")
            .header("Authorization", format!("Bearer {}", token))
            .body_json(&serde_json::json!({ "61332002": ["no-such-envelope"] }))
            .send()
            .await;

        resp.assert_status(http::StatusCode::NOT_FOUND);
    }
}
