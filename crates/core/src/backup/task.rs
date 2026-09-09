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
    backup::ops,
    common::periodic::{PeriodicTask, TaskHandle},
    context::MailboxdTask,
};
use chrono::Local;
use std::time::Duration;

/// How often the scheduler wakes to check whether a scheduled backup is due.
/// The cron expression controls the actual cadence; this only bounds how
/// promptly a due backup is noticed.
const TASK_INTERVAL: Duration = Duration::from_secs(60);

/// Periodic evaluation of the backup cron schedule. Runs a scheduled backup
/// whenever the configured cron expression indicates one is due.
pub struct BackupTask;

impl MailboxdTask for BackupTask {
    fn start() -> TaskHandle {
        let periodic_task = PeriodicTask::new("scheduled-backup");

        // Baseline for the very first schedule evaluation before any archive
        // exists, so a fresh instance does not immediately fire on boot.
        let started_at = Local::now();

        let task = move |_: Option<u64>| {
            Box::pin(async move {
                ops::run_scheduled_if_due(started_at).await?;
                Ok(())
            })
        };

        periodic_task.start(task, None, TASK_INTERVAL, false, false)
    }
}
