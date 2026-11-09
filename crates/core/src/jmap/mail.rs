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

//! JMAP mail methods (RFC 8621): `Mailbox/get`, `Mailbox/changes`,
//! `Email/query`, `Email/get`, `Email/changes`, `Email/import`.
//!
//! These are thin typed wrappers over [`JmapClient`](crate::jmap::client::JmapClient):
//! each builds the method arguments, issues the call, maps method-level errors
//! to [`MailboxdError`](crate::error::MailboxdError), and parses the result. The
//! objects are modelled only to the depth the archiver needs; unknown fields are
//! ignored so forward-compatible servers don't break parsing.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::MailboxdResult;
use crate::jmap::client::JmapClient;
use crate::jmap::protocol::{result_reference, Invocation};
use crate::jmap::session::CAP_MAIL;

/// A JMAP Mailbox (RFC 8621 §2).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JmapMailbox {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "parentId", default)]
    pub parent_id: Option<String>,
    /// Role (e.g. `inbox`, `sent`, `trash`, `junk`, `archive`), if any.
    #[serde(default)]
    pub role: Option<String>,
    #[serde(rename = "sortOrder", default)]
    pub sort_order: u32,
    #[serde(rename = "totalEmails", default)]
    pub total_emails: u64,
    #[serde(rename = "unreadEmails", default)]
    pub unread_emails: u64,
}

/// Result of `Mailbox/get` (RFC 8621 §2.1 → RFC 8620 §5.1).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct MailboxGetResponse {
    #[serde(rename = "accountId", default)]
    pub account_id: String,
    pub state: String,
    pub list: Vec<JmapMailbox>,
    #[serde(rename = "notFound", default)]
    pub not_found: Vec<String>,
}

/// Result of a `*/changes` call (RFC 8620 §5.2).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ChangesResponse {
    #[serde(rename = "accountId", default)]
    pub account_id: String,
    #[serde(rename = "oldState", default)]
    pub old_state: String,
    #[serde(rename = "newState", default)]
    pub new_state: String,
    #[serde(rename = "hasMoreChanges", default)]
    pub has_more_changes: bool,
    #[serde(default)]
    pub created: Vec<String>,
    #[serde(default)]
    pub updated: Vec<String>,
    #[serde(default)]
    pub destroyed: Vec<String>,
}

/// Result of `Email/query` (RFC 8621 §4.4 → RFC 8620 §5.5).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct EmailQueryResponse {
    #[serde(rename = "accountId", default)]
    pub account_id: String,
    #[serde(rename = "queryState", default)]
    pub query_state: String,
    #[serde(default)]
    pub position: u64,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub ids: Vec<String>,
}

/// A JMAP Email, modelled to the depth the archiver needs (RFC 8621 §4).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JmapEmail {
    pub id: String,
    #[serde(rename = "blobId", default)]
    pub blob_id: String,
    #[serde(rename = "threadId", default)]
    pub thread_id: Option<String>,
    /// Map of mailbox id → true: which mailboxes this email belongs to.
    #[serde(rename = "mailboxIds", default)]
    pub mailbox_ids: std::collections::HashMap<String, bool>,
    #[serde(default)]
    pub size: u64,
    #[serde(rename = "receivedAt", default)]
    pub received_at: Option<String>,
    #[serde(rename = "messageId", default)]
    pub message_id: Option<Vec<String>>,
    #[serde(default)]
    pub subject: Option<String>,
}

/// Result of `Email/get` (RFC 8621 §4.1 → RFC 8620 §5.1).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct EmailGetResponse {
    #[serde(rename = "accountId", default)]
    pub account_id: String,
    pub state: String,
    pub list: Vec<JmapEmail>,
    #[serde(rename = "notFound", default)]
    pub not_found: Vec<String>,
}

fn parse_result<T: for<'de> Deserialize<'de>>(
    value: serde_json::Value,
    context: &str,
) -> MailboxdResult<T> {
    serde_json::from_value(value).map_err(|e| {
        crate::raise_error!(
            format!("{context}: failed to parse result: {e}"),
            crate::error::code::ErrorCode::JmapUnexpectedResult
        )
    })
}

