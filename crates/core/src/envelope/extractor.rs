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

use async_imap::types::Fetch;
use bytes::Bytes;
use mail_parser::{AddressList, HeaderName, Message, MessageParser};
use tantivy::{schema::Facet, TantivyDocument};
use tracing::error;
use uuid::Uuid;

use crate::{
    account::migration::{AccountModel, ExtractionRules},
    archive::imap::mailbox::MailBox,
    common::AddrVec,
    envelope::{
        meta::parse_mailboxd_metadata,
        utils::{attachment_parts, header_raw, normalize_subject, part_contents},
    },
    error::{code::ErrorCode, MailboxdResult},
    id,
    imap::executor::ImapExecutor,
    message::content::AttachmentInfo,
    raise_error, retention,
    store::{
        blob::{DetachedEmail, BLOB_MANAGER},
        envelope::Envelope,
        tantivy::{
            attachment::ATTACHMENT_MANAGER,
            dedup_cache::DEDUP_CACHE,
            envelope::ENVELOPE_MANAGER,
            model::{AttachmentModel, EnvelopeWithAttachments},
            rebuild::message_body_text,
        },
    },
    utc_now,
    utils::{compute_content_hash, hex_hash, html::extract_text},
};

/// The outcome of extracting an envelope. `Duplicate` means the message was
/// skipped because its content hash was already archived. `OutsideRetention`
/// means it was skipped because it is older than the account's retention
/// window. `Imported` covers every other processed message, including mail
/// dropped by archive rules, which has always counted as a success on the
/// import surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum ExtractOutcome {
    Imported,
    Duplicate,
    OutsideRetention,
}

pub async fn extract_envelope_and_store_it(
    fetch: Fetch,
    account_id: u64,
    mailbox_id: u64,
) -> MailboxdResult<()> {
    let internal_date = fetch
        .internal_date()
        .map(|d| d.timestamp_millis())
        .unwrap_or(0);
    let uid = fetch.uid.unwrap_or(0);
    let body = match fetch.body() {
        Some(b) => b,
        None => {
            tracing::warn!(
                account_id,
                uid = fetch.uid,
                "FETCH response has no body, skipping message"
            );
            return Ok(());
        }
    };
    let size = fetch.size.unwrap_or(body.len() as u32);
    extract_envelope_core(body, uid, size, internal_date, account_id, mailbox_id)
        .await
        .map(|_| ())
}

pub async fn extract_envelope_from_eml(
    body: &[u8],
    account_id: u64,
    mailbox_id: u64,
) -> MailboxdResult<ExtractOutcome> {
    extract_envelope_core(body, 0, body.len() as u32, 0, account_id, mailbox_id).await
}

pub async fn extract_envelope_from_smtp(
    body: &[u8],
    account_id: u64,
    mailbox_id: u64,
) -> MailboxdResult<ExtractOutcome> {
    extract_envelope_core(
        body,
        0,
        body.len() as u32,
        utc_now!(),
        account_id,
        mailbox_id,
    )
    .await
}

async fn extract_envelope_core(
    body: &[u8],
    uid: u32,
    size: u32,
    internal_date: i64,
    account_id: u64,
    mailbox_id: u64,
) -> MailboxdResult<ExtractOutcome> {
    // The content hash of the original raw EML
    let email_content_hash = compute_content_hash(body);
    if DEDUP_CACHE.contains(account_id, mailbox_id, &email_content_hash) {
        tracing::debug!("Duplicate email detected");
        // println!("Duplicate email detected");
        return Ok(ExtractOutcome::Duplicate);
    }
    // A message that cannot be parsed is a per-message failure, not a system
    // failure: streaming callers match on EmailParseError to skip the message
    // (it is NOT archived) and continue with the rest of the batch instead of
    // letting one broken email abort the whole folder (#377).
    let message: Message<'_> = MessageParser::new().parse(body).ok_or_else(|| {
        raise_error!(
            format!("Email parse failed (uid {uid}, {size} bytes): message not archived"),
            ErrorCode::EmailParseError
        )
    })?;

    let account = AccountModel::get(account_id).ok();
    if let Some(ref account) = account {
        if let Some(ref rules) = account.archive_rules {
            let sender = message.from().and_then(|addr| {
                AddrVec::from(addr)
                    .0
                    .into_iter()
                    .next()
                    .and_then(|a| a.address)
            });
            let subject = message
                .subject()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            let is_spam = !rules.spam_headers.is_empty()
                && rules.spam_headers.iter().any(|h| {
                    header_raw(&message, h.clone())
                        .map(|v| matches!(v.trim().to_lowercase().as_str(), "yes" | "true"))
                        .unwrap_or(false)
                });

            if !rules.should_archive(sender.as_deref(), subject.as_deref(), size, is_spam) {
                tracing::debug!(
                    account_id,
                    uid,
                    sender = sender.as_deref().unwrap_or("?"),
                    subject = subject.as_deref().unwrap_or("?"),
                    "Email filtered out by archive rules"
                );
                return Ok(ExtractOutcome::Imported);
            }
        }
    }

    let preview_limit = 100;
    let text = message_body_text(&message);

    let preview = if text.chars().count() > preview_limit {
        text.chars().take(preview_limit).collect::<String>() + "..."
    } else {
        text.clone()
    };

    let body_text = text;

    let message_id = message
        .message_id()
        .map(String::from)
        .unwrap_or_else(generate_message_id);

    let in_reply_to = message
        .in_reply_to()
        .and_then(|ids| ids.last())
        .map(String::from);
    let references = extract_references(&message);
    let thread_id = compute_thread_id(in_reply_to, references, &message_id);

    let mut subject = message.subject().map(String::from).unwrap_or_default();
    if subject.contains('\u{FFFD}') {
        subject = normalize_subject(header_raw(&message, HeaderName::Subject));
    }

    let date = resolve_message_date(&message, internal_date);
    let internal_date = if internal_date == 0 {
        date
    } else {
        internal_date
    };

    // The sweep would delete it again, and a full re-sync would fetch it again.
    if let Some(cutoff) = account.as_ref().and_then(retention::cutoff_ms) {
        if retention::is_expired(date, internal_date, cutoff) {
            tracing::debug!(
                account_id,
                uid,
                "Email outside the retention window, skipped"
            );
            return Ok(ExtractOutcome::OutsideRetention);
        }
    }
    let parse_addrs = |addrs: Option<AddressList<'_>>| {
        addrs
            .map(|addr| {
                AddrVec::from(addr)
                    .0
                    .into_iter()
                    .filter_map(|a| a.address)
                    .collect()
            })
            .unwrap_or_default()
    };

    let bcc = parse_addrs(message.bcc());
    let cc = parse_addrs(message.cc());
    let to = parse_addrs(message.to());

    let from = message
        .from()
        .and_then(|addr| AddrVec::from(addr).0.into_iter().next())
        .and_then(|add| add.address)
        .unwrap_or_else(|| "unknown".to_string());
    let attachment_count = attachment_parts(&message).count();
    let attachments =
        detach_and_store_attachments(body, &message, &email_content_hash, account_id, mailbox_id)
            .await;

    let envelope_id = Uuid::new_v4().to_string();
    let now = utc_now!();

    let mut final_tags = Vec::new();

    if let Some(meta_header) = header_raw(&message, "X-Mailboxd-Metadata") {
        if let Some(bmd) = parse_mailboxd_metadata(meta_header) {
            if let Some(tags) = bmd.tags {
                let validated_tags: Result<Vec<String>, _> = tags
                    .iter()
                    .map(|tag| Facet::from_text(tag).map(|_| tag.clone()).map_err(|e| e))
                    .collect();

                match validated_tags {
                    Ok(valid_list) => {
                        final_tags = valid_list;
                    }
                    Err(e) => {
                        eprintln!("Tag validation failed, ignoring all tags: {:#?}", e);
                    }
                }
            }
        }
    }

    let attachment_docs: Vec<TantivyDocument> = attachments
        .iter()
        .filter(|a| !a.inline || a.content_id.is_none())
        .map(|a| {
            let has_text = a.extracted_text.is_some();
            AttachmentModel {
                id: Uuid::new_v4().to_string(),
                envelope_id: envelope_id.clone(),
                account_id,
                account_email: None,
                mailbox_id,
                mailbox_name: None,
                subject: subject.clone(),
                content_hash: a.content_hash.clone(),
                from: from.clone(),
                date,
                ingest_at: now,
                size: a.size as u64,
                ext: a.get_extension(),
                category: a.get_category().to_string(),
                content_type: a.file_type.clone(),
                shard_id: 0,
                text: a.extracted_text.clone(),
                has_text,
                is_ocr: a.extracted_is_ocr,
                page_count: a.extracted_page_count.map(|n| n as u64),
                is_indexed: has_text,
                is_message: a.is_message,
                name: a.filename.clone(),
                tags: None,
                auto_tags: None,
            }
        })
        .map(|a| a.into_document())
        .collect();

    let envelope = Envelope {
        id: envelope_id,
        message_id,
        account_id,
        mailbox_id,
        uid,
        subject,
        preview,
        from,
        to,
        cc,
        bcc,
        date,
        internal_date,
        ingest_at: now,
        size,
        thread_id,
        attachment_count,
        regular_attachment_count: attachment_docs.len(),
        tags: (!final_tags.is_empty()).then_some(final_tags),
        account_email: None,
        mailbox_name: None,
        content_hash: email_content_hash.clone(),
        account_name: None,
    };
    // 'attachments' contains both regular and inline attachments
    let ea = EnvelopeWithAttachments {
        envelope,
        attachments: Some(attachments),
    };
    let doc = ea.to_document(&body_text, 0)?;
    tracing::debug!(
        "[account {}][mailbox {}] extract: uid={} msg_id={} content_hash={}",
        account_id,
        mailbox_id,
        uid,
        &ea.envelope.message_id,
        &ea.envelope.content_hash,
    );
    ENVELOPE_MANAGER.queue(doc).await;
    DEDUP_CACHE.insert(account_id, mailbox_id, &email_content_hash);
    for doc in attachment_docs {
        ATTACHMENT_MANAGER.queue(doc).await;
    }
    Ok(ExtractOutcome::Imported)
}

