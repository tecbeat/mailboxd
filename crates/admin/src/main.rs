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

use console::style;
use dialoguer::{theme::ColorfulTheme, Select};

use crate::{migrate_v037::handle_migration_v037, migrate_v1::handle_migrate_v1, reset::handle_reset_password};

pub mod legacy;
pub mod meta;
pub mod migrate_store_v2;
pub mod migrate_v037;
pub mod migrate_v1;
pub mod reset;


fn main() {
    // Unattended startup migration. The container entrypoint runs
    // `mailboxd-admin --auto-migrate` before the server so that an existing
    // v1.x (fjall) data volume is converted to the v2 mailboxd-blob layout with
    // no operator interaction. No-op on fresh installs and volumes already on
    // v2, so it is safe to run on every start.
    if std::env::args().any(|arg| arg == "--auto-migrate") {
        run_auto_migrate();
        return;
    }

    run_interactive();
}

fn run_auto_migrate() {
    use std::path::PathBuf;

    let root_dir = match std::env::var("MAILBOXD_ROOT_DIR") {
        Ok(value) if !value.is_empty() => PathBuf::from(value),
        _ => {
            eprintln!("[auto-migrate] MAILBOXD_ROOT_DIR is not set; skipping storage migration");
            return;
        }
    };

    let env_dir = |key: &str| {
        std::env::var(key)
            .ok()
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let index_dir = env_dir("MAILBOXD_INDEX_DIR");
    let data_dir = env_dir("MAILBOXD_DATA_DIR");
    let index_base = index_dir.clone().unwrap_or_else(|| root_dir.clone());
    let data_base = data_dir.clone().unwrap_or_else(|| root_dir.clone());

    // Adopt an existing Bichon volume first (rename `bichon-*` → `mailboxd-*`).
    // The server performs the same rename on startup, but doing it here — before
    // the storage-generation migrations — lets a fjall-era Bichon volume convert
    // to v2 in a single boot instead of needing one crash cycle for the server
    // to rename it first.
    if let Err(error) = mailboxd_core::migrate::adopt_bichon_layout(&index_base, &data_base) {
        eprintln!("[auto-migrate] failed to adopt Bichon data layout: {error:#?}");
        std::process::exit(1);
    }

    // 1. v1.x fjall → v2 mailboxd-blob.
    match migrate_v1::migrate_v1_to_v2(&root_dir, &data_base, 1000) {
        Ok(Some(outcome)) => {
            println!(
                "[auto-migrate] migrated v1.x storage to v2: {} email + {} attachment blobs",
                outcome.emails, outcome.attachments
            );
            return;
        }
        Ok(None) => {}
        Err(error) => {
            eprintln!("[auto-migrate] v1.x storage migration failed: {error:#?}");
            // Refuse to start the server on unmigrated data rather than risk
            // running against a half-converted volume.
            std::process::exit(1);
        }
    }

    // 2. legacy v0.3.7 Tantivy → v2.
    let v037_paths =
        migrate_v037::V037Paths::derive(&root_dir, index_dir.as_deref(), data_dir.as_deref());
    match migrate_v037::migrate_v037_to_v2(&v037_paths, 3000) {
        Ok(Some(outcome)) => {
            println!(
                "[auto-migrate] migrated legacy v0.3.7 storage to v2: {} messages migrated, {} skipped",
                outcome.migrated, outcome.skipped
            );
        }
        Ok(None) => {
            println!("[auto-migrate] storage layout already current; nothing to do");
        }
        Err(error) => {
            eprintln!("[auto-migrate] v0.3.7 storage migration failed: {error:#?}");
            std::process::exit(1);
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
