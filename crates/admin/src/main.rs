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

use std::path::PathBuf;

use console::style;
use dialoguer::{theme::ColorfulTheme, Select};
use mailboxd_core::error::MailboxdResult;
use mailboxd_migrate::{
    migrate_v037::handle_migration_v037,
    migrate_v1::handle_migrate_v1,
    reindex::{reindex, ReindexOutcome},
    run_auto_migrate, AutoMigrateOutcome, AutoMigratePaths,
};

use crate::reindex::{handle_reindex, reindex_outcome_lines};
use crate::reset::handle_reset_password;

pub mod reindex;
pub mod reset;

fn main() {
    // Standalone migration entry point; the server also runs it on startup.
    if std::env::args().any(|arg| arg == "--auto-migrate") {
        std::process::exit(auto_migrate_from_env());
    }

    run_interactive();
}

/// Run the migration and the search index rebuild against the
/// `MAILBOXD_*_DIR` environment, returning a process exit code (0 = success
/// or nothing to do, 1 = migration or rebuild failed).
fn auto_migrate_from_env() -> i32 {
    let root_dir = match std::env::var("MAILBOXD_ROOT_DIR") {
        Ok(value) if !value.is_empty() => PathBuf::from(value),
        _ => {
            eprintln!("[auto-migrate] MAILBOXD_ROOT_DIR is not set; skipping storage migration");
            return 0;
        }
    };

    let env_dir = |key: &str| {
        std::env::var(key)
            .ok()
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let paths = AutoMigratePaths {
        root_dir,
        index_dir: env_dir("MAILBOXD_INDEX_DIR"),
        data_dir: env_dir("MAILBOXD_DATA_DIR"),
    };
    auto_migrate(&paths)
}

/// Run the migration and the search index rebuild against `paths`, printing
/// their results, and return the process exit code.
fn auto_migrate(paths: &AutoMigratePaths) -> i32 {
    match migration_report(&run_auto_migrate(paths)) {
        Ok(line) => println!("{line}"),
        Err(line) => {
            eprintln!("{line}");
            return 1;
        }
    }
    match reindex_report(&reindex(paths, false)) {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            0
        }
        Err(line) => {
            eprintln!("{line}");
            1
        }
    }
}

/// The line printed for a storage migration result: `Ok` on success, `Err`
/// on failure.
fn migration_report(result: &MailboxdResult<AutoMigrateOutcome>) -> Result<String, String> {
    match result {
        Ok(AutoMigrateOutcome::MigratedV1 {
            emails,
            attachments,
        }) => Ok(format!(
            "[auto-migrate] migrated v1.x storage to v2: {emails} email + {attachments} attachment blobs"
        )),
        Ok(AutoMigrateOutcome::MigratedV037 { migrated, skipped }) => Ok(format!(
            "[auto-migrate] migrated legacy v0.3.7 storage to v2: {migrated} messages migrated, {skipped} skipped"
        )),
        Ok(AutoMigrateOutcome::AlreadyCurrent) => {
            Ok("[auto-migrate] storage layout already current; nothing to do".to_string())
        }
        Err(error) => Err(format!(
            "[auto-migrate] storage migration failed: {error:#?}"
        )),
    }
}

/// The lines printed for a search index rebuild result: `Ok` on success,
/// `Err` on failure.
fn reindex_report(result: &MailboxdResult<Option<ReindexOutcome>>) -> Result<Vec<String>, String> {
    match result {
        Ok(None) => Ok(vec![
            "[auto-migrate] search index needs no rebuild".to_string()
        ]),
        Ok(Some(outcome)) => Ok(reindex_outcome_lines("[auto-migrate] ", outcome)),
        Err(error) => Err(format!(
            "[auto-migrate] search index rebuild failed: {error:#?}"
        )),
    }
}