pub fn extract_envelope_from_nested_message(
    message: Message<'_>,
    account_id: u64,
) -> MailboxdResult<Envelope> {
    let text = if let Some(text) = message.body_text(0).map(|cow| cow.into_owned()) {
        text
    } else if let Some(html) = message.body_html(0).map(|cow| cow.into_owned()) {
        extract_text(html)
    } else {
        String::new()
    };

    let message_id = message
        .message_id()
        .map(String::from)
        .unwrap_or_else(generate_message_id);

    let in_reply_to = message
        .in_reply_to()
        .and_then(|ids| ids.last())
        .map(String::from);
    let references = extract_references(&message);
    let thread_id = compute_thread_id(in_reply_to, references, &message_id);

    let mut subject = message.subject().map(String::from).unwrap_or_default();
    if subject.contains('\u{FFFD}') {
        subject = normalize_subject(header_raw(&message, HeaderName::Subject));
    }

    let date = resolve_message_date(&message, 0);

    let parse_addrs = |addrs: Option<AddressList<'_>>| {
        addrs
            .map(|addr| {
                AddrVec::from(addr)
                    .0
                    .into_iter()
                    .filter_map(|a| a.address)
                    .collect()
            })
            .unwrap_or_default()
    };

    let bcc = parse_addrs(message.bcc());
    let cc = parse_addrs(message.cc());
    let to = parse_addrs(message.to());

    let from = message
        .from()
        .and_then(|addr| AddrVec::from(addr).0.into_iter().next())
        .and_then(|add| add.address)
        .unwrap_or_else(|| "unknown".to_string());

    let envelope = Envelope {
        id: Default::default(),
        message_id,
        account_id,
        mailbox_id: Default::default(),
        uid: Default::default(),
        subject,
        preview: text,
        from,
        to,
        cc,
        bcc,
        date,
        internal_date: Default::default(),
        ingest_at: Default::default(),
        size: Default::default(),
        thread_id,
        attachment_count: Default::default(),
        regular_attachment_count: Default::default(),
        tags: Default::default(),
        account_email: Default::default(),
        account_name: Default::default(),
        mailbox_name: Default::default(),
        content_hash: Default::default(),
    };

    Ok(envelope)
}

/// Message date in milliseconds. Falls back from the `Date` header to the
/// IMAP internal date (pass 0 when there is none) and then to the newest
/// `Received` header, so a missing or unparseable `Date` (rustmailer/bichon#332)
/// doesn't archive the message as 1970. Returns 0 only without any source.
pub(crate) fn resolve_message_date(message: &Message<'_>, internal_date: i64) -> i64 {
    let to_ms = |d: mail_parser::DateTime| Some(d.to_timestamp() * 1000).filter(|&ms| ms > 0);
    if let Some(ms) = message.date().and_then(to_ms) {
        return ms;
    }
    if internal_date > 0 {
        return internal_date;
    }
    message
        .all_received()
        .filter_map(|r| r.date().and_then(to_ms))
        .max()
        .unwrap_or(0)
}

pub fn compute_thread_id(
    in_reply_to: Option<String>,
    references: Option<Vec<String>>,
    message_id: &str,
) -> String {
    if in_reply_to.is_some() && references.as_ref().map_or(false, |r| !r.is_empty()) {
        return hex_hash(&references.as_ref().unwrap()[0]);
    }
    hex_hash(message_id)
}

pub fn generate_message_id() -> String {
    let ts = utc_now!();
    let pid = std::process::id();
    format!("<{:016x}.{}.{}@{}>", id!(128), ts, pid, "mailboxd")
}

pub fn extract_references(message: &Message<'_>) -> Option<Vec<String>> {
    message
        .references()
        .map(|ids| ids.iter().map(String::from).collect())
}

/// Prefix of the placeholder that replaces an attachment body inside a
/// stripped EML. Full form: `<<MAILBOXD_DETACH_HASH:{64 hex chars}>>`.
///
/// Which hash the 64 hex chars carry depends on when the record was written,
/// so tooling that scans placeholders must consult the attachment metadata
/// (`AttachmentInfo::raw_content_hash`) — the prefix alone cannot tell the
/// domains apart, because both produce 64-character blake3 hex digests:
///
/// - **Current records** (`raw_content_hash` is `Some`): the
///   domain-separated hash of the *original encoded* MIME body
///   (`compute_raw_attachment_hash`). The blob is stored under the same key.
/// - **Legacy records** (pre-#375, `raw_content_hash` is `None`): the plain
///   hash of the *decoded* attachment contents (`compute_content_hash`),
///   which is also the blob key. `AttachmentInfo::storage_hash()` selects
///   the right one per record.
pub const DETACH_PLACEHOLDER_PREFIX: &str = "<<MAILBOXD_DETACH_HASH:";

/// Decoded contents of an attachment selected for text extraction, collected
/// while detaching so the extraction can run in one background batch.
pub struct TextCandidate {
    pub content_hash: String,
    pub file_type: String,
    pub ext: String,
    pub bytes: Vec<u8>,
}

/// Context deciding which detached attachments are queued for text
/// extraction. Supplied by the live ingest path; offline tools pass `None`
/// to [`strip_attachment_bodies`] and skip extraction entirely.
pub struct ExtractionFilter<'a> {
    /// Per-account rules; `None` means no rules are configured and the
    /// default is to extract every supported type.
    pub rules: Option<&'a ExtractionRules>,
    pub mailbox_name: Option<&'a str>,
    pub sender: Option<&'a str>,
}

/// Output of [`strip_attachment_bodies`].
pub struct DetachedBody {
    /// The EML with every valid attachment body replaced by a
    /// `<<MAILBOXD_DETACH_HASH:...>>` placeholder.
    pub stripped_eml: Vec<u8>,
    /// `(storage key, raw encoded MIME body)` pairs to persist in the blob
    /// store. Invalid attachment ranges contribute a shared empty record in
    /// the raw key domain.
    pub blobs: Vec<(String, Bytes)>,
    /// Attachment metadata, in descending raw-offset order.
    pub infos: Vec<AttachmentInfo>,
    /// Attachments selected for text extraction (empty unless an
    /// [`ExtractionFilter`] was supplied).
    pub text_candidates: Vec<TextCandidate>,
}

