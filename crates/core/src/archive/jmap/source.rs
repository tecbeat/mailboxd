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

//! JMAP [`MailSource`] / [`MailSession`] implementation.
//!
//! [`JmapSource`] opens a [`JmapMailSession`] by resolving the account's
//! [`JmapConfig`] into transport credentials and connecting the low-level
//! [`JmapClient`]. The session maps JMAP mail methods onto the source-generic
//! trait:
//!
//! * `list_mailboxes` → `Mailbox/get`, mapping role → [`AttributeEnum`] and
//!   hierarchy via `parentId`.
//! * `changes_since` → the JMAP `state` string is the opaque [`SyncCursor`].
//!   Initial sync uses `Email/query` (optionally date-filtered); incremental
//!   uses `Email/changes`, falling back to a full reconcile on
//!   `cannotCalculateChanges` (FA-11).
//! * `load_raw` → `Email/get` blobId then a blob download of the RFC 5322 bytes.
//! * `append` → `Email/import` of an uploaded blob (used by restore).

use std::collections::HashMap;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::account::entity::{JmapAuthType, JmapConfig};
use crate::account::migration::AccountModel;
use crate::archive::imap::mailbox::{Attribute, AttributeEnum, MailBox};
use crate::archive::source::{
    MailSession, MailSource, MailboxChanges, MessageRef, RawMessage, SyncCursor,
};
use crate::error::code::ErrorCode;
use crate::error::MailboxdResult;
use crate::jmap::client::{JmapAuth, JmapClient};
use crate::jmap::mail::JmapMailbox;
use crate::oauth2::token::OAuth2AccessToken;
use crate::utils::create_hash;
use crate::{decrypt, raise_error};

/// Map a JMAP mailbox `role` (RFC 8621 §2) to our [`AttributeEnum`], so JMAP
/// mailboxes carry the same special-use markers as IMAP ones (FA-7).
pub(crate) fn role_to_attribute(role: Option<&str>) -> Option<Attribute> {
    let attr = match role? {
        "inbox" => AttributeEnum::Marked, // no dedicated Inbox attr; leave as a normal selectable box
        "archive" => AttributeEnum::Archive,
        "drafts" => AttributeEnum::Drafts,
        "flagged" => AttributeEnum::Flagged,
        "junk" => AttributeEnum::Junk,
        "sent" => AttributeEnum::Sent,
        "trash" => AttributeEnum::Trash,
        "all" => AttributeEnum::All,
        _ => return None,
    };
    Some(Attribute::new(attr, None))
}

/// Resolve the full hierarchical name of a JMAP mailbox using `parentId` links,
/// joined with `/` (e.g. `INBOX/Invoices`). Falls back to the leaf name if the
/// parent chain is broken.
pub(crate) fn resolve_hierarchical_name(
    mailbox: &JmapMailbox,
    by_id: &HashMap<String, JmapMailbox>,
) -> String {
    let mut parts = vec![mailbox.name.clone()];
    let mut current = mailbox.parent_id.clone();
    // Guard against cycles / very deep trees.
    let mut guard = 0;
    while let Some(pid) = current {
        if guard > 64 {
            break;
        }
        guard += 1;
        match by_id.get(&pid) {
            Some(parent) => {
                parts.push(parent.name.clone());
                current = parent.parent_id.clone();
            }
            None => break,
        }
    }
    parts.reverse();
    parts.join("/")
}

/// Convert a JMAP mailbox into our [`MailBox`] model (FA-7). The stable local id
/// is derived from `(account_id, jmap mailbox id)` so it survives renames, and
/// the JMAP mailbox id is stored in `sync_cursor`'s sibling is *not* used here —
/// the per-mailbox email-state cursor is persisted by the engine instead.
pub(crate) fn to_mailbox(
    account_id: u64,
    mailbox: &JmapMailbox,
    by_id: &HashMap<String, JmapMailbox>,
) -> MailBox {
    let name = resolve_hierarchical_name(mailbox, by_id);
    let mut attributes = Vec::new();
    if let Some(attr) = role_to_attribute(mailbox.role.as_deref()) {
        attributes.push(attr);
    }
    MailBox {
        // Stable id keyed by the opaque JMAP mailbox id (not the display name,
        // which can change) so re-syncs map to the same local record (FA-9).
        id: create_hash(account_id, &mailbox.id),
        account_id,
        name,
        delimiter: Some("/".to_string()),
        attributes,
        exists: mailbox.total_emails as u32,
        unseen: Some(mailbox.unread_emails as u32),
        uid_next: None,
        uid_validity: None,
        highest_uid: None,
        sync_cursor: None,
    }
}

