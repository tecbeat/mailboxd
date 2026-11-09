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

// use poem_openapi::Object;
pub mod history;
pub mod pst;
pub mod reader;
use std::{collections::HashMap, path::Path, sync::RwLock};

pub use history::ImportHistory;
use serde::{Deserialize, Serialize};

use crate::{
    account::migration::{AccountModel, AccountType},
    base64_decode_url_safe,
    archive::imap::mailbox::{Attribute, AttributeEnum, MailBox},
    envelope::extractor::{extract_envelope_from_eml, ExtractOutcome},
    error::{code::ErrorCode, MailboxdResult},
    raise_error,
    settings::dir::{available_space, DATA_DIR_MANAGER},
    utils::create_hash,
};

/// Maximum byte size of an individual email message after splitting (100 MB).
const MAX_SINGLE_EML_BYTES: usize = 100 * 1024 * 1024;

/// Max file size accepted via the web upload endpoint.
pub const MAX_WEB_EML_BYTES: usize = 100 * 1024 * 1024; // 100 MB
pub const MAX_WEB_MBOX_BYTES: usize = 1024 * 1024 * 1024; // 1 GB

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct BatchEmlRequest {
    pub account_id: u64,
    pub mail_folder: String,
    /// A list of emails in base64-encoded format. Each element represents one
    /// .eml file.
    pub emls: Vec<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct FailedItemDetail {
    /// The index (0-based) of the failed item.
    pub index: usize,
    /// The error message that caused the import to fail.
    pub error_message: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct BatchEmlResult {
    /// Total number of emails processed.
    pub total: usize,
    /// Number of emails successfully imported.
    pub success: usize,
    /// Number of duplicate emails skipped (content hash already existed).
    pub duplicates: usize,
    /// Number of emails skipped because they are older than the account's
    /// retention window.
    pub outside_retention: usize,
    /// Number of emails failed to import.
    pub failed: usize,
    /// A list of details for failed imports.
    pub failed_details: Vec<FailedItemDetail>,
}

impl BatchEmlResult {
    /// A batch failed only when every message failed; skipped duplicates and
    /// mail outside the retention window count as handled.
    pub fn status(&self) -> ImportStatus {
        if self.failed > 0
            && self.success == 0
            && self.duplicates == 0
            && self.outside_retention == 0
        {
            ImportStatus::Failed
        } else {
            ImportStatus::Completed
        }
    }
}

/// How many messages of an import ended in each successful outcome.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OutcomeCounts {
    pub success: usize,
    pub duplicates: usize,
    pub outside_retention: usize,
}

impl OutcomeCounts {
    pub fn record(&mut self, outcome: ExtractOutcome) {
        match outcome {
            ExtractOutcome::Imported => self.success += 1,
            ExtractOutcome::Duplicate => self.duplicates += 1,
            ExtractOutcome::OutsideRetention => self.outside_retention += 1,
        }
    }
}

pub struct ImportEmls;

impl ImportEmls {
    pub async fn do_import(mut request: BatchEmlRequest) -> MailboxdResult<BatchEmlResult> {
        let account = AccountModel::check_account_exists(request.account_id)?;

        if !account.enabled {
            return Err(raise_error!(
                "The account is disabled and cannot be used for this operation.".into(),
                ErrorCode::InvalidParameter
            ));
        }

        let mailbox_id = match account.account_type {
            // Server-backed sources (IMAP, JMAP) require the target folder to
            // already exist from a prior sync.
            AccountType::IMAP | AccountType::JMAP => {
                let all_mailboxes = MailBox::list_all(account.id)?;
                let mailbox = all_mailboxes
                    .into_iter()
                    .find(|m| m.name == request.mail_folder);

                match mailbox {
                    Some(mailbox) => mailbox.id,
                    None => return Err(raise_error!(
                        format!("Mail folder '{}' not found for account ID {}. The target folder must exist before importing.",
                                request.mail_folder,
                                request.account_id).into(),
                        ErrorCode::ResourceNotFound
                    )),
                }
            }
            AccountType::NoSync => {
                let mailbox = MailBox {
                    id: create_hash(request.account_id, &request.mail_folder),
                    account_id: request.account_id,
                    name: request.mail_folder.clone(),
                    delimiter: Some("/".to_string()),
                    attributes: vec![Attribute {
                        attr: AttributeEnum::Extension,
                        extension: Some("CreatedByMailboxd".into()),
                    }],
                    exists: 0,
                    unseen: None,
                    uid_next: None,
                    uid_validity: None,
                    highest_uid: None,
                    sync_cursor: None,
                };
                let mailbox_id = mailbox.id;
                // Upsert the mailbox, creating it if it doesn't exist
                MailBox::batch_upsert(&[mailbox])?;
                mailbox_id
            }
        };

        let account_id = account.id;
        let mut counts = OutcomeCounts::default();
        let mut failed_details: Vec<FailedItemDetail> = Vec::new(); // Store failure details

        let total = request.emls.len();
        let mut index: usize = 0;
        while let Some(eml_base64) = request.emls.pop() {
            let decoded = match base64_decode_url_safe!(eml_base64.as_bytes()) {
                Ok(bytes) => bytes,
                Err(e) => {
                    let error_msg =
                        format!("Failed to decode base64 EML at index {}: {:?}", index, e);
                    tracing::error!("{}", error_msg);
                    failed_details.push(FailedItemDetail {
                        index,
                        error_message: error_msg,
                    });
                    index += 1;
                    continue;
                }
            };
            // eml_base64 string dropped here — frees base64 memory before parsing

            if decoded.len() > MAX_SINGLE_EML_BYTES {
                let size_mb = decoded.len() as f64 / 1024.0 / 1024.0;
                let error_msg = format!(
                    "Email at index {} is {:.1} MB (limit 50 MB). Skipping.",
                    index, size_mb,
                );
                tracing::warn!("{}", error_msg);
                failed_details.push(FailedItemDetail {
                    index,
                    error_message: error_msg,
                });
                index += 1;
                continue;
            }

            match extract_envelope_from_eml(&decoded, account_id, mailbox_id).await {
                Ok(outcome) => counts.record(outcome),
                Err(e) => {
                    let error_msg = format!(
                        "Failed to extract envelope from EML at index {}: {:?}",
                        index, e
                    );
                    tracing::error!("{}", error_msg);
                    failed_details.push(FailedItemDetail {
                        index,
                        error_message: error_msg,
                    });
                    index += 1;
                    continue;
                }
            };
            index += 1;
        }

        let failed_count = failed_details.len();

        Ok(BatchEmlResult {
            total,
            success: counts.success,
            duplicates: counts.duplicates,
            outside_retention: counts.outside_retention,
            failed: failed_count,
            failed_details, // Return the list of failure details
        })
    }
}

// ── File upload import ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum ImportStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ImportProgress {
    pub import_id: String,
    pub status: ImportStatus,
    pub format: String,
    pub total: usize,
    pub success: usize,
    pub duplicates: usize,
    #[serde(default)]
    pub outside_retention: usize,
    pub failed: usize,
    pub failed_details: Vec<FailedItemDetail>,
}