impl JmapClient {
    /// `Mailbox/get`: fetch mailboxes (all when `ids` is `None`).
    pub async fn mailbox_get(
        &self,
        account_id: &str,
        ids: Option<Vec<String>>,
    ) -> MailboxdResult<MailboxGetResponse> {
        let args = json!({
            "accountId": account_id,
            "ids": ids,
        });
        let result = self
            .call_one(
                vec![CAP_MAIL.to_string()],
                Invocation::new("Mailbox/get", args, "0"),
            )
            .await?;
        parse_result(result, "Mailbox/get")
    }

    /// `Mailbox/changes`: delta of mailbox ids since `since_state` (RFC 8620 §5.2).
    pub async fn mailbox_changes(
        &self,
        account_id: &str,
        since_state: &str,
    ) -> MailboxdResult<ChangesResponse> {
        let args = json!({
            "accountId": account_id,
            "sinceState": since_state,
        });
        let result = self
            .call_one(
                vec![CAP_MAIL.to_string()],
                Invocation::new("Mailbox/changes", args, "0"),
            )
            .await?;
        parse_result(result, "Mailbox/changes")
    }

    /// `Email/query`: ids of emails matching `filter`, newest first by
    /// `receivedAt`. `limit` is clamped by the server.
    pub async fn email_query(
        &self,
        account_id: &str,
        filter: Option<serde_json::Value>,
        position: u64,
        limit: u64,
    ) -> MailboxdResult<EmailQueryResponse> {
        let args = json!({
            "accountId": account_id,
            "filter": filter,
            "sort": [ { "property": "receivedAt", "isAscending": false } ],
            "position": position,
            "limit": limit,
            "calculateTotal": true,
        });
        let result = self
            .call_one(
                vec![CAP_MAIL.to_string()],
                Invocation::new("Email/query", args, "0"),
            )
            .await?;
        parse_result(result, "Email/query")
    }

    /// `Email/get`: fetch email objects by id with the given `properties`
    /// (defaults to the archiver's set when `None`).
    pub async fn email_get(
        &self,
        account_id: &str,
        ids: Vec<String>,
        properties: Option<Vec<String>>,
    ) -> MailboxdResult<EmailGetResponse> {
        let properties = properties.unwrap_or_else(|| {
            vec![
                "id".into(),
                "blobId".into(),
                "threadId".into(),
                "mailboxIds".into(),
                "size".into(),
                "receivedAt".into(),
                "messageId".into(),
                "subject".into(),
            ]
        });
        let args = json!({
            "accountId": account_id,
            "ids": ids,
            "properties": properties,
        });
        let result = self
            .call_one(
                vec![CAP_MAIL.to_string()],
                Invocation::new("Email/get", args, "0"),
            )
            .await?;
        parse_result(result, "Email/get")
    }

    /// `Email/changes`: delta of email ids since `since_state`. Returns a typed
    /// `cannotCalculateChanges` error (via the method-error mapping) so the sync
    /// engine can fall back to a full resync (FA-11).
    pub async fn email_changes(
        &self,
        account_id: &str,
        since_state: &str,
        max_changes: Option<u64>,
    ) -> MailboxdResult<ChangesResponse> {
        let args = json!({
            "accountId": account_id,
            "sinceState": since_state,
            "maxChanges": max_changes,
        });
        let result = self
            .call_one(
                vec![CAP_MAIL.to_string()],
                Invocation::new("Email/changes", args, "0"),
            )
            .await?;
        parse_result(result, "Email/changes")
    }

    /// `Email/query` + `Email/get` chained via a result reference in a single
    /// request (RFC 8620 §3.7): fetch the newest emails' full objects in one
    /// round-trip. Returns the `Email/get` result.
    pub async fn email_query_get(
        &self,
        account_id: &str,
        filter: Option<serde_json::Value>,
        limit: u64,
        properties: Option<Vec<String>>,
    ) -> MailboxdResult<EmailGetResponse> {
        let query_args = json!({
            "accountId": account_id,
            "filter": filter,
            "sort": [ { "property": "receivedAt", "isAscending": false } ],
            "position": 0,
            "limit": limit,
        });
        let properties = properties.unwrap_or_else(|| {
            vec![
                "id".into(),
                "blobId".into(),
                "mailboxIds".into(),
                "size".into(),
                "receivedAt".into(),
            ]
        });
        let get_args = json!({
            "accountId": account_id,
            // #ids is a back-reference to the query call's /ids output.
            "#ids": result_reference("q", "Email/query", "/ids"),
            "properties": properties,
        });

        let response = self
            .request(
                vec![CAP_MAIL.to_string()],
                vec![
                    Invocation::new("Email/query", query_args, "q"),
                    Invocation::new("Email/get", get_args, "g"),
                ],
            )
            .await?;

        let result = response
            .result_for("g")
            .map_err(|e| e.into_mailboxd_error("Email/query+get"))?;
        parse_result(result.clone(), "Email/get")
    }

