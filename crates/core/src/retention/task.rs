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
    common::periodic::{PeriodicTask, TaskHandle},
    context::MailboxdTask,
    retention,
};
use std::time::Duration;

/// How often the retention sweep runs. The first run is one interval after
/// startup.
pub const SWEEP_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// Periodic deletion of mail past its account's retention window.
pub struct RetentionSweepTask;

impl MailboxdTask for RetentionSweepTask {
    fn start() -> TaskHandle {
        start_every(SWEEP_INTERVAL)
    }
}

fn start_every(interval: Duration) -> TaskHandle {
    let task = |_: Option<u64>| Box::pin(retention::run_sweep());
    PeriodicTask::new("retention-sweep").start(task, None, interval, false, false)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    #[test]
    fn the_sweep_runs_every_six_hours() {
        assert_eq!(SWEEP_INTERVAL, Duration::from_secs(6 * 60 * 60));
    }

    /// Start the task on `runtime` with a short interval.
    pub(in crate::retention) fn start_on(
        runtime: &tokio::runtime::Runtime,
        interval: Duration,
    ) -> TaskHandle {
        let _entered = runtime.enter();
        start_every(interval)
    }
}