static PROGRESS_STORE: std::sync::LazyLock<RwLock<HashMap<String, ImportProgress>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn get_import_progress(import_id: &str) -> Option<ImportProgress> {
    PROGRESS_STORE.read().ok()?.get(import_id).cloned()
}

pub fn update_progress(import_id: &str, progress: ImportProgress) {
    if let Ok(mut store) = PROGRESS_STORE.write() {
        store.insert(import_id.to_string(), progress);
    }
}

/// Check free disk space (in bytes) on the temp directory's filesystem.
/// Reports plenty of space if the file system cannot be determined.
pub fn check_temp_disk_space() -> MailboxdResult<u64> {
    Ok(available_space(&DATA_DIR_MANAGER.temp_dir))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    Eml,
    Mbox,
    Pst,
}

pub fn detect_format(bytes: &[u8], file_name: &str) -> Option<FileFormat> {
    // PST files start with OLE2 compound document magic bytes
    if bytes.len() >= 8 && &bytes[..8] == b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1" {
        return Some(FileFormat::Pst);
    }

    // MBOX files start with "From " (note the trailing space after From)
    if bytes.starts_with(b"From ") {
        // Double-check: look for a valid date after the first "From " line
        // MBOX format: "From sender@host DayOfWeek Mon DD HH:MM:SS YYYY"
        if let Some(first_newline) = bytes.iter().position(|&b| b == b'\n') {
            let from_line = std::str::from_utf8(&bytes[..first_newline]).unwrap_or("");
            let parts: Vec<&str> = from_line.split_whitespace().collect();
            if parts.len() >= 7 {
                return Some(FileFormat::Mbox);
            }
        }
    }
    // EML: starts with a header line or "Return-Path:", "Received:", "From:",
    // "Date:", etc. Or check extension
    if bytes.starts_with(b"Return-Path:")
        || bytes.starts_with(b"Received:")
        || bytes.starts_with(b"Date:")
        || bytes.starts_with(b"From:")
        || bytes.starts_with(b"Subject:")
        || bytes.starts_with(b"To:")
        || bytes.starts_with(b"Message-ID:")
    {
        return Some(FileFormat::Eml);
    }
    // Fallback: check file extension
    let lower = file_name.to_lowercase();
    if lower.ends_with(".eml") {
        Some(FileFormat::Eml)
    } else if lower.ends_with(".mbox") {
        Some(FileFormat::Mbox)
    } else if lower.ends_with(".pst") {
        Some(FileFormat::Pst)
    } else {
        None
    }
}

