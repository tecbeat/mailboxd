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


use mailboxd_core::context::executors::MAILBOXD_CONTEXT;
use chrono::Local;
use poem_openapi::Object;
use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;
use timeago::Formatter;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Object)]
pub struct MailboxdStatus {
    /// The service uptime in milliseconds since it started.
    pub uptime_ms: i64,
    /// A human-readable string indicating the time elapsed since the service started (e.g., "2 hours ago").
    pub timeago: String,
    /// The timezone in which the service is operating (e.g., "UTC" or "Asia/Tokyo").
    pub timezone: String,
    /// The version of the RustMailer service currently running.
    pub version: String,
}

impl MailboxdStatus {
    pub fn get() -> Self {
        Self {
            uptime_ms: MAILBOXD_CONTEXT.uptime_ms(),
            timeago: Formatter::new()
                .convert(Duration::from_millis(MAILBOXD_CONTEXT.uptime_ms() as u64)),
            timezone: Local::now().offset().to_string(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}
