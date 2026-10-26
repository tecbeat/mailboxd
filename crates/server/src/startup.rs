//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd.
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

//! The storage steps that run before `initialize()` opens any database,
//! blob or index handle.

use mailboxd_core::error::MailboxdResult;
use mailboxd_migrate::{reindex::ReindexOutcome, AutoMigrateOutcome};
use tracing::{error, info};

use crate::rest::maintenance::{
    Maintenance, LAYOUT_CHECK_FAILED, MIGRATION_FAILED, MIGRATION_REQUIRED, REINDEX_FAILED,
    RESTORE_FAILED,
};

/// The startup steps, in the order they run. Passed in so tests can
/// replace them.
pub struct StorageSteps<R, M, C, X>
where
    R: FnOnce() -> MailboxdResult<()>,
    M: FnOnce() -> MailboxdResult<AutoMigrateOutcome>,
    C: FnOnce() -> std::io::Result<bool>,
    X: FnOnce() -> MailboxdResult<Option<ReindexOutcome>>,
{
    /// Applies a staged backup restore. It replaces the on-disk state
    /// wholesale, so it runs first.
    pub restore: R,
    /// Migrates an existing data volume up to v2.
    pub migrate: M,
    /// Checks that the data layout is one this version can open.
    pub check_layout: C,
    /// Rebuilds search indexes of an older format.
    pub reindex: X,
}