/// Check whether `bytes` looks like a text file by inspecting the first chunk.
/// Returns `true` if it passes, `false` if it appears to be binary (video,
/// executable, etc.).
///
/// Email files (EML/MBOX) are text-based with printable ASCII, whitespace, and
/// optional UTF-8. Binary files like video contain null bytes and high ratios
/// of non-printable control characters.
pub fn detect_text_file(bytes: &[u8]) -> bool {
    let check_len = bytes.len().min(8192);
    if check_len == 0 {
        return false;
    }
    let sample = &bytes[..check_len];

    // Null bytes are a strong binary indicator
    if sample.contains(&0x00) {
        return false;
    }

    let mut printable = 0usize;
    let mut total = 0usize;

    let mut i = 0;
    while i < sample.len() {
        total += 1;
        let b = sample[i];

        if b.is_ascii_graphic() || b.is_ascii_whitespace() {
            // Printable ASCII + whitespace (space, tab, CR, LF)
            printable += 1;
        } else if b == 0x1b {
            // ESC — common in terminal sequences, rare in email
            // Count as printable to avoid false positives
            printable += 1;
        } else if b >= 0x80 {
            // UTF-8 continuation or multi-byte lead byte — allow.
            // Check that we have a valid UTF-8 sequence ahead.
            let seq_len = match b {
                b if b & 0xE0 == 0xC0 => 2,
                b if b & 0xF0 == 0xE0 => 3,
                b if b & 0xF8 == 0xF0 => 4,
                _ => 0,
            };
            if seq_len > 0 && i + seq_len <= sample.len() {
                let valid = std::str::from_utf8(&sample[i..i + seq_len]).is_ok();
                if valid {
                    printable += 1;
                    i += 1; // lead byte counted, continuations counted in loop
                }
                // if invalid, don't count as printable
            }
            // standalone continuation byte — not printable
        }
        // Other control characters (0x01-0x1F except whitespace/Esc) are not counted as
        // printable

        i += 1;
    }

    // Require at least 90% printable characters
    printable as f64 / total as f64 >= 0.90
}

/// Validate that the target account exists, is enabled, and is NoSync type.
fn validate_import_account(account_id: u64) -> MailboxdResult<AccountModel> {
    let account = AccountModel::check_account_exists(account_id)?;
    if !account.enabled {
        return Err(raise_error!(
            "The account is disabled.".into(),
            ErrorCode::InvalidParameter
        ));
    }
    if !matches!(account.account_type, AccountType::NoSync) {
        return Err(raise_error!(
            "Import is only allowed for NoSync accounts. IMAP accounts sync from the server."
                .into(),
            ErrorCode::InvalidParameter
        ));
    }
    Ok(account)
}

/// Resolve or create a mailbox/folder for the given account.
pub(super) fn resolve_mailbox(account: &AccountModel, folder: &str) -> MailboxdResult<u64> {
    match account.account_type {
        AccountType::IMAP | AccountType::JMAP => {
            // Shouldn't reach here (validated above), but handle gracefully
            let all_mailboxes = MailBox::list_all(account.id)?;
            let mailbox = all_mailboxes.into_iter().find(|m| m.name == folder);
            match mailbox {
                Some(m) => Ok(m.id),
                None => Err(raise_error!(
                    format!("Mail folder '{}' not found.", folder).into(),
                    ErrorCode::ResourceNotFound
                )),
            }
        }
        AccountType::NoSync => {
            let mailbox = MailBox {
                id: create_hash(account.id, folder),
                account_id: account.id,
                name: folder.to_string(),
                delimiter: Some("/".to_string()),
                attributes: vec![Attribute {
                    attr: AttributeEnum::Extension,
                    extension: Some("CreatedByMailboxd".into()),
                }],
                exists: 0,
                unseen: None,
                uid_next: None,
                uid_validity: None,
                highest_uid: None,
                sync_cursor: None,
            };
            let mailbox_id = mailbox.id;
            MailBox::batch_upsert(&[mailbox])?;
            Ok(mailbox_id)
        }
    }
}

