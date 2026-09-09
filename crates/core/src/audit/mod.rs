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

//! Persistent audit log.
//!
//! [`AuditLogSink`] is the concrete [`EventBus`] sink that turns every emitted
//! [`Event`] into a stored [`AuditEntry`]. Install it once at startup with
//! [`install`]; from then on the security-relevant events emitted across the
//! server are queryable by administrators and pruned by [`clean`] according to
//! `MAILBOXD_AUDIT_RETENTION_DAYS`.

pub mod task;

use crate::{
    common::paginated::Paginated,
    database::{
        batch_delete_impl, insert_impl, list_all_impl, manager::DB_MANAGER, paginate_impl,
        MemDbModel,
    },
    error::MailboxdResult,
    ext::event_bus::{set_event_bus, Event, EventBus},
    generate_token,
    settings::cli::SETTINGS,
    utc_now,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

const MS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

/// A single recorded security-relevant event.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct AuditEntry {
    /// Storage key: zero-padded creation timestamp plus random suffix, so the
    /// natural key order is chronological and ties are unique.
    pub id: String,
    /// Creation timestamp in milliseconds since the Unix epoch.
    pub created_at: i64,
    /// Machine-readable event type, e.g. `email.viewed`.
    pub event_type: String,
    /// Username that performed the action (for a login, the attempted name).
    pub actor: String,
    /// The object the action targeted (affected user, role name, email id).
    pub target: Option<String>,
    /// Client IP address, when known.
    pub ip: Option<String>,
    /// Extra context (search query, content hash, changed settings key).
    pub detail: Option<String>,
    /// Whether the action succeeded. Only meaningful for login attempts.
    pub success: bool,
}

impl MemDbModel for AuditEntry {
    fn collection() -> &'static str {
        "audit_log"
    }
    fn key(&self) -> String {
        self.id.clone()
    }
}

impl AuditEntry {
    fn build(
        event_type: &str,
        actor: String,
        target: Option<String>,
        ip: Option<String>,
        detail: Option<String>,
        success: bool,
    ) -> Self {
        let created_at = utc_now!();
        Self {
            id: format!("{:013}-{}", created_at, generate_token!(64)),
            created_at,
            event_type: event_type.to_string(),
            actor,
            target,
            ip,
            detail,
            success,
        }
    }

    /// Flatten an [`Event`] into a storable audit entry.
    pub fn from_event(event: Event) -> Self {
        match event {
            Event::UserLoggedIn { user, ip, success } => Self::build(
                "user.login",
                user,
                None,
                ip.map(|ip| ip.to_string()),
                None,
                success,
            ),
            Event::UserCreated {
                created_by,
                new_user,
            } => Self::build("user.created", created_by, Some(new_user), None, None, true),
            Event::UserUpdated {
                updated_by,
                target_user,
            } => Self::build(
                "user.updated",
                updated_by,
                Some(target_user),
                None,
                None,
                true,
            ),
            Event::UserRemoved {
                removed_by,
                target_user,
            } => Self::build(
                "user.removed",
                removed_by,
                Some(target_user),
                None,
                None,
                true,
            ),
            Event::RoleCreated {
                created_by,
                role_name,
            } => Self::build("role.created", created_by, Some(role_name), None, None, true),
            Event::RoleUpdated {
                updated_by,
                role_name,
            } => Self::build("role.updated", updated_by, Some(role_name), None, None, true),
            Event::RoleRemoved {
                removed_by,
                role_name,
            } => Self::build("role.removed", removed_by, Some(role_name), None, None, true),
            Event::EmailViewed {
                email_id,
                user,
                ip,
            } => Self::build(
                "email.viewed",
                user,
                Some(email_id),
                ip.map(|ip| ip.to_string()),
                None,
                true,
            ),
            Event::EmailDeleted { email_id, user } => {
                Self::build("email.deleted", user, Some(email_id), None, None, true)
            }
            Event::AttachmentDownloaded {
                email_id,
                content_hash,
                user,
            } => Self::build(
                "attachment.downloaded",
                user,
                Some(email_id),
                None,
                Some(content_hash),
                true,
            ),
            Event::SearchPerformed { query, user } => {
                Self::build("search.performed", user, None, None, Some(query), true)
            }
            Event::SettingsChanged { key, user } => {
                Self::build("settings.changed", user, None, None, Some(key), true)
            }
            Event::BackupCreated { user } => {
                Self::build("backup.created", user, None, None, None, true)
            }
            Event::BackupRestoreStaged { user } => {
                Self::build("backup.restore_staged", user, None, None, None, true)
            }
        }
    }
}

/// Persist a single audit entry.
pub fn record(entry: AuditEntry) -> MailboxdResult<()> {
    insert_impl(DB_MANAGER.db(), entry)?;
    Ok(())
}

/// Return one page of audit entries, newest first.
pub fn list(page: Option<u64>, page_size: Option<u64>) -> MailboxdResult<Paginated<AuditEntry>> {
    paginate_impl::<AuditEntry>(DB_MANAGER.db(), page, page_size, Some(true))
}

/// Delete audit entries older than the configured retention window.
///
/// A retention of `0` disables cleanup and keeps entries indefinitely.
pub fn clean() -> MailboxdResult<()> {
    let retention_days = SETTINGS.mailboxd_audit_retention_days;
    if retention_days == 0 {
        return Ok(());
    }
    let cutoff = utc_now!() - retention_days as i64 * MS_PER_DAY;
    let all = list_all_impl::<AuditEntry>(DB_MANAGER.db())?;
    let to_delete: Vec<String> = all
        .into_iter()
        .filter(|e| e.created_at < cutoff)
        .map(|e| e.id)
        .collect();
    if !to_delete.is_empty() {
        batch_delete_impl::<AuditEntry>(DB_MANAGER.db(), to_delete)?;
    }
    Ok(())
}

/// The concrete sink that persists emitted events as audit entries.
pub struct AuditLogSink;

impl EventBus for AuditLogSink {
    fn emit(&self, event: Event) {
        if let Err(e) = record(AuditEntry::from_event(event)) {
            warn!("failed to persist audit event: {:#?}", e);
        }
    }
}

/// Register [`AuditLogSink`] as the process-wide event sink.
pub fn install() {
    set_event_bus(Box::new(AuditLogSink));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn maps_login_failure() {
        let entry = AuditEntry::from_event(Event::UserLoggedIn {
            user: "alice".into(),
            ip: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            success: false,
        });
        assert_eq!(entry.event_type, "user.login");
        assert_eq!(entry.actor, "alice");
        assert!(!entry.success);
        assert_eq!(entry.ip.as_deref(), Some("127.0.0.1"));
    }

    #[test]
    fn maps_attachment_download_detail() {
        let entry = AuditEntry::from_event(Event::AttachmentDownloaded {
            email_id: "e1".into(),
            content_hash: "abc123".into(),
            user: "bob".into(),
            });
        assert_eq!(entry.event_type, "attachment.downloaded");
        assert_eq!(entry.target.as_deref(), Some("e1"));
        assert_eq!(entry.detail.as_deref(), Some("abc123"));
    }

    #[test]
    fn key_is_chronological_prefix() {
        let entry = AuditEntry::from_event(Event::SearchPerformed {
            query: "invoice".into(),
            user: "carol".into(),
        });
        assert!(entry.id.starts_with(&format!("{:013}", entry.created_at)));
        assert_eq!(entry.detail.as_deref(), Some("invoice"));
    }
}
