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
pub mod model;
pub mod rebuild;
pub mod schema;
pub mod tokenizers;

pub fn fatal_commit(writer: &mut IndexWriter) {
    const MAX_RETRIES: usize = 3;
    const RETRY_DELAY_MS: u64 = 1000;

    for attempt in 0..=MAX_RETRIES {
        match writer.commit() {
            Ok(_) => {
                if attempt > 0 {
                    eprintln!("[INFO] Commit succeeded on attempt {}", attempt + 1);
                }
                return;
            }
            Err(e) if is_retryable_commit_error(&e) => {
                if attempt < MAX_RETRIES {
                    eprintln!(
                        "[WARN] Commit failed (attempt {}/{}): {:?}. Retrying in {}ms...",
                        attempt + 1,
                        MAX_RETRIES + 1,
                        e,
                        RETRY_DELAY_MS * (attempt as u64 + 1)
                    );
                    std::thread::sleep(std::time::Duration::from_millis(
                        RETRY_DELAY_MS * (attempt as u64 + 1),
                    ));
                } else {
                    eprintln!(
                        "[FATAL] Tantivy commit failed after {} attempts: {:?}",
                        MAX_RETRIES + 1,
                        e
                    );
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("[FATAL] Tantivy commit failed with non-retryable error: {e:?}");
                std::process::exit(1);
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
mod commit_retry_tests {
    use super::is_retryable_commit_error;
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
}