/// Resolve or create a mailbox for a given account_id and folder name.
/// Used by PST import to create per-folder mailboxes.
pub fn resolve_mailbox_by_account_id(account_id: u64, folder: &str) -> MailboxdResult<u64> {
    let account = AccountModel::check_account_exists(account_id)?;
    resolve_mailbox(&account, folder)
}

/// Process an uploaded file (EML or MBOX) and import into the given
/// account/folder. This runs synchronously and should be spawned on a
/// background thread.
///
/// For MBOX files, the file is memory-mapped via `memmap2` and messages are
/// yielded one at a time — the full file is never loaded into RAM. Individual
/// messages exceeding `MAX_SINGLE_EML_BYTES` (100 MB) are skipped.
pub fn process_uploaded_file(
    import_id: &str,
    file_path: &Path,
    file_name: &str,
    account_id: u64,
    folder: &str,
    user_id: u64,
) {
    let account = match validate_import_account(account_id) {
        Ok(a) => a,
        Err(e) => {
            let progress = ImportProgress {
                import_id: import_id.to_string(),
                status: ImportStatus::Failed,
                format: "unknown".to_string(),
                total: 0,
                success: 0,
                duplicates: 0,
                outside_retention: 0,
                failed: 0,
                failed_details: vec![FailedItemDetail {
                    index: 0,
                    error_message: format!("Account validation failed: {:?}", e),
                }],
            };
            update_progress(import_id, progress.clone());
            history::save_import_history(user_id, account_id, folder, &progress);
            return;
        }
    };

    let mailbox_id = match resolve_mailbox(&account, folder) {
        Ok(id) => id,
        Err(e) => {
            let progress = ImportProgress {
                import_id: import_id.to_string(),
                status: ImportStatus::Failed,
                format: "unknown".to_string(),
                total: 0,
                success: 0,
                duplicates: 0,
                outside_retention: 0,
                failed: 0,
                failed_details: vec![FailedItemDetail {
                    index: 0,
                    error_message: format!("Mailbox resolution failed: {:?}", e),
                }],
            };
            update_progress(import_id, progress.clone());
            history::save_import_history(user_id, account_id, folder, &progress);
            return;
        }
    };

    // Read a small prefix for format detection
    let format = match detect_format_from_file(file_path, file_name) {
        Ok(f) => f,
        Err(e) => {
            let progress = ImportProgress {
                import_id: import_id.to_string(),
                status: ImportStatus::Failed,
                format: "unknown".to_string(),
                total: 0,
                success: 0,
                duplicates: 0,
                outside_retention: 0,
                failed: 0,
                failed_details: vec![FailedItemDetail {
                    index: 0,
                    error_message: format!("{:?}", e),
                }],
            };
            update_progress(import_id, progress.clone());
            history::save_import_history(user_id, account_id, folder, &progress);
            let _ = std::fs::remove_file(file_path);
            return;
        }
    };

    match format {
        FileFormat::Eml => process_eml_file(
            import_id, file_path, account_id, mailbox_id, user_id, folder,
        ),
        FileFormat::Mbox => process_mbox_file(
            import_id, file_path, account_id, mailbox_id, user_id, folder,
        ),
        FileFormat::Pst => process_pst_upload(
            import_id, file_path, account_id, mailbox_id, user_id, folder,
        ),
    }
}

/// Detect format from a file by reading only the first few KB.
fn detect_format_from_file(file_path: &Path, file_name: &str) -> MailboxdResult<FileFormat> {
    use std::io::Read;
    let mut file = std::fs::File::open(file_path).map_err(|e| {
        raise_error!(
            format!("Failed to open file: {}", e),
            ErrorCode::InternalError
        )
    })?;
    let mut buf = vec![0u8; 8192];
    let n = file.read(&mut buf).unwrap_or(0);
    buf.truncate(n);

    detect_format(&buf, file_name).ok_or_else(|| {
        raise_error!(
            "Unknown file format. Supported: .eml, .mbox, .pst".into(),
            ErrorCode::InvalidParameter
        )
    })
}

