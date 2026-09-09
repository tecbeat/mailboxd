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

use crate::audit::task::AuditCleanTask;
use crate::backup::{ops as backup_ops, task::BackupTask};
use crate::common::periodic::TaskHandle;
use crate::context::MailboxdTask;
use crate::oauth2::{refresh::OAuth2RefreshTask, task::OAuth2CleanTask};
use crate::oidc::task::OidcCleanTask;
use crate::store::tantivy::dedup::DedupTask;

pub struct PeriodicTasks {
    tasks: Vec<TaskHandle>,
}

impl PeriodicTasks {
    pub fn setup() -> Self {
        let mut tasks = Vec::new();
        tasks.push(OAuth2CleanTask::start());
        tasks.push(OAuth2RefreshTask::start());
        tasks.push(OidcCleanTask::start());
        tasks.push(AuditCleanTask::start());
        tasks.push(DedupTask::start());
        // Only run the backup scheduler when a directory and cron schedule are
        // configured; otherwise it would tick uselessly every minute.
        if backup_ops::scheduled_enabled() {
            tasks.push(BackupTask::start());
        }
        Self { tasks }
    }

    pub async fn shutdown(self) {
        for handle in self.tasks {
            handle.stop().await;
        }
    }
}