    /// `Email/import`: import a previously-uploaded blob into mailboxes (RFC 8621
    /// §4.8), used for restore (FA-17). Returns the raw result JSON.
    pub async fn email_import(
        &self,
        account_id: &str,
        blob_id: &str,
        mailbox_ids: &[String],
        keywords: Option<serde_json::Value>,
        received_at: Option<&str>,
    ) -> MailboxdResult<serde_json::Value> {
        let mailbox_map: serde_json::Map<String, serde_json::Value> = mailbox_ids
            .iter()
            .map(|id| (id.clone(), json!(true)))
            .collect();

        let mut import_obj = json!({
            "blobId": blob_id,
            "mailboxIds": mailbox_map,
            "keywords": keywords.unwrap_or_else(|| json!({})),
        });
        if let Some(ts) = received_at {
            import_obj["receivedAt"] = json!(ts);
        }

        let args = json!({
            "accountId": account_id,
            "emails": { "restore": import_obj },
        });
        self.call_one(
            vec![CAP_MAIL.to_string()],
            Invocation::new("Email/import", args, "0"),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mailbox_get_response() {
        let v = json!({
            "accountId": "A1",
            "state": "m-1",
            "list": [
                {"id": "mb1", "name": "Inbox", "role": "inbox", "totalEmails": 10, "unreadEmails": 2},
                {"id": "mb2", "name": "Sent", "parentId": null, "role": "sent"}
            ],
            "notFound": []
        });
        let r: MailboxGetResponse = parse_result(v, "test").unwrap();
        assert_eq!(r.account_id, "A1");
        assert_eq!(r.list.len(), 2);
        assert_eq!(r.list[0].role.as_deref(), Some("inbox"));
        assert_eq!(r.list[0].total_emails, 10);
    }

    #[test]
    fn parses_changes_response() {
        let v = json!({
            "accountId": "A1",
            "oldState": "s1",
            "newState": "s2",
            "hasMoreChanges": false,
            "created": ["e1", "e2"],
            "updated": [],
            "destroyed": ["e0"]
        });
        let r: ChangesResponse = parse_result(v, "test").unwrap();
        assert_eq!(r.new_state, "s2");
        assert_eq!(r.created, vec!["e1", "e2"]);
        assert_eq!(r.destroyed, vec!["e0"]);
        assert!(!r.has_more_changes);
    }

    #[test]
    fn parses_email_query_response() {
        let v = json!({
            "accountId": "A1",
            "queryState": "q1",
            "position": 0,
            "total": 3,
            "ids": ["e1", "e2", "e3"]
        });
        let r: EmailQueryResponse = parse_result(v, "test").unwrap();
        assert_eq!(r.ids.len(), 3);
        assert_eq!(r.total, Some(3));
    }

    #[test]
    fn parses_email_get_with_mailbox_ids() {
        let v = json!({
            "accountId": "A1",
            "state": "e-1",
            "list": [
                {
                    "id": "e1",
                    "blobId": "b1",
                    "threadId": "t1",
                    "mailboxIds": {"mb1": true, "mb2": true},
                    "size": 2048,
                    "receivedAt": "2026-01-01T00:00:00Z",
                    "messageId": ["<abc@example.com>"],
                    "subject": "Hello"
                }
            ],
            "notFound": []
        });
        let r: EmailGetResponse = parse_result(v, "test").unwrap();
        let email = &r.list[0];
        assert_eq!(email.blob_id, "b1");
        assert_eq!(email.size, 2048);
        // one email, two mailboxes (FA-12 shape)
        assert_eq!(email.mailbox_ids.len(), 2);
        assert!(email.mailbox_ids.get("mb1").copied().unwrap_or(false));
    }

    #[test]
    fn email_unknown_fields_are_ignored() {
        let v = json!({
            "id": "e1",
            "blobId": "b1",
            "mailboxIds": {"mb1": true},
            "size": 1,
            "futureField": {"nested": [1, 2, 3]},
            "anotherNewThing": "ignored"
        });
        let email: JmapEmail = serde_json::from_value(v).unwrap();
        assert_eq!(email.id, "e1");
    }
}