/// Process a single EML file. The file is at most `MAX_WEB_EML_BYTES` (100 MB),
/// so reading it entirely is safe.
fn process_eml_file(
    import_id: &str,
    file_path: &Path,
    account_id: u64,
    mailbox_id: u64,
    user_id: u64,
    folder: &str,
) {
    let file_bytes = match std::fs::read(file_path) {
        Ok(b) => b,
        Err(e) => {
            fail_progress(
                import_id,
                "eml",
                &format!("Failed to read file: {}", e),
                user_id,
                account_id,
                folder,
            );
            let _ = std::fs::remove_file(file_path);
            return;
        }
    };

    let total = 1;
    update_progress(
        import_id,
        ImportProgress {
            import_id: import_id.to_string(),
            status: ImportStatus::Processing,
            format: "eml".to_string(),
            total,
            success: 0,
            duplicates: 0,
            outside_retention: 0,
            failed: 0,
            failed_details: vec![],
        },
    );

    let (counts, failed_details) = process_single_eml(&file_bytes, 0, account_id, mailbox_id);

    // Clean up
    let _ = std::fs::remove_file(file_path);

    let final_progress = ImportProgress {
        import_id: import_id.to_string(),
        status: ImportStatus::Completed,
        format: "eml".to_string(),
        total,
        success: counts.success,
        duplicates: counts.duplicates,
        outside_retention: counts.outside_retention,
        failed: failed_details.len(),
        failed_details,
    };
    history::save_import_history(user_id, account_id, folder, &final_progress);
    update_progress(import_id, final_progress);
}

/// Process an MBOX file using memory-mapped I/O. Messages are yielded one at a
/// time by `MboxReader` — the full file is never loaded into RAM.
fn process_mbox_file(
    import_id: &str,
    file_path: &Path,
    account_id: u64,
    mailbox_id: u64,
    user_id: u64,
    folder: &str,
) {
    let mbox = match reader::MboxFile::from_file(file_path) {
        Ok(m) => m,
        Err(e) => {
            fail_progress(
                import_id,
                "mbox",
                &format!("Failed to open MBOX file: {}", e),
                user_id,
                account_id,
                folder,
            );
            let _ = std::fs::remove_file(file_path);
            return;
        }
    };

    // First pass: count total messages (MboxReader is lazy, so this is O(n) but
    // cheap)
    let total = mbox.iter().count();

    update_progress(
        import_id,
        ImportProgress {
            import_id: import_id.to_string(),
            status: ImportStatus::Processing,
            format: "mbox".to_string(),
            total,
            success: 0,
            duplicates: 0,
            outside_retention: 0,
            failed: 0,
            failed_details: vec![],
        },
    );

    let mut counts = OutcomeCounts::default();
    let mut failed_details: Vec<FailedItemDetail> = Vec::new();

    for (index, entry) in mbox.iter().enumerate() {
        let eml_bytes = entry.data;

        if eml_bytes.len() > MAX_SINGLE_EML_BYTES {
            let size_mb = eml_bytes.len() as f64 / 1024.0 / 1024.0;
            failed_details.push(FailedItemDetail {
                index,
                error_message: format!(
                    "Email at index {} is {:.1} MB (limit {} MB). Skipping.",
                    index,
                    size_mb,
                    MAX_SINGLE_EML_BYTES / 1024 / 1024
                ),
            });
            continue;
        }

        match futures::executor::block_on(extract_envelope_from_eml(
            eml_bytes, account_id, mailbox_id,
        )) {
            Ok(outcome) => counts.record(outcome),
            Err(e) => {
                failed_details.push(FailedItemDetail {
                    index,
                    error_message: format!("{:?}", e),
                });
            }
        };

        // Update progress every 100 items
        if index % 100 == 0 || index == total - 1 {
            update_progress(
                import_id,
                ImportProgress {
                    import_id: import_id.to_string(),
                    status: ImportStatus::Processing,
                    format: "mbox".to_string(),
                    total,
                    success: counts.success,
                    duplicates: counts.duplicates,
                    outside_retention: counts.outside_retention,
                    failed: failed_details.len(),
                    failed_details: failed_details.clone(),
                },
            );
        }
    }

    // Clean up temp file (drop the mmap first — MboxFile owns it)
    drop(mbox);
    let _ = std::fs::remove_file(file_path);

    let final_progress = ImportProgress {
        import_id: import_id.to_string(),
        status: ImportStatus::Completed,
        format: "mbox".to_string(),
        total,
        success: counts.success,
        duplicates: counts.duplicates,
        outside_retention: counts.outside_retention,
        failed: failed_details.len(),
        failed_details,
    };
    history::save_import_history(user_id, account_id, folder, &final_progress);
    update_progress(import_id, final_progress);
}

