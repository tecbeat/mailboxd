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

use std::path::PathBuf;

use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use mailboxd_migrate::{
    reindex::{reindex, ReindexOutcome},
    AutoMigratePaths,
};

/// Interactive menu entry: rebuild the search indexes, whatever their format.
pub fn handle_reindex(theme: &ColorfulTheme) {
    println!("\n{}", style("REBUILD SEARCH INDEX").bold().yellow());
    println!(
        "{}\n",
        style(
            "Rebuilds the email and attachment indexes from the current ones and the stored\n\
             messages. mailboxd must be stopped. The previous index is kept next to the new\n\
             one; remove it once search works as expected."
        )
        .dim()
    );

    let default_root = std::env::var("MAILBOXD_ROOT_DIR").unwrap_or_default();
    let root_dir: String = Input::with_theme(theme)
        .with_prompt("Enter --mailboxd-root-dir")
        .with_initial_text(default_root)
        .validate_with(|input: &String| -> Result<(), &str> {
            let path = PathBuf::from(input.trim());
            if !path.is_absolute() {
                return Err("Path must be absolute.");
            }
            if !path.is_dir() {
                return Err("Directory does not exist.");
            }
            Ok(())
        })
        .interact_text()
        .unwrap();
    let paths = AutoMigratePaths {
        root_dir: PathBuf::from(root_dir.trim()),
        index_dir: optional_dir(theme, "--mailboxd-index-dir", "MAILBOXD_INDEX_DIR"),
        data_dir: optional_dir(theme, "--mailboxd-data-dir", "MAILBOXD_DATA_DIR"),
    };

    let confirmed = Confirm::with_theme(theme)
        .with_prompt("Is mailboxd stopped, and should the search index be rebuilt now?")
        .default(false)
        .interact()
        .unwrap_or(false);
    if !confirmed {
        println!("{}", style("Rebuild cancelled.").dim());
        return;
    }

    match reindex(&paths, true) {
        Ok(Some(outcome)) => print_reindex_outcome("", &outcome),
        Ok(None) => println!(
            "{}",
            style("No search index found; nothing to rebuild.").yellow()
        ),
        Err(error) => eprintln!(
            "{}",
            style(format!("Search index rebuild failed: {error:#?}")).red()
        ),
    }
}

fn optional_dir(theme: &ColorfulTheme, flag: &str, env: &str) -> Option<PathBuf> {
    let input: String = Input::with_theme(theme)
        .with_prompt(format!(
            "Enter {flag} (leave blank to use the root directory)"
        ))
        .with_initial_text(std::env::var(env).unwrap_or_default())
        .allow_empty(true)
        .validate_with(|input: &String| -> Result<(), &str> {
            let input = input.trim();
            if input.is_empty() || PathBuf::from(input).is_dir() {
                Ok(())
            } else {
                Err("Directory does not exist.")
            }
        })
        .interact_text()
        .unwrap();
    let input = input.trim();
    (!input.is_empty()).then(|| PathBuf::from(input))
}

pub fn print_reindex_outcome(prefix: &str, outcome: &ReindexOutcome) {
    println!(
        "{prefix}rebuilt the search index in {:.1}s: {} emails, {} attachments, {} skipped, \
         {} without body text, {} attachments without text",
        outcome.elapsed.as_secs_f64(),
        outcome.emails,
        outcome.attachments,
        outcome.skipped,
        outcome.missing_bodies,
        outcome.missing_texts
    );
    println!(
        "{prefix}the previous index is kept in {}; remove it once search works as expected",
        outcome.backup_dir.display()
    );
    if let Some(report) = &outcome.report {
        println!(
            "{prefix}documents not carried over completely are listed in {}",
            report.display()
        );
    }
}