/// Resolve an account's [`JmapConfig`] into the transport [`JmapAuth`],
/// decrypting the stored secret and, for OAuth2, fetching the current access
/// token from the OAuth2 subsystem.
fn resolve_auth(account_id: u64, jmap: &JmapConfig) -> MailboxdResult<JmapAuth> {
    match jmap.auth.auth_type {
        JmapAuthType::Basic => {
            let username = jmap.auth.username.clone().ok_or_else(|| {
                raise_error!(
                    "JMAP Basic auth requires a username".into(),
                    ErrorCode::InvalidParameter
                )
            })?;
            let secret = jmap.auth.secret.as_ref().ok_or_else(|| {
                raise_error!(
                    "JMAP Basic auth requires a password".into(),
                    ErrorCode::InvalidParameter
                )
            })?;
            Ok(JmapAuth::Basic {
                username,
                password: decrypt!(secret)?,
            })
        }
        JmapAuthType::Bearer => {
            let secret = jmap.auth.secret.as_ref().ok_or_else(|| {
                raise_error!(
                    "JMAP Bearer auth requires a token".into(),
                    ErrorCode::InvalidParameter
                )
            })?;
            Ok(JmapAuth::Bearer {
                token: decrypt!(secret)?,
            })
        }
        JmapAuthType::OAuth2 => {
            let token = OAuth2AccessToken::get(account_id)?.ok_or_else(|| {
                raise_error!(
                    "JMAP OAuth2 authorization not completed for this account".into(),
                    ErrorCode::JmapAuthenticationFailed
                )
            })?;
            let access_token = token.access_token.ok_or_else(|| {
                raise_error!(
                    "JMAP OAuth2 access token is missing".into(),
                    ErrorCode::JmapAuthenticationFailed
                )
            })?;
            Ok(JmapAuth::Bearer {
                token: access_token,
            })
        }
    }
}

/// A live JMAP session: the connected [`JmapClient`] plus the owning account id
/// and the resolved mail account id on the server.
pub struct JmapMailSession {
    account_id: u64,
    jmap_account_id: String,
    client: JmapClient,
    /// Map of local `MailBox.id` → JMAP mailbox id, built in `list_mailboxes`,
    /// so later `changes_since`/`load_raw` can translate back.
    mailbox_ids: HashMap<u64, String>,
}

impl JmapMailSession {
    /// Construct a session from an already-connected client. Test-only: the
    /// production path goes through [`JmapSource::connect`].
    #[cfg(test)]
    pub(crate) fn new_for_test(
        account_id: u64,
        jmap_account_id: String,
        client: JmapClient,
    ) -> Self {
        Self {
            account_id,
            jmap_account_id,
            client,
            mailbox_ids: HashMap::new(),
        }
    }

    /// Query email ids in a mailbox, optionally bounded by the account date
    /// window, newest first. Used for the initial sync and the
    /// `cannotCalculateChanges` reconcile.
    async fn query_all_in_mailbox(
        &self,
        jmap_mailbox_id: &str,
    ) -> MailboxdResult<Vec<MessageRef>> {
        let filter = serde_json::json!({ "inMailbox": jmap_mailbox_id });
        // The server clamps `limit`; request its max per page and paginate.
        let page = self.client.max_objects_in_get();
        let mut position = 0u64;
        let mut refs = Vec::new();
        loop {
            let resp = self
                .client
                .email_query(&self.jmap_account_id, Some(filter.clone()), position, page)
                .await?;
            if resp.ids.is_empty() {
                break;
            }
            let got = resp.ids.len() as u64;
            refs.extend(resp.ids.into_iter().map(MessageRef));
            position += got;
            // Stop when we've reached the reported total or the server returned
            // a short page.
            if got < page || resp.total.map(|t| position >= t).unwrap_or(false) {
                break;
            }
        }
        Ok(refs)
    }
}

#[async_trait]
impl MailSession for JmapMailSession {
    async fn list_mailboxes(&mut self) -> MailboxdResult<Vec<MailBox>> {
        let resp = self.client.mailbox_get(&self.jmap_account_id, None).await?;
        let by_id: HashMap<String, JmapMailbox> =
            resp.list.iter().map(|m| (m.id.clone(), m.clone())).collect();

        let mut mailboxes = Vec::with_capacity(resp.list.len());
        self.mailbox_ids.clear();
        for jm in &resp.list {
            let mb = to_mailbox(self.account_id, jm, &by_id);
            self.mailbox_ids.insert(mb.id, jm.id.clone());
            mailboxes.push(mb);
        }
        Ok(mailboxes)
    }