/// Process a single EML byte slice and return the outcome counts and the
/// failure details.
fn process_single_eml(
    eml_bytes: &[u8],
    index: usize,
    account_id: u64,
    mailbox_id: u64,
) -> (OutcomeCounts, Vec<FailedItemDetail>) {
    let mut counts = OutcomeCounts::default();
    if eml_bytes.len() > MAX_SINGLE_EML_BYTES {
        let size_mb = eml_bytes.len() as f64 / 1024.0 / 1024.0;
        return (
            counts,
            vec![FailedItemDetail {
                index,
                error_message: format!(
                    "Email is {:.1} MB (limit {} MB). Skipping.",
                    size_mb,
                    MAX_SINGLE_EML_BYTES / 1024 / 1024
                ),
            }],
        );
    }

    match futures::executor::block_on(extract_envelope_from_eml(eml_bytes, account_id, mailbox_id))
    {
        Ok(outcome) => {
            counts.record(outcome);
            (counts, vec![])
        }
        Err(e) => (
            counts,
            vec![FailedItemDetail {
                index,
                error_message: format!("{:?}", e),
            }],
        ),
    }
}

/// Process a PST file uploaded via the web UI.
/// Two-pass approach: count messages first, then process with periodic progress
/// updates.
fn process_pst_upload(
    import_id: &str,
    file_path: &Path,
    account_id: u64,
    _mailbox_id: u64, // ignored; PST creates its own mailboxes per folder
    user_id: u64,
    folder: &str,
) {
    // Pass 1: count total messages
    let total = match pst::count_pst_messages(file_path) {
        Ok(n) => n,
        Err(e) => {
            fail_progress(
                import_id,
                "pst",
                &format!("{:?}", e),
                user_id,
                account_id,
                folder,
            );
            let _ = std::fs::remove_file(file_path);
            return;
        }
    };

    update_progress(
        import_id,
        ImportProgress {
            import_id: import_id.to_string(),
            status: ImportStatus::Processing,
            format: "pst".to_string(),
            total,
            success: 0,
            duplicates: 0,
            outside_retention: 0,
            failed: 0,
            failed_details: vec![],
        },
    );

    // Pass 2: process messages with progress updates
    let mut counts = OutcomeCounts::default();
    let mut failed_details: Vec<FailedItemDetail> = Vec::new();
    let mut index: usize = 0;

    let pst_store = match outlook_pst::open_store(file_path) {
        Ok(s) => s,
        Err(e) => {
            fail_progress(
                import_id,
                "pst",
                &format!("{:?}", e),
                user_id,
                account_id,
                folder,
            );
            let _ = std::fs::remove_file(file_path);
            return;
        }
    };

    let ipm_sub_tree = match pst_store.properties().ipm_sub_tree_entry_id() {
        Ok(id) => id,
        Err(e) => {
            fail_progress(
                import_id,
                "pst",
                &format!("{:?}", e),
                user_id,
                account_id,
                folder,
            );
            let _ = std::fs::remove_file(file_path);
            return;
        }
    };

    let ipm_subtree_folder = match pst_store.open_folder(&ipm_sub_tree) {
        Ok(f) => f,
        Err(e) => {
            fail_progress(
                import_id,
                "pst",
                &format!("{:?}", e),
                user_id,
                account_id,
                folder,
            );
            let _ = std::fs::remove_file(file_path);
            return;
        }
    };

    // Progress callback: update progress every 50 messages
    let import_id = import_id.to_string();
    let format_str = "pst".to_string();
    pst::process_folder_with_progress(
        &ipm_subtree_folder,
        "", // parent_path starts empty
        account_id,
        total, // pass pre-counted total for accurate progress
        &mut counts,
        &mut failed_details,
        &mut index,
        &|counts: OutcomeCounts, actual_failed| {
            update_progress(
                &import_id,
                ImportProgress {
                    import_id: import_id.clone(),
                    status: ImportStatus::Processing,
                    format: format_str.clone(),
                    total,
                    success: counts.success,
                    duplicates: counts.duplicates,
                    outside_retention: counts.outside_retention,
                    failed: actual_failed,
                    failed_details: vec![],
                },
            );
        },
    );

    // Clean up temp file
    let _ = std::fs::remove_file(file_path);

    let final_progress = ImportProgress {
        import_id: import_id.to_string(),
        status: ImportStatus::Completed,
        format: "pst".to_string(),
        total,
        success: counts.success,
        duplicates: counts.duplicates,
        outside_retention: counts.outside_retention,
        failed: failed_details.len(),
        failed_details,
    };
    history::save_import_history(user_id, account_id, folder, &final_progress);
    update_progress(&import_id, final_progress);
}

