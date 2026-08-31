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
use mailboxd_migrate::{
    migrate_v037::handle_migration_v037, migrate_v1::handle_migrate_v1, run_auto_migrate,
    AutoMigrateOutcome, AutoMigratePaths,
};

use crate::reset::handle_reset_password;

pub mod reset;

fn main() {
    // Standalone migration entry point; the server also runs it on startup.
    if std::env::args().any(|arg| arg == "--auto-migrate") {
        std::process::exit(auto_migrate_from_env());
    }

    run_interactive();
}

/// Run the migration against the `MAILBOXD_*_DIR` environment, returning a
/// process exit code (0 = success or nothing to do, 1 = migration failed).
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

    match run_auto_migrate(&paths) {
        Ok(AutoMigrateOutcome::MigratedV1 {
            emails,
            attachments,
        }) => {
            println!(
                "[auto-migrate] migrated v1.x storage to v2: {emails} email + {attachments} attachment blobs"
            );
            0
        }
        Ok(AutoMigrateOutcome::MigratedV037 { migrated, skipped }) => {
            println!(
                "[auto-migrate] migrated legacy v0.3.7 storage to v2: {migrated} messages migrated, {skipped} skipped"
            );
            0
        }
        Ok(AutoMigrateOutcome::AlreadyCurrent) => {
            println!("[auto-migrate] storage layout already current; nothing to do");
            0
        }
        Err(error) => {
            eprintln!("[auto-migrate] storage migration failed: {error:#?}");
            1
        }
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
        _ => {
            println!("{}", style("Exiting...").dim());
        }
    }
}