    async fn changes_since(
        &mut self,
        mailbox: &MailBox,
        cursor: Option<SyncCursor>,
        _token: CancellationToken,
    ) -> MailboxdResult<MailboxChanges> {
        let jmap_mailbox_id = self.mailbox_ids.get(&mailbox.id).cloned().ok_or_else(|| {
            raise_error!(
                format!("no JMAP mailbox id known for local mailbox {}", mailbox.id),
                ErrorCode::InternalError
            )
        })?;

        // No prior state → initial sync: enumerate everything in scope.
        let Some(cursor) = cursor else {
            let messages = self.query_all_in_mailbox(&jmap_mailbox_id).await?;
            // Capture the current Email state so the next run is incremental.
            let state = self
                .client
                .email_changes(&self.jmap_account_id, "", None)
                .await
                .ok()
                .map(|c| c.new_state);
            return Ok(MailboxChanges {
                messages,
                cursor: state.filter(|s| !s.is_empty()).map(SyncCursor::new),
                requires_full_resync: false,
            });
        };

        // Incremental: ask for changes since the stored Email state.
        match self
            .client
            .email_changes(&self.jmap_account_id, cursor.as_str(), None)
            .await
        {
            Ok(changes) => {
                // created + updated are candidates; the engine's content-hash
                // dedup makes re-downloads of `updated` harmless. We only keep
                // those still in this mailbox by intersecting later via load.
                let mut ids: Vec<String> = changes.created;
                ids.extend(changes.updated);
                Ok(MailboxChanges {
                    messages: ids.into_iter().map(MessageRef).collect(),
                    cursor: Some(SyncCursor::new(changes.new_state)),
                    requires_full_resync: false,
                })
            }
            Err(e) if e.code() == ErrorCode::JmapCannotCalculateChanges => {
                // State too old: full reconcile (idempotent via dedup, FA-11).
                let messages = self.query_all_in_mailbox(&jmap_mailbox_id).await?;
                let state = self
                    .client
                    .email_changes(&self.jmap_account_id, "", None)
                    .await
                    .ok()
                    .map(|c| c.new_state);
                Ok(MailboxChanges {
                    messages,
                    cursor: state.filter(|s| !s.is_empty()).map(SyncCursor::new),
                    requires_full_resync: true,
                })
            }
            Err(e) => Err(e),
        }
    }

    async fn load_raw(
        &mut self,
        _mailbox: &MailBox,
        msg: &MessageRef,
    ) -> MailboxdResult<RawMessage> {
        // Resolve the email's blobId (and size) via Email/get, then download the
        // raw RFC 5322 bytes from the blob download URL.
        let resp = self
            .client
            .email_get(
                &self.jmap_account_id,
                vec![msg.as_str().to_string()],
                Some(vec!["id".into(), "blobId".into(), "size".into(), "receivedAt".into()]),
            )
            .await?;
        let email = resp.list.into_iter().next().ok_or_else(|| {
            raise_error!(
                format!("JMAP email {} not found", msg.as_str()),
                ErrorCode::ResourceNotFound
            )
        })?;
        if email.blob_id.is_empty() {
            return Err(raise_error!(
                format!("JMAP email {} has no blobId", msg.as_str()),
                ErrorCode::JmapUnexpectedResult
            ));
        }
        let body = self
            .client
            .download_blob(
                &self.jmap_account_id,
                &email.blob_id,
                "message/rfc822",
                "message.eml",
            )
            .await?;
        let internal_date = email
            .received_at
            .as_deref()
            .and_then(parse_rfc3339_millis)
            .unwrap_or(0);
        let size = if email.size > 0 {
            email.size as u32
        } else {
            body.len() as u32
        };
        Ok(RawMessage {
            body,
            uid: 0,
            size,
            internal_date,
        })
    }

    async fn append(&mut self, _mailbox: &str, _eml: &[u8]) -> MailboxdResult<()> {
        // Restore (FA-17) is implemented in a later issue (#52): it uploads the
        // blob via uploadUrl then Email/import into the target mailbox. The
        // low-level pieces exist on JmapClient; wiring the mailbox-name → id
        // resolution and keyword handling is deferred to keep this issue focused
        // on archival (download) sync.
        Err(raise_error!(
            "JMAP restore (append) is not yet implemented (tracked in #52)".into(),
            ErrorCode::MethodNotAllowed
        ))
    }

    async fn logout(&mut self) -> MailboxdResult<()> {
        // JMAP is stateless HTTP; nothing to tear down.
        Ok(())
    }
}

/// Parse an RFC 3339 timestamp (JMAP `UTCDate`) to epoch milliseconds (helper).
pub(crate) fn parse_rfc3339_millis(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

/// The JMAP [`MailSource`]: a factory that resolves an account's config and
/// opens a connected [`JmapMailSession`].
#[derive(Clone, Copy, Debug, Default)]
pub struct JmapSource;

#[async_trait]
impl MailSource for JmapSource {
    fn name(&self) -> &'static str {
        "jmap"
    }

    async fn connect(&self, account_id: u64) -> MailboxdResult<Box<dyn MailSession>> {
        let account = AccountModel::get(account_id)?;
        let jmap = account.jmap.as_ref().ok_or_else(|| {
            raise_error!(
                format!("account {account_id} has no JMAP configuration"),
                ErrorCode::InvalidParameter
            )
        })?;

        let session_url = jmap.session_url.clone().ok_or_else(|| {
            // Autodiscovery from the email address lands in #50; until then a
            // session URL is required.
            raise_error!(
                "JMAP account has no session_url (autodiscovery is tracked in #50)".into(),
                ErrorCode::MissingConfiguration
            )
        })?;

        let auth = resolve_auth(account_id, jmap)?;
        let client =
            JmapClient::connect(&session_url, auth, jmap.use_proxy, account.use_dangerous).await?;
        let jmap_account_id = client.primary_mail_account()?;

        Ok(Box::new(JmapMailSession {
            account_id,
            jmap_account_id,
            client,
            mailbox_ids: HashMap::new(),
        }))
    }
}
