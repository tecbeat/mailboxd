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

use crate::{
    audit,
    common::periodic::{PeriodicTask, TaskHandle},
    context::MailboxdTask,
};
use std::time::Duration;

const TASK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// Periodic pruning of audit entries past their retention window.
pub struct AuditCleanTask;

impl MailboxdTask for AuditCleanTask {
    fn start() -> TaskHandle {
        let periodic_task = PeriodicTask::new("audit-cleanup");

        let task = move |_: Option<u64>| {
            Box::pin(async move {
                audit::clean()?;
                Ok(())
            })
        };

        periodic_task.start(task, None, TASK_INTERVAL, false, false)
    }
}
