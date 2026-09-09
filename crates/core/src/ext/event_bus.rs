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

//! Event-bus extension point for security-relevant events.
//!
//! The server calls [`emit`] at security-relevant points (login, view,
//! delete, download, search, user/role/settings changes). Events are
//! fire-and-forget: the emitter never reads back and a failing sink never
//! breaks the originating request.
//!
//! The default [`NoopEventBus`] discards everything. Registering a concrete
//! sink via [`set_event_bus`] (see `crate::audit`) turns this into a real
//! audit log.

use std::net::IpAddr;
use std::sync::{LazyLock, Mutex, RwLock};
use std::time::{Duration, Instant};

/// A security-relevant event worth recording in the audit log.
#[derive(Debug, Clone)]
pub enum Event {
    /// A login attempt. `success` distinguishes accepted from rejected
    /// credentials; `user` is the attempted username either way.
    UserLoggedIn {
        user: String,
        ip: Option<IpAddr>,
        success: bool,
    },
    UserCreated {
        created_by: String,
        new_user: String,
    },
    UserUpdated {
        updated_by: String,
        target_user: String,
    },
    UserRemoved {
        removed_by: String,
        target_user: String,
    },
    RoleCreated {
        created_by: String,
        role_name: String,
    },
    RoleUpdated {
        updated_by: String,
        role_name: String,
    },
    RoleRemoved {
        removed_by: String,
        role_name: String,
    },
    EmailViewed {
        email_id: String,
        user: String,
        ip: Option<IpAddr>,
    },
    EmailDeleted {
        email_id: String,
        user: String,
    },
    AttachmentDownloaded {
        email_id: String,
        content_hash: String,
        user: String,
    },
    SearchPerformed {
        query: String,
        user: String,
    },
    SettingsChanged {
        key: String,
        user: String,
    },
    /// An administrator created and downloaded a full instance backup.
    BackupCreated {
        user: String,
    },
    /// An administrator staged a backup archive for restore-on-restart.
    BackupRestoreStaged {
        user: String,
    },
}

/// A sink that receives every emitted [`Event`].
///
/// Implementations must be cheap and non-blocking enough to run inline in a
/// request handler, and must never panic — the emitter treats delivery as
/// best-effort.
pub trait EventBus: Send + Sync {
    fn emit(&self, event: Event);
}

/// Default sink — all events are discarded.
struct NoopEventBus;
impl EventBus for NoopEventBus {
    fn emit(&self, _event: Event) {}
}

static EVENT_BUS: LazyLock<RwLock<Box<dyn EventBus>>> =
    LazyLock::new(|| RwLock::new(Box::new(NoopEventBus)));

/// Short-window dedup of view/download events.
///
/// The web UI can fire duplicate content requests for the same email (effect
/// double-invocation, remote-content toggle, thread expansion). Deduping here
/// keeps the audit trail to one record per intentional access without hiding
/// genuinely repeated accesses.
static VIEW_DEDUP: LazyLock<Mutex<Vec<(String, Instant)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

const VIEW_DEDUP_WINDOW: Duration = Duration::from_secs(10);

fn dedup_key(event: &Event) -> Option<String> {
    match event {
        Event::EmailViewed { user, email_id, .. } => Some(format!("email.viewed|{user}|{email_id}")),
        Event::AttachmentDownloaded {
            user,
            email_id,
            content_hash,
            ..
        } => Some(format!(
            "attachment.downloaded|{user}|{email_id}|{content_hash}"
        )),
        _ => None,
    }
}

fn is_duplicate_view(event: &Event) -> bool {
    let Some(key) = dedup_key(event) else {
        return false;
    };
    let mut entries = VIEW_DEDUP.lock().unwrap();
    let now = Instant::now();
    entries.retain(|(_, at)| now.duration_since(*at) < VIEW_DEDUP_WINDOW);
    if entries.iter().any(|(k, _)| *k == key) {
        return true;
    }
    entries.push((key, now));
    false
}

/// Register a concrete sink at startup to replace the noop default.
pub fn set_event_bus(bus: Box<dyn EventBus>) {
    *EVENT_BUS.write().unwrap() = bus;
}

/// Fire-and-forget. Called by the server at security-relevant points.
pub fn emit(event: Event) {
    if is_duplicate_view(&event) {
        return;
    }
    EVENT_BUS.read().unwrap().emit(event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_suppresses_repeated_view_within_window() {
        let event = Event::EmailViewed {
            email_id: "e1".into(),
            user: "alice".into(),
            ip: None,
        };
        assert!(!is_duplicate_view(&event));
        assert!(is_duplicate_view(&event));
    }

    #[test]
    fn dedup_ignores_non_view_events() {
        let event = Event::UserLoggedIn {
            user: "alice".into(),
            ip: None,
            success: true,
        };
        assert!(!is_duplicate_view(&event));
        assert!(!is_duplicate_view(&event));
    }

    #[test]
    fn dedup_distinguishes_different_emails() {
        let a = Event::EmailViewed {
            email_id: "a".into(),
            user: "u".into(),
            ip: None,
        };
        let b = Event::EmailViewed {
            email_id: "b".into(),
            user: "u".into(),
            ip: None,
        };
        assert!(!is_duplicate_view(&a));
        assert!(!is_duplicate_view(&b));
    }
}