/// Runs the steps and stops at the first failure. Returns the maintenance
/// page to serve instead of starting, or `None` when startup can go on. The
/// error itself only goes to the log.
pub fn prepare_storage<R, M, C, X>(steps: StorageSteps<R, M, C, X>) -> Option<&'static Maintenance>
where
    R: FnOnce() -> MailboxdResult<()>,
    M: FnOnce() -> MailboxdResult<AutoMigrateOutcome>,
    C: FnOnce() -> std::io::Result<bool>,
    X: FnOnce() -> MailboxdResult<Option<ReindexOutcome>>,
{
    if let Err(e) = (steps.restore)() {
        error!("Failed to apply staged restore: {:#?}", e);
        return Some(&RESTORE_FAILED);
    }

    match (steps.migrate)() {
        Ok(AutoMigrateOutcome::AlreadyCurrent) => {
            info!("Storage layout already current; nothing to migrate")
        }
        Ok(AutoMigrateOutcome::MigratedV1 {
            emails,
            attachments,
        }) => info!(emails, attachments, "Migrated v1.x storage to v2"),
        Ok(AutoMigrateOutcome::MigratedV037 { migrated, skipped }) => {
            info!(migrated, skipped, "Migrated legacy v0.3.7 storage to v2")
        }
        Err(e) => {
            error!("Failed to migrate data layout: {:#?}", e);
            return Some(&MIGRATION_FAILED);
        }
    }

    match (steps.check_layout)() {
        Ok(true) => {}
        Ok(false) => {
            error!("Incompatible data format detected.");
            error!("Please run: mailboxd-admin");
            return Some(&MIGRATION_REQUIRED);
        }
        Err(e) => {
            error!("Failed to check data layout: {:#?}", e);
            return Some(&LAYOUT_CHECK_FAILED);
        }
    }

    match (steps.reindex)() {
        Ok(None) => {}
        Ok(Some(outcome)) => {
            info!(
                emails = outcome.emails,
                attachments = outcome.attachments,
                skipped = outcome.skipped,
                "Rebuilt the search index; remove {:?} once search works as expected",
                outcome.backup_dir
            );
            if let Some(report) = outcome.report {
                info!(
                    "Documents not carried over completely are listed in {:?}",
                    report
                );
            }
        }
        Err(e) => {
            error!("Failed to rebuild the search index: {:#?}", e);
            return Some(&REINDEX_FAILED);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use mailboxd_core::{error::code::ErrorCode, raise_error};

    use super::*;

    fn failure<T>() -> MailboxdResult<T> {
        Err(raise_error!("boom".into(), ErrorCode::InternalError))
    }

    fn rebuilt(report: Option<PathBuf>) -> ReindexOutcome {
        ReindexOutcome {
            emails: 3,
            attachments: 1,
            skipped: 0,
            missing_bodies: 0,
            missing_texts: 0,
            backup_dir: PathBuf::from("mailboxd-indices.old-1"),
            report,
            elapsed: std::time::Duration::from_secs(1),
        }
    }

    /// Runs `prepare_storage` with the given step results and records which
    /// steps ran.
    fn run(
        restore: MailboxdResult<()>,
        migrate: MailboxdResult<AutoMigrateOutcome>,
        check_layout: std::io::Result<bool>,
        reindex: MailboxdResult<Option<ReindexOutcome>>,
    ) -> (Option<&'static str>, Vec<&'static str>) {
        let ran = RefCell::new(Vec::new());
        let page = prepare_storage(StorageSteps {
            restore: || {
                ran.borrow_mut().push("restore");
                restore
            },
            migrate: || {
                ran.borrow_mut().push("migrate");
                migrate
            },
            check_layout: || {
                ran.borrow_mut().push("check_layout");
                check_layout
            },
            reindex: || {
                ran.borrow_mut().push("reindex");
                reindex
            },
        });
        (page.map(|p| p.heading), ran.into_inner())
    }

    const ALL: [&str; 4] = ["restore", "migrate", "check_layout", "reindex"];

    #[test]
    fn starts_when_every_step_succeeds() {
        for migrate in [
            AutoMigrateOutcome::AlreadyCurrent,
            AutoMigrateOutcome::MigratedV1 {
                emails: 2,
                attachments: 1,
            },
            AutoMigrateOutcome::MigratedV037 {
                migrated: 2,
                skipped: 1,
            },
        ] {
            for reindex in [
                None,
                Some(rebuilt(None)),
                Some(rebuilt(Some(PathBuf::from("report.csv")))),
            ] {
                assert_eq!(
                    run(Ok(()), Ok(migrate), Ok(true), Ok(reindex)),
                    (None, ALL.to_vec())
                );
            }
        }
    }

    #[test]
    fn a_failed_restore_stops_before_the_migration() {
        let (page, ran) = run(
            failure(),
            Ok(AutoMigrateOutcome::AlreadyCurrent),
            Ok(true),
            Ok(None),
        );
        assert_eq!(page, Some(RESTORE_FAILED.heading));
        assert_eq!(ran, ["restore"]);
    }

    #[test]
    fn a_failed_migration_stops_before_the_layout_check() {
        let (page, ran) = run(Ok(()), failure(), Ok(true), Ok(None));
        assert_eq!(page, Some(MIGRATION_FAILED.heading));
        assert_eq!(ran, ["restore", "migrate"]);
    }

    #[test]
    fn an_old_layout_asks_for_a_manual_migration() {
        let (page, ran) = run(
            Ok(()),
            Ok(AutoMigrateOutcome::AlreadyCurrent),
            Ok(false),
            Ok(None),
        );
        assert_eq!(page, Some(MIGRATION_REQUIRED.heading));
        assert_eq!(ran, ["restore", "migrate", "check_layout"]);
    }

    #[test]
    fn a_failed_layout_check_stops_before_the_rebuild() {
        let (page, ran) = run(
            Ok(()),
            Ok(AutoMigrateOutcome::AlreadyCurrent),
            Err(std::io::Error::other("permission denied")),
            Ok(None),
        );
        assert_eq!(page, Some(LAYOUT_CHECK_FAILED.heading));
        assert_eq!(ran, ["restore", "migrate", "check_layout"]);
    }

    #[test]
    fn a_failed_rebuild_serves_the_maintenance_page() {
        let (page, ran) = run(
            Ok(()),
            Ok(AutoMigrateOutcome::AlreadyCurrent),
            Ok(true),
            failure(),
        );
        assert_eq!(page, Some(REINDEX_FAILED.heading));
        assert_eq!(ran, ALL);
    }
}