/// Record a fatal failure and save history.
fn fail_progress(
    import_id: &str,
    format: &str,
    message: &str,
    user_id: u64,
    account_id: u64,
    folder: &str,
) {
    let progress = ImportProgress {
        import_id: import_id.to_string(),
        status: ImportStatus::Failed,
        format: format.to_string(),
        total: 0,
        success: 0,
        duplicates: 0,
        outside_retention: 0,
        failed: 0,
        failed_details: vec![FailedItemDetail {
            index: 0,
            error_message: message.to_string(),
        }],
    };
    update_progress(import_id, progress.clone());
    history::save_import_history(user_id, account_id, folder, &progress);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_disk_space_check_reports_free_space() {
        // The temp dir always lives on some file system; when it cannot be
        // identified the check reports plenty of space rather than failing.
        let free = check_temp_disk_space().expect("disk space check never errors");
        assert!(free > 0, "temp file system must report free space");
    }

    #[test]
    fn outcome_counts_tally_each_outcome() {
        let mut counts = OutcomeCounts::default();
        for outcome in [
            ExtractOutcome::Imported,
            ExtractOutcome::Imported,
            ExtractOutcome::Duplicate,
            ExtractOutcome::OutsideRetention,
        ] {
            counts.record(outcome);
        }
        assert_eq!(
            counts,
            OutcomeCounts {
                success: 2,
                duplicates: 1,
                outside_retention: 1,
            }
        );
    }

    #[test]
    fn a_batch_fails_only_when_no_message_was_handled() {
        let result = |success, duplicates, outside_retention, failed| BatchEmlResult {
            total: success + duplicates + outside_retention + failed,
            success,
            duplicates,
            outside_retention,
            failed,
            failed_details: vec![],
        };
        assert_eq!(result(0, 0, 0, 0).status(), ImportStatus::Completed);
        assert_eq!(result(2, 0, 0, 0).status(), ImportStatus::Completed);
        assert_eq!(result(0, 0, 0, 2).status(), ImportStatus::Failed);
        assert_eq!(result(1, 0, 0, 1).status(), ImportStatus::Completed);
        assert_eq!(result(0, 1, 0, 1).status(), ImportStatus::Completed);
        assert_eq!(result(0, 0, 1, 1).status(), ImportStatus::Completed);
    }

    mod retention {
        use super::*;
        use crate::{
            database::{manager::DB_MANAGER, upsert_impl},
            store::tantivy::envelope::test_support::{init_globals, unique_id},
        };

        /// An enabled no-sync account that keeps 30 days of mail.
        pub(super) fn retained_account() -> u64 {
            let id = unique_id();
            upsert_impl(
                DB_MANAGER.db(),
                AccountModel {
                    id,
                    email: "import-retention@example.com".into(),
                    enabled: true,
                    account_type: AccountType::NoSync,
                    retention_days: Some(30),
                    ..Default::default()
                },
            )
            .unwrap();
            id
        }

        /// A message with a unique Message-ID dated `date` (RFC 2822).
        fn eml(date: &str) -> String {
            format!(
                "From: sender@example.com\r\nTo: rcpt@example.com\r\nSubject: retention\r\n\
                 Message-ID: <{}@import.test>\r\nDate: {date}\r\n\r\nbody\r\n",
                uuid::Uuid::new_v4()
            )
        }

        fn recent() -> String {
            chrono::Utc::now().to_rfc2822()
        }

        const OLD: &str = "Mon, 1 Jan 2001 10:00:00 +0000";

        #[tokio::test(flavor = "multi_thread")]
        async fn a_batch_import_counts_mail_outside_the_window() {
            init_globals();
            let account_id = retained_account();
            let result = ImportEmls::do_import(BatchEmlRequest {
                account_id,
                mail_folder: "Imported".into(),
                emls: [eml(OLD), eml(&recent())]
                    .iter()
                    .map(|e| crate::base64_encode_url_safe!(e))
                    .collect(),
            })
            .await
            .unwrap();
            assert_eq!(
                (
                    result.total,
                    result.success,
                    result.outside_retention,
                    result.failed
                ),
                (2, 1, 1, 0)
            );
        }

        /// Upload `content` as `name` and return the final progress.
        pub(super) async fn upload(account_id: u64, name: &str, content: String) -> ImportProgress {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(name);
            std::fs::write(&path, content).unwrap();
            let import_id = format!("retention-{}", unique_id());
            let (id, file) = (import_id.clone(), name.to_string());
            tokio::task::spawn_blocking(move || {
                process_uploaded_file(&id, &path, &file, account_id, "Uploaded", 1)
            })
            .await
            .unwrap();
            get_import_progress(&import_id).expect("progress is recorded")
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn an_eml_upload_outside_the_window_is_counted_not_imported() {
            init_globals();
            let progress = upload(retained_account(), "old.eml", eml(OLD)).await;
            assert_eq!(progress.status, ImportStatus::Completed);
            assert_eq!(
                (
                    progress.success,
                    progress.outside_retention,
                    progress.failed
                ),
                (0, 1, 0)
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn an_mbox_upload_counts_mail_outside_the_window() {
            init_globals();
            let mbox = format!(
                "From sender@example.com Mon Jan  1 10:00:00 2001\n{}\n\
                 From sender@example.com Mon Jan  1 10:00:00 2001\n{}\n",
                eml(OLD).replace("\r\n", "\n"),
                eml(&recent()).replace("\r\n", "\n"),
            );
            let progress = upload(retained_account(), "mail.mbox", mbox).await;
            assert_eq!(progress.status, ImportStatus::Completed);
            assert_eq!(
                (progress.total, progress.success, progress.outside_retention),
                (2, 1, 1)
            );
        }
    }

    mod failures {
        use super::retention::{retained_account, upload};
        use super::*;
        use crate::store::tantivy::envelope::test_support::{init_globals, unique_id};

        fn error_of(progress: &ImportProgress) -> &str {
            &progress.failed_details[0].error_message
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn an_upload_to_an_unknown_account_fails() {
            init_globals();
            let progress = upload(unique_id(), "mail.eml", "Subject: x\r\n\r\nx\r\n".into()).await;
            assert_eq!(progress.status, ImportStatus::Failed);
            assert!(
                error_of(&progress).starts_with("Account validation failed"),
                "{progress:?}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn an_upload_in_an_unknown_format_fails() {
            init_globals();
            let progress = upload(retained_account(), "notes.txt", "\0\0\0".into()).await;
            assert_eq!(
                (&progress.status, progress.format.as_str()),
                (&ImportStatus::Failed, "unknown")
            );
            assert!(
                error_of(&progress).contains("Unknown file format"),
                "{progress:?}"
            );
        }

        /// Run `process` on a file that does not exist and return its progress.
        async fn missing_file(process: fn(&str, &Path, u64, u64, u64, &str)) -> ImportProgress {
            let import_id = format!("missing-{}", unique_id());
            let id = import_id.clone();
            tokio::task::spawn_blocking(move || {
                let dir = tempfile::tempdir().unwrap();
                process(&id, &dir.path().join("gone"), unique_id(), 1, 1, "Uploaded");
            })
            .await
            .unwrap();
            get_import_progress(&import_id).expect("progress is recorded")
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn an_unreadable_eml_file_fails_the_import() {
            init_globals();
            let progress = missing_file(process_eml_file).await;
            assert_eq!(
                (&progress.status, progress.format.as_str()),
                (&ImportStatus::Failed, "eml")
            );
            assert!(
                error_of(&progress).starts_with("Failed to read file"),
                "{progress:?}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn an_unreadable_mbox_file_fails_the_import() {
            init_globals();
            let progress = missing_file(process_mbox_file).await;
            assert_eq!(
                (&progress.status, progress.format.as_str()),
                (&ImportStatus::Failed, "mbox")
            );
            assert!(
                error_of(&progress).starts_with("Failed to open MBOX file"),
                "{progress:?}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn an_oversized_message_is_a_failed_item() {
            init_globals();
            let (counts, failed) = tokio::task::spawn_blocking(|| {
                process_single_eml(&vec![b'x'; MAX_SINGLE_EML_BYTES + 1], 3, unique_id(), 1)
            })
            .await
            .unwrap();
            assert_eq!(counts, OutcomeCounts::default());
            assert_eq!(failed.len(), 1);
            assert_eq!(failed[0].index, 3);
            assert!(
                failed[0].error_message.contains("limit 100 MB"),
                "{failed:?}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn an_unparsable_message_is_a_failed_item() {
            init_globals();
            let account_id = retained_account();
            let (counts, failed) =
                tokio::task::spawn_blocking(move || process_single_eml(b"", 4, account_id, 1))
                    .await
                    .unwrap();
            assert_eq!(counts, OutcomeCounts::default());
            assert_eq!(failed.len(), 1);
            assert_eq!(failed[0].index, 4);
        }
    }
}