/// Strips attachment bodies out of a raw message and computes everything the
/// storage layer needs to persist and later reconstruct it: the stripped EML
/// with hash placeholders, the raw blob records, attachment metadata, and
/// text-extraction candidates.
///
/// This is the single canonical implementation shared by the live ingest path
/// ([`detach_and_store_attachments`]) and the offline store migration; the
/// two must never diverge, or a migrated store stops matching what the
/// runtime writes and reads back.
pub fn strip_attachment_bodies(
    original_body: &[u8],
    message: &Message<'_>,
    filter: Option<&ExtractionFilter<'_>>,
) -> DetachedBody {
    let mut stripped_eml = original_body.to_vec();
    let mut infos = Vec::new();
    let mut blobs = Vec::new();
    let mut text_candidates = Vec::new();

    // Collect and sort attachment ranges in reverse to maintain offset
    // integrity while splicing placeholders.
    let mut ranges: Vec<_> = attachment_parts(message)
        .map(|att| (att.offset_body() as usize, att.offset_end() as usize, att))
        .collect();
    ranges.sort_by(|a, b| b.0.cmp(&a.0));

    for (raw_start, raw_end, att) in ranges {
        // mail-parser may report attachment offsets past the body end for
        // malformed messages; clamp the range to avoid a slice panic.
        let body_len = original_body.len();
        let raw_start = raw_start.min(body_len);
        let raw_end = raw_end.min(body_len);
        let range_valid = raw_start < raw_end;

        // content hash is computed from the decoded attachment contents,
        // which is always available regardless of raw offset validity.
        let contents = part_contents(&att);
        let content_hash = compute_content_hash(&contents);

        let raw_content_hash = if range_valid {
            let raw_bytes = &original_body[raw_start..raw_end];
            // Different transfer encodings of one file must not share a raw blob.
            let raw_hash = crate::utils::compute_raw_attachment_hash(raw_bytes);
            blobs.push((raw_hash.clone(), Bytes::copy_from_slice(raw_bytes)));

            // Replace raw attachment content with a hash-based placeholder
            let placeholder = format!("{DETACH_PLACEHOLDER_PREFIX}{raw_hash}>>");
            stripped_eml.splice(raw_start..raw_end, placeholder.as_bytes().iter().cloned());
            Some(raw_hash)
        } else {
            // No replacement is needed, but keep the empty storage record in
            // the raw key domain so it cannot shadow a decoded or email blob.
            let raw_hash = crate::utils::compute_raw_attachment_hash(b"");
            blobs.push((raw_hash.clone(), Bytes::new()));
            Some(raw_hash)
        };

        let inline = att
            .content_disposition()
            .map(|d| d.is_inline())
            .unwrap_or_else(|| att.content_id().is_some());
        let file_type = att
            .content_type()
            .map(|ct| format!("{}/{}", ct.ctype(), ct.subtype().unwrap_or("")))
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let has_cid = att.content_id().is_some();
        let att_name = att.attachment_name().map(|n| n.to_string());
        let ext = att_name
            .as_deref()
            .and_then(|n| {
                std::path::Path::new(n)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_ascii_lowercase())
            })
            .unwrap_or_default();

        // Collect candidates for text extraction (non-inline, known document
        // types allowed by the filter).
        if let Some(filter) = filter {
            let should_extract = filter.rules.is_none_or(|r| {
                r.should_extract(
                    &ext,
                    filter.mailbox_name,
                    att_name.as_deref(),
                    filter.sender,
                )
            });

            if (!inline || !has_cid)
                && should_extract
                && contents.len() <= crate::ext::text_extractor::MAX_EXTRACT_BYTES
                && crate::ext::text_extractor::should_try_extract(&file_type, &ext)
            {
                text_candidates.push(TextCandidate {
                    content_hash: content_hash.clone(),
                    file_type: file_type.clone(),
                    ext: ext.clone(),
                    bytes: contents.to_vec(),
                });
            }
        }

        infos.push(AttachmentInfo {
            filename: att.attachment_name().map(|n| n.to_string()),
            size: contents.len(),
            inline,
            file_type,
            content_id: att.content_id().map(|id| id.to_string()),
            content_hash,
            raw_content_hash,
            is_message: att.is_message(),
            extracted_text: None,
            extracted_page_count: None,
            extracted_is_ocr: false,
        });
    }

    DetachedBody {
        stripped_eml,
        blobs,
        infos,
        text_candidates,
    }
}

pub async fn detach_and_store_attachments(
    original_body: &[u8],
    message: &Message<'_>,
    eml_content_hash: &str,
    account_id: u64,
    mailbox_id: u64,
) -> Vec<AttachmentInfo> {
    let rules = if account_id > 0 {
        AccountModel::get(account_id)
            .ok()
            .and_then(|a| a.extraction_rules)
    } else {
        None
    };

    let mailbox_name = match rules.as_ref().map(|r| !r.folders.is_empty()) {
        Some(true) => MailBox::get(mailbox_id).ok().map(|mb| mb.name),
        _ => None,
    };

    let sender = message
        .from()
        .and_then(|addr| AddrVec::from(addr).0.into_iter().next())
        .and_then(|add| add.address);

    let filter = ExtractionFilter {
        rules: rules.as_ref(),
        mailbox_name: mailbox_name.as_deref(),
        sender: sender.as_deref(),
    };
    let detached = strip_attachment_bodies(original_body, message, Some(&filter));
    let mut attachment_infos = detached.infos;

    // Run text extraction in a single spawn_blocking batch.
    let text_candidates = detached.text_candidates;
    if !text_candidates.is_empty() {
        if let Ok(mut extracted_map) = tokio::task::spawn_blocking(move || {
            let mut map: std::collections::HashMap<String, (String, Option<u32>, bool)> =
                std::collections::HashMap::new();
            for c in text_candidates {
                if let Some(r) =
                    crate::ext::text_extractor::extract_text(&c.file_type, &c.ext, &c.bytes)
                {
                    map.insert(c.content_hash, (r.text, r.page_count, r.is_ocr));
                }
            }
            map
        })
        .await
        {
            for info in &mut attachment_infos {
                if let Some((text, pages, is_ocr)) = extracted_map.remove(&info.content_hash) {
                    info.extracted_text = Some(text);
                    info.extracted_page_count = pages;
                    info.extracted_is_ocr = is_ocr;
                }
            }
        }
    }
    // Step 4: Store the final stripped EML content
    BLOB_MANAGER
        .queue(DetachedEmail {
            email: (
                eml_content_hash.to_string(),
                Bytes::from(detached.stripped_eml),
            ),
            attachments: Some(detached.blobs),
        })
        .await;

    attachment_infos
}

pub fn reattach_eml_content(
    account_id: u64,
    envelope_id: String,
) -> MailboxdResult<(Envelope, Bytes)> {
    let e = ENVELOPE_MANAGER
        .get_envelope_by_id(account_id, &envelope_id)?
        .ok_or_else(|| {
            raise_error!(
                format!(
                    "Envelope not found: account_id={} envelope_id={}",
                    account_id, &envelope_id
                ),
                ErrorCode::ResourceNotFound
            )
        })?;

    let restored_eml = BLOB_MANAGER
        .get_email(&e.envelope.content_hash)?
        .ok_or_else(|| {
            raise_error!(
                format!(
                "Original email content not found: account_id={} envelope_id={} content_hash={}",
                account_id, &envelope_id, &e.envelope.content_hash
            ),
                ErrorCode::ResourceNotFound
            )
        })?;

    if !e.envelope.has_any_attachments() {
        return Ok((e.envelope, restored_eml));
    }

    let actual_count = e.attachments.as_ref().map(|a| a.len()).unwrap_or(0);
    if e.envelope.attachment_count != actual_count {
        return Err(raise_error!(
            format!(
                "Consistency check failed: envelope.attachment_count ({}) does not match attachments.len ({})",
                e.envelope.attachment_count,
                actual_count
            ),
            ErrorCode::InternalError
        ));
    }

    let restored = reattach_parts(restored_eml.to_vec(), e.attachments.unwrap(), |hash| {
        BLOB_MANAGER.get_attachment(hash)
    })?;
    Ok((e.envelope, restored))
}