#[tokio::main]
async fn run_interactive() {
    let theme = ColorfulTheme::default();
    println!(
        "\n{}\n",
        style("MAILBOXD ADMINISTRATIVE TOOL").bold().bright().cyan()
    );

    let main_options = vec![
        "Reset Admin Password",
        "Migrate Legacy v0.3.7 Storage to v2.x (mailboxd-blob)",
        "Migrate v1.x Storage to v2.x (Fjall → mailboxd-blob)",
        "Rebuild Search Index",
        "Exit",
    ];

    let selection = Select::with_theme(&theme)
        .with_prompt("Select an operation")
        .default(0)
        .items(&main_options)
        .interact()
        .unwrap();

    match selection {
        0 => handle_reset_password(&theme),
        1 => handle_migration_v037(&theme),
        2 => handle_migrate_v1(&theme),
        3 => handle_reindex(&theme),
        _ => {
            println!("{}", style("Exiting...").dim());
        }
    }
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{fake_tantivy_dir, TestDir};
    use mailboxd_core::{
        migrate::{write_storage_version, CURRENT_STORAGE_VERSION},
        store::tantivy::validate_facet,
    };
    use std::time::Duration;

    fn paths(root: &std::path::Path) -> AutoMigratePaths {
        AutoMigratePaths {
            root_dir: root.to_path_buf(),
            index_dir: None,
            data_dir: None,
        }
    }

    fn outcome() -> ReindexOutcome {
        ReindexOutcome {
            emails: 3,
            attachments: 2,
            skipped: 1,
            missing_bodies: 0,
            missing_texts: 0,
            backup_dir: PathBuf::from("/data/mailboxd-indices.old-1"),
            report: None,
            elapsed: Duration::from_millis(1500),
        }
    }

    #[test]
    fn migration_results_are_reported_in_one_line() {
        assert_eq!(
            migration_report(&Ok(AutoMigrateOutcome::MigratedV1 {
                emails: 4,
                attachments: 2
            })),
            Ok("[auto-migrate] migrated v1.x storage to v2: 4 email + 2 attachment blobs".into())
        );
        assert_eq!(
            migration_report(&Ok(AutoMigrateOutcome::MigratedV037 {
                migrated: 5,
                skipped: 1
            })),
            Ok("[auto-migrate] migrated legacy v0.3.7 storage to v2: 5 messages migrated, 1 skipped".into())
        );
        assert_eq!(
            migration_report(&Ok(AutoMigrateOutcome::AlreadyCurrent)),
            Ok("[auto-migrate] storage layout already current; nothing to do".into())
        );
        let error = migration_report(&Err(validate_facet("bad").unwrap_err())).unwrap_err();
        assert!(
            error.starts_with("[auto-migrate] storage migration failed: "),
            "{error}"
        );
    }

    #[test]
    fn rebuild_results_are_reported_with_the_auto_migrate_prefix() {
        assert_eq!(
            reindex_report(&Ok(None)),
            Ok(vec![
                "[auto-migrate] search index needs no rebuild".to_string()
            ])
        );
        let lines = reindex_report(&Ok(Some(outcome()))).unwrap();
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line.starts_with("[auto-migrate] ")));
        let error = reindex_report(&Err(validate_facet("bad").unwrap_err())).unwrap_err();
        assert!(
            error.starts_with("[auto-migrate] search index rebuild failed: "),
            "{error}"
        );
    }

    #[test]
    fn an_empty_root_needs_nothing() {
        let root = TestDir::new();
        assert_eq!(auto_migrate(&paths(root.path())), 0);
    }

    #[test]
    fn a_failed_migration_exits_with_one() {
        // A v0.3.7 layout next to v2 blobs: an interrupted migration.
        let root = TestDir::new();
        fake_tantivy_dir(&root.path().join("envelope"));
        fake_tantivy_dir(&root.path().join("eml"));
        std::fs::create_dir_all(root.path().join("mailboxd-storage").join("blobs")).unwrap();

        assert_eq!(auto_migrate(&paths(root.path())), 1);
        let error = migration_report(&run_auto_migrate(&paths(root.path()))).unwrap_err();
        assert!(error.contains("interrupted migration"), "{error}");
    }

    #[test]
    fn a_failed_rebuild_exits_with_one() {
        let root = TestDir::new();
        write_storage_version(root.path(), CURRENT_STORAGE_VERSION).unwrap();
        fake_tantivy_dir(&root.path().join("mailboxd-indices").join("mail_metadata"));

        assert_eq!(auto_migrate(&paths(root.path())), 1);
        let error = reindex_report(&reindex(&paths(root.path()), false)).unwrap_err();
        assert!(error.contains("cannot open index"), "{error}");
    }
}
