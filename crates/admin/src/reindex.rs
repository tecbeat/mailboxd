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
use mailboxd_core::error::MailboxdResult;
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
        .validate_with(|input: &String| validate_root_dir(input))
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

    match rebuild_report(&reindex(&paths, true)) {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
        }
        Err(line) => eprintln!("{line}"),
    }
}

/// The lines printed for an interactive rebuild result: `Ok` on success,
/// `Err` on failure.
fn rebuild_report(result: &MailboxdResult<Option<ReindexOutcome>>) -> Result<Vec<String>, String> {
    match result {
        Ok(Some(outcome)) => Ok(reindex_outcome_lines("", outcome)),
        Ok(None) => Ok(vec![style("No search index found; nothing to rebuild.")
            .yellow()
            .to_string()]),
        Err(error) => Err(style(format!("Search index rebuild failed: {error:#?}"))
            .red()
            .to_string()),
    }
}

fn validate_root_dir(input: &str) -> Result<(), &'static str> {
    let path = PathBuf::from(input.trim());
    if !path.is_absolute() {
        return Err("Path must be absolute.");
    }
    if !path.is_dir() {
        return Err("Directory does not exist.");
    }
    Ok(())
}

fn optional_dir(theme: &ColorfulTheme, flag: &str, env: &str) -> Option<PathBuf> {
    let input: String = Input::with_theme(theme)
        .with_prompt(format!(
            "Enter {flag} (leave blank to use the root directory)"
        ))
        .with_initial_text(std::env::var(env).unwrap_or_default())
        .allow_empty(true)
        .validate_with(|input: &String| validate_optional_dir(input))
        .interact_text()
        .unwrap();
    parse_optional_dir(&input)
}

fn validate_optional_dir(input: &str) -> Result<(), &'static str> {
    let input = input.trim();
    if input.is_empty() || PathBuf::from(input).is_dir() {
        Ok(())
    } else {
        Err("Directory does not exist.")
    }
}

/// A blank answer means "use the root directory".
fn parse_optional_dir(input: &str) -> Option<PathBuf> {
    let input = input.trim();
    (!input.is_empty()).then(|| PathBuf::from(input))
}

/// The lines describing a completed rebuild, each starting with `prefix`.
pub fn reindex_outcome_lines(prefix: &str, outcome: &ReindexOutcome) -> Vec<String> {
    let mut lines = vec![
        format!(
            "{prefix}rebuilt the search index in {:.1}s: {} emails, {} attachments, {} skipped, \
             {} without body text, {} attachments without text",
            outcome.elapsed.as_secs_f64(),
            outcome.emails,
            outcome.attachments,
            outcome.skipped,
            outcome.missing_bodies,
            outcome.missing_texts
        ),
        format!(
            "{prefix}the previous index is kept in {}; remove it once search works as expected",
            outcome.backup_dir.display()
        ),
    ];
    if let Some(report) = &outcome.report {
        lines.push(format!(
            "{prefix}documents not carried over completely are listed in {}",
            report.display()
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;
    use mailboxd_core::store::tantivy::validate_facet;
    use std::time::Duration;

    fn outcome(report: Option<PathBuf>) -> ReindexOutcome {
        ReindexOutcome {
            emails: 3,
            attachments: 2,
            skipped: 1,
            missing_bodies: 4,
            missing_texts: 5,
            backup_dir: PathBuf::from("/data/mailboxd-indices.old-1"),
            report,
            elapsed: Duration::from_millis(1500),
        }
    }

    fn plain(styled: &str) -> String {
        console::strip_ansi_codes(styled).into_owned()
    }

    #[test]
    fn a_rebuild_is_described_with_its_counts_and_backup() {
        assert_eq!(
            reindex_outcome_lines("> ", &outcome(None)),
            [
                "> rebuilt the search index in 1.5s: 3 emails, 2 attachments, 1 skipped, \
                 4 without body text, 5 attachments without text",
                "> the previous index is kept in /data/mailboxd-indices.old-1; \
                 remove it once search works as expected",
            ]
        );
    }

    #[test]
    fn a_skip_report_is_pointed_to() {
        let lines =
            reindex_outcome_lines("", &outcome(Some("/data/reindex-skipped.ndjson".into())));
        assert_eq!(lines.len(), 3);
        assert_eq!(
            lines[2],
            "documents not carried over completely are listed in /data/reindex-skipped.ndjson"
        );
    }

    #[test]
    fn interactive_rebuild_results_are_reported() {
        let lines = rebuild_report(&Ok(Some(outcome(None)))).unwrap();
        assert!(
            lines[0].starts_with("rebuilt the search index"),
            "{lines:?}"
        );

        let lines = rebuild_report(&Ok(None)).unwrap();
        assert_eq!(
            lines.iter().map(|l| plain(l)).collect::<Vec<_>>(),
            ["No search index found; nothing to rebuild."]
        );

        let error = rebuild_report(&Err(validate_facet("bad").unwrap_err())).unwrap_err();
        assert!(
            plain(&error).starts_with("Search index rebuild failed: "),
            "{error}"
        );
    }

    #[test]
    fn the_root_dir_must_be_an_existing_absolute_dir() {
        let dir = TestDir::new();
        let existing = dir.path().to_str().unwrap();
        assert_eq!(validate_root_dir(existing), Ok(()));
        assert_eq!(validate_root_dir(&format!("  {existing}  ")), Ok(()));
        assert_eq!(
            validate_root_dir("relative/dir"),
            Err("Path must be absolute.")
        );
        assert_eq!(
            validate_root_dir(dir.path().join("missing").to_str().unwrap()),
            Err("Directory does not exist.")
        );
    }

    #[test]
    fn an_optional_dir_may_be_blank_or_must_exist() {
        let dir = TestDir::new();
        let existing = dir.path().to_str().unwrap();
        assert_eq!(validate_optional_dir("  "), Ok(()));
        assert_eq!(validate_optional_dir(existing), Ok(()));
        assert_eq!(
            validate_optional_dir(dir.path().join("missing").to_str().unwrap()),
            Err("Directory does not exist.")
        );

        assert_eq!(parse_optional_dir("   "), None);
        assert_eq!(
            parse_optional_dir(&format!(" {existing} ")),
            Some(dir.path().to_path_buf())
        );
    }
}