fn reattach_parts(
    mut restored_eml: Vec<u8>,
    attachments: Vec<AttachmentInfo>,
    load: impl Fn(&str) -> MailboxdResult<Option<Bytes>>,
) -> MailboxdResult<Bytes> {
    let mut tasks = Vec::new();
    for detail in attachments {
        let storage_hash = detail.storage_hash();

        // Primary scan: placeholders written by the record's own format
        // (raw-body hash for new records, decoded hash for legacy records).
        let found = schedule_placeholder(&mut tasks, &restored_eml, storage_hash, None);

        // Mixed-format recovery: a re-ingest refreshes the envelope metadata
        // to raw storage keys, but the blob store's `exists` dedup keeps the
        // previously stored stripped EML, whose placeholders still carry the
        // legacy decoded hash. When the primary scan finds nothing, retry with
        // the legacy placeholder; the old blob is still stored under that key.
        // The raw key is kept as an alternate load target for the case where
        // the legacy blob was garbage-collected after the original envelope
        // was deleted (the re-ingest stored the identical bytes under it).
        if found == 0 && detail.raw_content_hash.is_some() {
            schedule_placeholder(
                &mut tasks,
                &restored_eml,
                &detail.content_hash,
                Some(storage_hash),
            );
        }
    }

    tasks.sort_by(|a, b| b.0.cmp(&a.0));
    tasks.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

    for (start, end, hash, alt) in tasks {
        let original_data = match load(&hash)? {
            Some(data) => Some(data),
            None => match alt.as_deref() {
                Some(alt) => load(alt)?,
                None => None,
            },
        };
        if let Some(original_data) = original_data {
            restored_eml.splice(start..end, original_data.iter().cloned());
        } else {
            error!("[ERROR] Missing attachment blob for hash: {}", hash);
        }
    }

    Ok(Bytes::from(restored_eml))
}

/// Schedules one replacement task per occurrence of the placeholder carrying
/// `hash` and returns the number of occurrences found. `alt` is a secondary
/// storage key tried at load time when the primary blob is absent.
fn schedule_placeholder(
    tasks: &mut Vec<(usize, usize, String, Option<String>)>,
    eml: &[u8],
    hash: &str,
    alt: Option<&str>,
) -> usize {
    let placeholder_str = format!("{DETACH_PLACEHOLDER_PREFIX}{hash}>>");
    let pattern = placeholder_str.as_bytes();
    let pattern_len = pattern.len();

    let mut found = 0;
    let mut search_cursor = 0;
    while let Some(pos) = eml[search_cursor..]
        .windows(pattern_len)
        .position(|window| window == pattern)
    {
        let absolute_start = search_cursor + pos;
        let absolute_end = absolute_start + pattern_len;

        tasks.push((
            absolute_start,
            absolute_end,
            hash.to_string(),
            alt.map(str::to_string),
        ));
        search_cursor = absolute_end;
        found += 1;
    }
    found
}

/// Returns the raw EML for an indexed message, self-healing a missing content
/// blob.
///
/// Behaves like [`reattach_eml_content`], but when the message's content blob
/// is absent from the blob store it fetches that single message on demand from
/// the IMAP server (`UID FETCH <uid> (BODY.PEEK[])`), persists it for future
/// requests, and returns it. If the on-demand fetch itself fails, the original
/// "content not found" error from [`reattach_eml_content`] is surfaced
/// unchanged so the caller still produces its 404.
pub async fn reattach_eml_content_self_healing(
    account_id: u64,
    envelope_id: String,
) -> MailboxdResult<(Envelope, Bytes)> {
    let envelope = ENVELOPE_MANAGER
        .get_envelope_by_id(account_id, &envelope_id)?
        .ok_or_else(|| {
            raise_error!(
                format!(
                    "Envelope not found: account_id={} envelope_id={}",
                    account_id, &envelope_id
                ),
                ErrorCode::ResourceNotFound
            )
        })?
        .envelope;

    // Fast path: the content blob is present, reuse the regular reattach logic.
    if BLOB_MANAGER.get_email(&envelope.content_hash)?.is_some() {
        return reattach_eml_content(account_id, envelope_id);
    }

    // The blob is missing. Try to recover it directly from the IMAP server.
    match recover_message_blob(&envelope).await {
        Ok(raw_body) => {
            tracing::info!(
                account_id,
                envelope_id = %envelope_id,
                uid = envelope.uid,
                "Self-healed missing email content blob via on-demand IMAP fetch"
            );
            Ok((envelope, raw_body))
        }
        Err(e) => {
            tracing::warn!(
                account_id,
                envelope_id = %envelope_id,
                uid = envelope.uid,
                error = %e,
                "On-demand IMAP fetch for missing content blob failed; returning not-found"
            );
            Err(e)
        }
    }
}

/// Fetches one message from IMAP and re-stores its detached blob.
///
/// On success the freshly fetched raw RFC822 body is returned after its
/// detached form (stripped EML + attachments) has been written to the blob
/// store. Fails if the message cannot be fetched, or if no fetched message
/// matches the archived `content_hash` (the server-side message no longer
/// matches what mailboxd archived, so it cannot be treated as a recovery of
/// that blob).
async fn recover_message_blob(envelope: &Envelope) -> MailboxdResult<Bytes> {
    let mailbox =
        MailBox::find_mailbox(envelope.account_id, envelope.mailbox_id)?.ok_or_else(|| {
            raise_error!(
                format!(
                    "Mailbox not found: account_id={} mailbox_id={}",
                    envelope.account_id, envelope.mailbox_id
                ),
                ErrorCode::ResourceNotFound
            )
        })?;

    let mut session = ImapExecutor::create_connection(envelope.account_id).await?;
    let result = fetch_archived_body(&mut session, &mailbox.encoded_name(), envelope).await;
    session.logout().await.ok();
    let raw_body = result?;

    let message = MessageParser::new()
        .parse(raw_body.as_slice())
        .ok_or_else(|| {
            raise_error!(
                "Failed to parse fetched email content".into(),
                ErrorCode::InternalError
            )
        })?;
    // The attachment metadata is still in the index, so no text extraction:
    // only the blobs are written again. `fetch_archived_body` only returns
    // bytes that hash to the archived content hash, so the detached EML is
    // stored under the envelope's own hash.
    let detached = strip_attachment_bodies(&raw_body, &message, None);
    BLOB_MANAGER
        .store(DetachedEmail {
            email: (
                envelope.content_hash.clone(),
                Bytes::from(detached.stripped_eml),
            ),
            attachments: Some(detached.blobs),
        })
        .await?;

    Ok(Bytes::from(raw_body))
}

