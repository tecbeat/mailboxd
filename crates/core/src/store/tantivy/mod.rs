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

use tantivy::{schema::Facet, IndexWriter};

use crate::{
    error::{code::ErrorCode, MailboxdResult},
    raise_error,
};

pub mod attachment;
pub mod dedup;
pub mod dedup_cache;
pub mod envelope;
pub mod fields;
pub mod filter;
pub mod format;
pub mod model;
pub mod rebuild;
pub mod schema;
pub mod sort;
pub mod tokenizers;

pub fn fatal_commit(writer: &mut IndexWriter) {
    if let Err(message) =
        commit_with_retry(&mut || writer.commit().map(|_| ()), &std::thread::sleep)
    {
        eprintln!("[FATAL] {message}");
        std::process::exit(1);
    }
}

/// Run `commit`, retrying retryable failures with a growing delay passed to
/// `sleep`. Returns the message of the failure that ends the retries.
fn commit_with_retry(
    commit: &mut dyn FnMut() -> tantivy::Result<()>,
    sleep: &dyn Fn(std::time::Duration),
) -> Result<(), String> {
    const MAX_RETRIES: usize = 3;
    const RETRY_DELAY_MS: u64 = 1000;

    let mut attempt = 0;
    loop {
        match commit() {
            Ok(()) => {
                if attempt > 0 {
                    eprintln!("[INFO] Commit succeeded on attempt {}", attempt + 1);
                }
                return Ok(());
            }
            Err(e) if is_retryable_commit_error(&e) && attempt < MAX_RETRIES => {
                eprintln!(
                    "[WARN] Commit failed (attempt {}/{}): {:?}. Retrying in {}ms...",
                    attempt + 1,
                    MAX_RETRIES + 1,
                    e,
                    RETRY_DELAY_MS * (attempt as u64 + 1)
                );
                sleep(std::time::Duration::from_millis(
                    RETRY_DELAY_MS * (attempt as u64 + 1),
                ));
                attempt += 1;
            }
            Err(e) if is_retryable_commit_error(&e) => {
                return Err(format!(
                    "Tantivy commit failed after {} attempts: {:?}",
                    MAX_RETRIES + 1,
                    e
                ));
            }
            Err(e) => {
                return Err(format!(
                    "Tantivy commit failed with non-retryable error: {e:?}"
                ));
            }
        }
    }
}

pub fn validate_facet(tag: &str) -> MailboxdResult<()> {
    Facet::from_text(tag)
        .map(|_| ())
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InvalidParameter))
}

/// Transient I/O failures, including a file briefly locked while opening it
/// for writing, are retried; anything else is fatal.
fn is_retryable_commit_error(error: &tantivy::TantivyError) -> bool {
    matches!(
        error,
        tantivy::TantivyError::IoError(_) | tantivy::TantivyError::OpenWriteError(_)
    )
}

#[cfg(test)]
mod facet_tests {
    use super::validate_facet;

    #[test]
    fn only_absolute_facet_paths_are_valid() {
        assert!(validate_facet("/inbox").is_ok());
        assert!(validate_facet("inbox").is_err());
    }
}

#[cfg(test)]
mod commit_retry_tests {
    use super::{commit_with_retry, is_retryable_commit_error};
    use std::{path::PathBuf, sync::Arc};
    use tantivy::{directory::error::OpenWriteError, TantivyError};

    #[test]
    fn io_and_open_write_errors_are_retried() {
        let io = std::io::Error::other("locked");
        assert!(is_retryable_commit_error(&TantivyError::IoError(Arc::new(io))));
        assert!(is_retryable_commit_error(&TantivyError::OpenWriteError(
            OpenWriteError::FileAlreadyExists(PathBuf::from("meta.json"))
        )));
    }

    #[test]
    fn other_errors_are_fatal() {
        assert!(!is_retryable_commit_error(&TantivyError::InvalidArgument(
            "bad".into()
        )));
    }

    fn locked() -> TantivyError {
        TantivyError::IoError(Arc::new(std::io::Error::other("locked")))
    }

    /// Run `commit_with_retry` over `results`, returning its result, the
    /// number of commit attempts and the delays slept, in milliseconds.
    fn run_commits(results: Vec<tantivy::Result<()>>) -> (Result<(), String>, usize, Vec<u128>) {
        let mut results = results.into_iter();
        let mut attempts = 0;
        let delays = std::cell::RefCell::new(Vec::new());
        let result = commit_with_retry(
            &mut || {
                attempts += 1;
                results.next().unwrap()
            },
            &|delay| delays.borrow_mut().push(delay.as_millis()),
        );
        (result, attempts, delays.into_inner())
    }

    #[test]
    fn a_transient_commit_failure_is_retried_with_a_growing_delay() {
        let (result, attempts, delays) = run_commits(vec![Err(locked()), Err(locked()), Ok(())]);
        assert_eq!(result, Ok(()));
        assert_eq!(attempts, 3);
        assert_eq!(delays, [1000, 2000]);
    }

    #[test]
    fn a_persistent_commit_failure_gives_up_after_four_attempts() {
        let (result, attempts, delays) = run_commits((0..4).map(|_| Err(locked())).collect());
        let message = result.unwrap_err();
        assert!(
            message.starts_with("Tantivy commit failed after 4 attempts"),
            "{message}"
        );
        assert_eq!(attempts, 4);
        assert_eq!(delays, [1000, 2000, 3000]);
    }

    #[test]
    fn a_non_retryable_commit_failure_is_not_retried() {
        let (result, attempts, delays) =
            run_commits(vec![Err(TantivyError::InvalidArgument("bad".into()))]);
        let message = result.unwrap_err();
        assert!(message.contains("non-retryable"), "{message}");
        assert_eq!(attempts, 1);
        assert!(delays.is_empty());
    }
}