/// Fetches the archived message from IMAP and returns it only if its content
/// hash equals the archived `content_hash`. Tries the stored UID first, then
/// the UIDs returned by `UID SEARCH HEADER Message-ID` (the UID changed or the
/// message moved within the folder).
async fn fetch_archived_body(
    session: &mut async_imap::Session<Box<dyn crate::imap::session::SessionStream>>,
    encoded_mailbox: &str,
    envelope: &Envelope,
) -> MailboxdResult<Vec<u8>> {
    let fetched =
        ImapExecutor::fetch_single_message_body(session, encoded_mailbox, envelope.uid).await;
    let mut last_err = match fetched {
        Ok(body) if compute_content_hash(&body) == envelope.content_hash => return Ok(body),
        Ok(body) => raise_error!(
            format!(
                "Fetched message does not match archived content: \
                 expected content_hash={} got={}",
                envelope.content_hash,
                compute_content_hash(&body)
            ),
            ErrorCode::ImapUnexpectedResult
        ),
        Err(e) => e,
    };

    // `uid_search` runs against the mailbox the fetch above selected, so the
    // search and the fetches below stay in the same folder.
    let Some(message_id) = searchable_message_id(&envelope.message_id) else {
        return Err(last_err);
    };
    let uids = session
        .uid_search(format!("HEADER Message-ID \"{}\"", message_id))
        .await
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapUnexpectedResult))?;
    for uid in bounded_message_id_candidates(uids, envelope.uid) {
        match ImapExecutor::fetch_single_message_body(session, encoded_mailbox, uid).await {
            Ok(body) if recovered_body_matches(&body, envelope) => return Ok(body),
            Ok(_) => {}
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

/// Suffixes of the ids [`generate_message_id`] gives messages archived
/// without a `Message-ID` header, including those archived by Bichon.
const SYNTHETIC_MESSAGE_ID_SUFFIXES: [&str; 2] = ["@mailboxd>", "@bichon>"];

/// The trimmed Message-ID to search for, or `None` when a `UID SEARCH HEADER
/// Message-ID` cannot match: a synthetic id exists on no server, and an id
/// with quotes, backslashes or line breaks would break out of the IMAP quoted
/// string.
fn searchable_message_id(message_id: &str) -> Option<&str> {
    let message_id = message_id.trim();
    if message_id.is_empty()
        || SYNTHETIC_MESSAGE_ID_SUFFIXES
            .iter()
            .any(|suffix| message_id.ends_with(suffix))
        || message_id.contains(['"', '\\', '\r', '\n'])
    {
        return None;
    }
    Some(message_id)
}

/// Upper bound on the Message-ID search hits that are fetched.
const MAX_MESSAGE_ID_CANDIDATES: usize = 32;

/// The search hits to try, without the stored UID (already tried), lowest
/// first and at most [`MAX_MESSAGE_ID_CANDIDATES`].
fn bounded_message_id_candidates<I>(uids: I, stored_uid: u32) -> Vec<u32>
where
    I: IntoIterator<Item = u32>,
{
    let mut candidates: Vec<u32> = uids.into_iter().filter(|&uid| uid != stored_uid).collect();
    candidates.sort_unstable();
    candidates.truncate(MAX_MESSAGE_ID_CANDIDATES);
    candidates
}

/// Whether a message found by Message-ID is the archived one: same content
/// hash and, when the archive has one, the same Message-ID.
fn recovered_body_matches(body: &[u8], envelope: &Envelope) -> bool {
    if compute_content_hash(body) != envelope.content_hash {
        return false;
    }
    if envelope.message_id.is_empty() {
        return true;
    }
    MessageParser::new()
        .parse(body)
        .and_then(|message| message.message_id().map(String::from))
        .is_some_and(|message_id| message_id == envelope.message_id)
}

/// [`BlobRepairReport::status`] of a record that was already intact.
pub const STATUS_COMPLETE: &str = "complete";
/// [`BlobRepairReport::status`] of a record that is intact after the repair.
pub const STATUS_REPAIRED: &str = "repaired";
/// [`BlobRepairReport::status`] of a record that is still broken after a
/// verified fetch.
pub const STATUS_PARTIAL: &str = "partial";
/// [`BlobRepairReport::status`] of a record that could not be repaired.
pub const STATUS_FAILED: &str = "failed";

/// Outcome of [`repair_envelope_blobs`].
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct BlobRepairReport {
    pub envelope_id: String,
    /// Account that owns the record.
    pub account_id: u64,
    /// Mailbox the record lives in.
    pub mailbox_id: u64,
    /// Subject of the record, for display.
    pub subject: String,
    /// RFC `Message-ID` of the record, for display.
    pub message_id: String,
    /// `complete` (nothing missing), `repaired`, `partial` or `failed`.
    pub status: String,
    /// Whether the email blob was missing before the repair.
    pub email_missing_before: bool,
    /// Whether the email blob is still missing after the repair.
    pub email_missing_after: bool,
    /// Attachment hashes to suspect before the repair. Best-effort naming
    /// only: the verdict is [`Self::status`], so this can be empty while the
    /// record is broken (see [`known_placeholder_hashes`]).
    pub missing_before: Vec<String>,
    /// Attachment hashes to suspect after the repair. Same caveat as
    /// [`Self::missing_before`].
    pub missing_after: Vec<String>,
    pub error: Option<String>,
}

/// A record's blob store state, from [`diagnose_blobs`].
struct BlobDiagnosis {
    /// The email blob is absent from the store.
    email_missing: bool,
    /// The archived body cannot be rebuilt byte for byte. This, not
    /// [`Self::unresolved`], decides whether a repair is attempted.
    broken: bool,
    /// Best-effort naming of the attachment hashes to suspect.
    unresolved: Vec<String>,
}

/// Inspects a record's blob store state.
///
/// An integrity check, not a presence check: splicing every stored blob back
/// into the detached EML must reproduce the exact body the envelope's
/// `content_hash` was taken from. Any gap (a missing attachment blob, a
/// placeholder the metadata cannot account for, a blob under an unexpected
/// key) changes the hash. It reuses [`reattach_parts`] so it cannot drift
/// from the read path.
fn diagnose_blobs(
    envelope: &Envelope,
    attachments: Option<&[AttachmentInfo]>,
) -> MailboxdResult<BlobDiagnosis> {
    let Some(eml) = BLOB_MANAGER.get_email(&envelope.content_hash)? else {
        return Ok(BlobDiagnosis {
            email_missing: true,
            broken: true,
            unresolved: Vec::new(),
        });
    };

    let attachments = attachments.unwrap_or_default();
    let restored = reattach_parts(eml.to_vec(), attachments.to_vec(), |hash| {
        BLOB_MANAGER.get_attachment(hash)
    })?;
    let intact = compute_content_hash(&restored) == envelope.content_hash;
    Ok(BlobDiagnosis {
        email_missing: false,
        broken: !intact,
        unresolved: if intact {
            Vec::new()
        } else {
            known_placeholder_hashes(&eml, attachments)
        },
    })
}

/// Content hashes referenced by the placeholders in a detached EML, each
/// once, in order of appearance.
fn placeholder_hashes(detached_eml: &[u8]) -> Vec<String> {
    let prefix = DETACH_PLACEHOLDER_PREFIX.as_bytes();
    let mut hashes = Vec::new();
    let mut rest = detached_eml;
    while let Some(pos) = rest.windows(prefix.len()).position(|w| w == prefix) {
        rest = &rest[pos + prefix.len()..];
        let Some(end) = rest.windows(2).position(|w| w == b">>") else {
            break;
        };
        if let Ok(h) = std::str::from_utf8(&rest[..end]) {
            if !hashes.iter().any(|x| x == h) {
                hashes.push(h.to_string());
            }
        }
        rest = &rest[end + 2..];
    }
    hashes
}

/// The placeholders in a detached EML that the record's own attachment
/// metadata accounts for. A placeholder carries the storage hash (the raw
/// MIME key for current records, the decoded hash for legacy ones), so both
/// are matched.
///
/// A naming aid, never a verdict: a placeholder the metadata does not list is
/// left out, so a literal placeholder in a message body is never named as a
/// missing attachment.
fn known_placeholder_hashes(detached_eml: &[u8], attachments: &[AttachmentInfo]) -> Vec<String> {
    let indexed = attachments
        .iter()
        .flat_map(|item| [item.storage_hash(), item.content_hash.as_str()])
        .collect::<std::collections::HashSet<_>>();
    placeholder_hashes(detached_eml)
        .into_iter()
        .filter(|hash| indexed.contains(hash.as_str()))
        .collect()
}

/// Re-fetches one archived message from IMAP and restores its missing email
/// or attachment blobs in place: the envelope, its id and its index entry are
/// kept, so no duplicate is created. Nothing is written unless the fetched
/// message hashes to the archived `content_hash`.
///
/// Only missing blobs can be healed. A blob that is present but corrupt stays
/// `partial` however often the repair is retried, because the blob store
/// keeps the first write for a key.
pub async fn repair_envelope_blobs(
    account_id: u64,
    envelope_id: &str,
) -> MailboxdResult<BlobRepairReport> {
    let envelope_with_attachments = ENVELOPE_MANAGER
        .get_envelope_by_id(account_id, envelope_id)?
        .ok_or_else(|| {
            raise_error!(
                format!("Envelope not found: account_id={account_id} envelope_id={envelope_id}"),
                ErrorCode::ResourceNotFound
            )
        })?;
    let attachments = envelope_with_attachments.attachments;
    let envelope = envelope_with_attachments.envelope;

    let mut report = BlobRepairReport {
        envelope_id: envelope_id.to_string(),
        account_id: envelope.account_id,
        mailbox_id: envelope.mailbox_id,
        subject: envelope.subject.clone(),
        message_id: envelope.message_id.clone(),
        ..Default::default()
    };

    // Without readable attachment metadata the record's parts cannot be
    // enumerated, so it can neither be verified nor healed. The read path
    // rejects the same condition.
    let actual_count = attachments.as_ref().map_or(0, Vec::len);
    if envelope.attachment_count != actual_count {
        report.status = STATUS_FAILED.into();
        report.error = Some(format!(
            "Consistency check failed: envelope.attachment_count ({}) does not match attachments.len ({})",
            envelope.attachment_count, actual_count
        ));
        return Ok(report);
    }

    let before = diagnose_blobs(&envelope, attachments.as_deref())?;
    report.email_missing_before = before.email_missing;
    report.email_missing_after = before.email_missing;
    report.missing_before = before.unresolved;
    if !before.broken {
        report.status = STATUS_COMPLETE.into();
        return Ok(report);
    }
    if let Err(e) = recover_message_blob(&envelope).await {
        report.status = STATUS_FAILED.into();
        report.error = Some(e.to_string());
        // Nothing was written, so the state after is the state before.
        report.missing_after = report.missing_before.clone();
        return Ok(report);
    }
    let after = diagnose_blobs(&envelope, attachments.as_deref())?;
    report.email_missing_after = after.email_missing;
    report.missing_after = after.unresolved;
    report.status = if after.broken {
        STATUS_PARTIAL
    } else {
        STATUS_REPAIRED
    }
    .into();
    Ok(report)
}

#[cfg(test)]
#[path = "extractor_repair_tests.rs"]
mod repair_tests;

#[cfg(test)]
mod test {
    use html2text::config;

    mod message_date {
        use super::super::resolve_message_date;
        use mail_parser::MessageParser;

        const RECEIVED: &str = "Received: from relay.example.com by mx.example.com; Tue, 2 Jan 2024 10:00:00 +0000\r\n\
             Received: from origin.example.com by relay.example.com; Mon, 1 Jan 2024 10:00:00 +0000\r\n";
        // Tue, 2 Jan 2024 10:00:00 UTC
        const NEWEST_RECEIVED_MS: i64 = 1_704_189_600_000;

        fn date_of(headers: &str, internal_date: i64) -> i64 {
            let raw = format!("{headers}From: a@example.com\r\nSubject: x\r\n\r\nbody");
            let message = MessageParser::default().parse(raw.as_bytes()).unwrap();
            resolve_message_date(&message, internal_date)
        }

        #[test]
        fn uses_the_date_header() {
            // Wed, 3 Jan 2024 10:00:00 UTC
            let headers = format!("{RECEIVED}Date: Wed, 3 Jan 2024 10:00:00 +0000\r\n");
            assert_eq!(date_of(&headers, 42), 1_704_276_000_000);
        }

        #[test]
        fn unparseable_date_falls_back_to_internal_date() {
            let headers = format!("{RECEIVED}Date: not a date\r\n");
            assert_eq!(date_of(&headers, 1_700_000_000_000), 1_700_000_000_000);
        }

        #[test]
        fn missing_date_and_internal_date_falls_back_to_newest_received() {
            assert_eq!(date_of(RECEIVED, 0), NEWEST_RECEIVED_MS);
        }

        #[test]
        fn corrupted_header_before_date_falls_back_to_received() {
            // rustmailer/bichon#332: a broken header swallowed the Date header.
            let headers = format!(
                "{RECEIVED}X-MS-TNEF-Correlator: =?utf-8?B?broken\r\n Date: Wed, 3 Jan 2024 10:00:00 +0000\r\n"
            );
            assert_eq!(date_of(&headers, 0), NEWEST_RECEIVED_MS);
        }

        #[test]
        fn zero_only_without_any_date_source() {
            assert_eq!(date_of("", 0), 0);
        }
    }

    // Keep the blob writer alive across these storage tests.
    static STORAGE_RUNTIME: std::sync::LazyLock<tokio::runtime::Runtime> =
        std::sync::LazyLock::new(|| tokio::runtime::Runtime::new().unwrap());

    /// A body that cannot be parsed must surface as `EmailParseError` (not a
    /// generic InternalError): the batch streaming path matches on that code
    /// to skip the single message and keep the batch going (#377).
    #[test]
    fn unparseable_body_raises_email_parse_error() {
        STORAGE_RUNTIME.block_on(async {
            // mail-parser 0.11 is extremely lenient; the empty body is the
            // input it still refuses. If a parser upgrade starts parsing it,
            // replace the input; the assertion is the contract.
            let junk: &[u8] = b"";
            assert!(
                mail_parser::MessageParser::new().parse(junk).is_none(),
                "test input must actually be unparseable"
            );
            let err = super::extract_envelope_from_eml(junk, 1, 1)
                .await
                .expect_err("unparseable body must be an error");
            assert_eq!(
                err.code(),
                crate::error::code::ErrorCode::EmailParseError,
                "parse failures must classify as EmailParseError, got {err:?}"
            );
        });
    }

    #[test]
    fn empty_attachment_uses_the_raw_storage_key_space() {
        STORAGE_RUNTIME.block_on(async {
            let raw = b"From: test@example.com\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=empty.bin\r\n\r\n";
            let message = mail_parser::MessageParser::new().parse(raw).unwrap();
            let infos = super::detach_and_store_attachments(
                raw, &message, &crate::utils::compute_content_hash(raw), 0, 0,
            ).await;
            assert_ne!(infos[0].storage_hash(), infos[0].content_hash);
        });
    }

    #[test]
    fn repeated_attachment_is_restored_once_per_placeholder() {
        let payload = b"BEGIN:VCALENDAR\r\nDESCRIPTION:Repeated invitation\r\nEND:VCALENDAR\r\n";
        let hash = crate::utils::compute_content_hash(payload);
        let placeholder = format!("<<MAILBOXD_DETACH_HASH:{hash}>>");
        let stripped = format!("part-one\r\n{placeholder}\r\npart-two\r\n{placeholder}");
        let info = crate::message::content::AttachmentInfo {
            content_hash: hash,
            ..Default::default()
        };
        let restored =
            super::reattach_parts(stripped.into_bytes(), vec![info.clone(), info], |_| {
                Ok(Some(bytes::Bytes::from_static(payload)))
            })
            .expect("reattach repeated invitation");
        let expected = [
            b"part-one\r\n".as_slice(),
            payload,
            b"\r\npart-two\r\n",
            payload,
        ]
        .concat();
        assert_eq!(restored.as_ref(), expected);
    }

    #[test]
    fn distinct_transfer_encodings_keep_their_original_bytes() {
        let plain = b"BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
        let base64 = b"QkVHSU46VkNBTEVOREFSDQpFTkQ6VkNBTEVOREFSDQo=\r\n";
        let decoded_hash = crate::utils::compute_content_hash(plain);
        let plain_hash = crate::utils::compute_raw_attachment_hash(plain);
        let base64_hash = crate::utils::compute_raw_attachment_hash(base64);
        let infos = vec![
            crate::message::content::AttachmentInfo {
                content_hash: decoded_hash.clone(),
                raw_content_hash: Some(plain_hash.clone()),
                ..Default::default()
            },
            crate::message::content::AttachmentInfo {
                content_hash: decoded_hash,
                raw_content_hash: Some(base64_hash.clone()),
                ..Default::default()
            },
        ];
        let stripped = format!(
            "plain\r\n<<MAILBOXD_DETACH_HASH:{plain_hash}>>base64\r\n<<MAILBOXD_DETACH_HASH:{base64_hash}>>"
        );
        let restored = super::reattach_parts(stripped.into_bytes(), infos, |hash| {
            Ok(Some(if hash == plain_hash {
                bytes::Bytes::from_static(plain)
            } else {
                bytes::Bytes::from_static(base64)
            }))
        })
        .expect("reattach independently encoded copies");
        assert_eq!(
            restored.as_ref(),
            [b"plain\r\n".as_slice(), plain, b"base64\r\n", base64].concat()
        );
    }

    #[test]
    fn mixed_format_metadata_falls_back_to_legacy_placeholders() {
        // A stripped EML written by the pre-#375 format (decoded-hash
        // placeholders) combined with metadata refreshed by a re-ingest
        // (raw storage keys) must still reconstruct: the legacy blob under
        // the decoded key is still present in the blob store.
        let decoded_hash = "d".repeat(64);
        let raw_hash = "r".repeat(64);
        let info = crate::message::content::AttachmentInfo {
            content_hash: decoded_hash.clone(),
            raw_content_hash: Some(raw_hash.clone()),
            ..Default::default()
        };
        let stripped = format!("head\r\n<<MAILBOXD_DETACH_HASH:{decoded_hash}>>\r\ntail");
        // Both blobs exist; the legacy one must win for a legacy placeholder.
        let store = std::collections::HashMap::from([
            (
                decoded_hash.clone(),
                bytes::Bytes::from_static(b"legacy body"),
            ),
            (raw_hash.clone(), bytes::Bytes::from_static(b"raw body")),
        ]);
        let restored = super::reattach_parts(stripped.into_bytes(), vec![info], |hash| {
            Ok(store.get(hash).cloned())
        })
        .expect("reattach mixed-format record");
        assert_eq!(restored.as_ref(), b"head\r\nlegacy body\r\ntail".as_slice());
    }

    #[test]
    fn mixed_format_fallback_also_loads_the_raw_blob() {
        // Same mixed state, but the legacy blob under the decoded key was
        // garbage-collected after the original envelope was deleted. The
        // re-ingest stored the identical bytes under the raw key, which must
        // be used as the alternate load target.
        let decoded_hash = "d".repeat(64);
        let raw_hash = "r".repeat(64);
        let info = crate::message::content::AttachmentInfo {
            content_hash: decoded_hash.clone(),
            raw_content_hash: Some(raw_hash.clone()),
            ..Default::default()
        };
        let stripped = format!("<<MAILBOXD_DETACH_HASH:{decoded_hash}>>");
        let restored = super::reattach_parts(stripped.into_bytes(), vec![info], |hash| {
            if hash == raw_hash {
                Ok(Some(bytes::Bytes::from_static(b"raw body")))
            } else {
                Ok(None)
            }
        })
        .expect("reattach via raw-key alternate");
        assert_eq!(restored.as_ref(), b"raw body".as_slice());
    }

    #[test]
    fn legacy_record_with_missing_blob_keeps_its_placeholder() {
        // A legacy record has no raw key, so there is no alternate load target:
        // a missing blob leaves the placeholder in place instead of failing.
        let decoded_hash = "d".repeat(64);
        let info = crate::message::content::AttachmentInfo {
            content_hash: decoded_hash.clone(),
            raw_content_hash: None,
            ..Default::default()
        };
        let stripped = format!("head\r\n<<MAILBOXD_DETACH_HASH:{decoded_hash}>>\r\ntail");
        let restored =
            super::reattach_parts(stripped.clone().into_bytes(), vec![info], |_| Ok(None))
                .expect("a missing blob is not an error");
        assert_eq!(restored.as_ref(), stripped.as_bytes());
    }

    /// A valid message with a unique Message-ID, so the on-disk dedup state of
    /// earlier test runs never marks it as already archived.
    fn unique_eml(extra_headers: &str) -> Vec<u8> {
        format!(
            "From: sender@example.com\r\nTo: rcpt@example.com\r\nSubject: unique\r\n\
             Message-ID: <{}@extractor.test>\r\n{extra_headers}\r\nbody text\r\n",
            uuid::Uuid::new_v4()
        )
        .into_bytes()
    }

    #[test]
    fn valid_body_is_imported_once_then_reported_as_duplicate() {
        STORAGE_RUNTIME.block_on(async {
            let eml = unique_eml("Date: Wed, 3 Jan 2024 10:00:00 +0000\r\n");
            let first = super::extract_envelope_from_eml(&eml, 59_342_201, 201)
                .await
                .expect("valid body is archived");
            assert_eq!(first, super::ExtractOutcome::Imported);
            let second = super::extract_envelope_from_eml(&eml, 59_342_201, 201)
                .await
                .expect("re-import is not an error");
            assert_eq!(second, super::ExtractOutcome::Duplicate);
        });
    }

    #[test]
    fn mail_older_than_the_retention_window_is_not_archived() {
        let account_id = crate::id!(64);
        crate::database::upsert_impl(
            crate::database::manager::DB_MANAGER.db(),
            crate::account::migration::AccountModel {
                id: account_id,
                email: "extract-retention@example.com".into(),
                retention_days: Some(30),
                ..Default::default()
            },
        )
        .unwrap();
        STORAGE_RUNTIME.block_on(async {
            let old = unique_eml("Date: Mon, 1 Jan 2001 10:00:00 +0000\r\n");
            let outcome = super::extract_envelope_from_eml(&old, account_id, 202)
                .await
                .expect("skipping is not an error");
            assert_eq!(outcome, super::ExtractOutcome::OutsideRetention);
            // Skipped mail is not remembered as archived, so it is skipped
            // again rather than reported as a duplicate.
            let again = super::extract_envelope_from_eml(&old, account_id, 202)
                .await
                .unwrap();
            assert_eq!(again, super::ExtractOutcome::OutsideRetention);

            let now = chrono::Utc::now().to_rfc2822();
            let recent = unique_eml(&format!("Date: {now}\r\n"));
            let outcome = super::extract_envelope_from_eml(&recent, account_id, 202)
                .await
                .unwrap();
            assert_eq!(outcome, super::ExtractOutcome::Imported);
        });
    }

    #[test]
    fn nested_message_date_falls_back_to_newest_received_header() {
        let with_date = unique_eml("Date: Wed, 3 Jan 2024 10:00:00 +0000\r\n");
        let message = mail_parser::MessageParser::new().parse(&with_date).unwrap();
        let envelope = super::extract_envelope_from_nested_message(message, 7).unwrap();
        // Wed, 3 Jan 2024 10:00:00 UTC
        assert_eq!(envelope.date, 1_704_276_000_000);
        assert_eq!(envelope.account_id, 7);

        let without_date = unique_eml(
            "Received: from a by b; Tue, 2 Jan 2024 10:00:00 +0000\r\n\
             Received: from c by a; Mon, 1 Jan 2024 10:00:00 +0000\r\n",
        );
        let message = mail_parser::MessageParser::new()
            .parse(&without_date)
            .unwrap();
        let envelope = super::extract_envelope_from_nested_message(message, 7).unwrap();
        // Tue, 2 Jan 2024 10:00:00 UTC
        assert_eq!(envelope.date, 1_704_189_600_000);
    }

    /// Two text attachments, `notes.txt` and `report.txt`, plus a body part.
    fn two_attachment_eml(content_type: &str, ext: &str) -> Vec<u8> {
        format!(
            "From: sender@example.com\r\nSubject: att\r\nMIME-Version: 1.0\r\n\
             Content-Type: multipart/mixed; boundary=\"bnd\"\r\n\r\n\
             --bnd\r\nContent-Type: text/plain\r\n\r\nHello\r\n\
             --bnd\r\nContent-Type: {content_type}\r\n\
             Content-Disposition: attachment; filename=\"notes.{ext}\"\r\n\r\nnotes text\r\n\
             --bnd\r\nContent-Type: {content_type}\r\n\
             Content-Disposition: attachment; filename=\"report.{ext}\"\r\n\r\nreport text\r\n\
             --bnd--\r\n"
        )
        .into_bytes()
    }

    #[test]
    fn extraction_rules_select_the_text_candidates() {
        let raw = two_attachment_eml("text/plain", "txt");
        let message = mail_parser::MessageParser::new().parse(&raw).unwrap();
        let rules = crate::account::migration::ExtractionRules {
            enabled: true,
            attachment_names: crate::account::migration::FilterRule {
                include: vec!["^notes".into()],
                exclude: vec![],
            },
            ..Default::default()
        };
        let filter = super::ExtractionFilter {
            rules: Some(&rules),
            mailbox_name: Some("INBOX"),
            sender: Some("sender@example.com"),
        };
        let detached = super::strip_attachment_bodies(&raw, &message, Some(&filter));
        assert_eq!(detached.infos.len(), 2);
        let candidates: Vec<&[u8]> = detached
            .text_candidates
            .iter()
            .map(|c| c.bytes.as_slice())
            .collect();
        assert_eq!(candidates, vec![b"notes text".as_slice()]);

        let disabled = crate::account::migration::ExtractionRules::default();
        let filter = super::ExtractionFilter {
            rules: Some(&disabled),
            mailbox_name: None,
            sender: None,
        };
        let detached = super::strip_attachment_bodies(&raw, &message, Some(&filter));
        assert!(detached.text_candidates.is_empty());
    }

    /// Extracts text only from a content type no real mail uses, and defers
    /// to "no text" for everything else, like the default no-op extractor.
    struct ProbeExtractor;
    impl crate::ext::text_extractor::AttachmentTextExtractor for ProbeExtractor {
        fn extract(
            &self,
            content_type: &str,
            _ext: &str,
            bytes: &[u8],
        ) -> Option<crate::ext::text_extractor::ExtractedText> {
            (content_type == "text/x-mailboxd-probe").then(|| {
                crate::ext::text_extractor::ExtractedText {
                    text: String::from_utf8_lossy(bytes).into_owned(),
                    page_count: None,
                    is_ocr: false,
                }
            })
        }
    }

    #[test]
    fn folder_rules_limit_text_extraction_to_matching_mailboxes() {
        STORAGE_RUNTIME.block_on(async {
            use crate::database::{manager::DB_MANAGER, upsert_impl};
            crate::ext::text_extractor::set_extractor(Box::new(ProbeExtractor));
            let account_id = 59_342_202;
            let account = super::AccountModel {
                id: account_id,
                extraction_rules: Some(crate::account::migration::ExtractionRules {
                    enabled: true,
                    folders: crate::account::migration::FilterRule {
                        include: vec!["^Invoices$".into()],
                        exclude: vec![],
                    },
                    ..Default::default()
                }),
                ..Default::default()
            };
            upsert_impl(DB_MANAGER.db(), account).unwrap();
            let mailbox = |id: u64, name: &str| super::MailBox {
                id,
                account_id,
                name: name.into(),
                ..Default::default()
            };
            super::MailBox::batch_upsert(&[mailbox(202_001, "Invoices"), mailbox(202_002, "Spam")])
                .unwrap();

            let raw = two_attachment_eml("text/x-mailboxd-probe", "txt");
            let message = mail_parser::MessageParser::new().parse(&raw).unwrap();
            let hash = crate::utils::compute_content_hash(&raw);
            let texts = |infos: Vec<super::AttachmentInfo>| {
                let mut texts: Vec<Option<String>> =
                    infos.into_iter().map(|i| i.extracted_text).collect();
                texts.sort();
                texts
            };

            let infos =
                super::detach_and_store_attachments(&raw, &message, &hash, account_id, 202_001)
                    .await;
            assert_eq!(
                texts(infos),
                vec![Some("notes text".into()), Some("report text".into())]
            );

            let infos =
                super::detach_and_store_attachments(&raw, &message, &hash, account_id, 202_002)
                    .await;
            assert_eq!(texts(infos), vec![None, None]);
        });
    }

    #[test]
    fn raw_blob_key_cannot_alias_legacy_attachment_or_email_key() {
        let payload = b"same bytes in a legacy decoded attachment or original email";
        assert_ne!(
            crate::utils::compute_raw_attachment_hash(payload),
            crate::utils::compute_content_hash(payload)
        );
    }

    #[test]
    fn legacy_metadata_uses_the_original_decoded_storage_key() {
        let info: crate::message::content::AttachmentInfo = serde_json::from_str(
            r#"{"file_type":"application/pdf","inline":false,"filename":"a.pdf","size":4,"content_id":null,"content_hash":"legacy-hash","is_message":false,"extracted_text":null,"extracted_page_count":null}"#
        ).expect("read legacy attachment metadata");
        assert_eq!(info.storage_hash(), "legacy-hash");
    }

    #[test]
    fn test_various_html_with_overflow_enabled() {
        let cases = [
            ("<p>Hello World</p>", "Simple paragraph"),
            ("<h1>Title</h1><p>Content</p>", "Heading + paragraph"),
            ("<ul><li>Item1</li><li>Item2</li></ul>", "Unordered list"),
            (
                "<strong>Bold</strong> and <em>italic</em>",
                "Inline formatting",
            ),
            (
                "<div><span>Nested</span> elements</div>",
                "Nested inline elements inside block",
            ),
            (
                "<table><tr><td>A</td><td>B</td></tr></table>",
                "Simple table",
            ),
            (
                "<pre>  preformatted text\n  line2</pre>",
                "Preformatted block",
            ),
            ("😃 emoji test", "Wide emoji"),
            ("<a href=\"#\">link</a>", "Anchor tag"),
            (
                "<blockquote><p>Quoted text</p></blockquote>",
                "Blockquote with paragraph",
            ),
        ];

        for (html, desc) in cases {
            let result = config::plain()
                .allow_width_overflow()
                .string_from_read(html.as_bytes(), 100);

            match result {
                Ok(output) => {
                    println!("✓ Rendered ({}) =>\n{}", desc, output);
                }
                Err(e) => panic!("Unexpected error for {}: {:?}", desc, e),
            }
        }
    }

    /// Verifies that [`super::detach_and_store_attachments`] does not panic
    /// when mail-parser reports attachment offsets past the raw body length.
    ///
    /// Regression test for: "range end index X out of range for slice of
    /// length Y" panic caused by a malformed email whose attachment
    /// `raw_end_offset` exceeded the actual body size.
    #[test]
    fn detach_attachments_bounds_check() {
        STORAGE_RUNTIME.block_on(check_attachment_bounds());
    }

    async fn check_attachment_bounds() {
        let raw = concat!(
            "From: sender@example.com\r\n",
            "To: recipient@example.com\r\n",
            "Subject: Test\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: multipart/mixed; boundary=\"bnd\"\r\n",
            "\r\n",
            "--bnd\r\n",
            "Content-Type: text/plain\r\n",
            "\r\n",
            "Hello\r\n",
            "--bnd\r\n",
            "Content-Type: application/octet-stream\r\n",
            "Content-Disposition: attachment; filename=\"test.bin\"\r\n",
            "\r\n",
            "AAAAABBBBBCCCCCDDDDDEEEEEAAAAABBBBBCCCCCDDDDDEEEEE\r\n",
            "--bnd--\r\n",
        )
        .as_bytes()
        .to_vec();

        let message = mail_parser::MessageParser::new()
            .parse(&raw)
            .expect("parse valid MIME message");
        assert_eq!(super::attachment_parts(&message).count(), 1);

        // Truncate the raw body so the attachment's raw_end_offset lies
        // past the body end — exactly the scenario reported by users.
        let truncated = &raw[..raw.len() - 20];
        assert!(truncated.len() < raw.len());

        // Must not panic.
        let infos =
            super::detach_and_store_attachments(truncated, &message, "test_content_hash", 0, 0)
                .await;

        // The attachment count must still match so the consistency check
        // in reattach_eml_content doesn't fail later.
        assert_eq!(infos.len(), 1);
    }
}
