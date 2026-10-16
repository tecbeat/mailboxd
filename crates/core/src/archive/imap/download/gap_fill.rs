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

use std::collections::{HashMap, HashSet};

use tracing::{info, warn};

use crate::error::code::ErrorCode;
use crate::raise_error;
use crate::{
    account::{
        migration::AccountModel,
        state::{DownloadState, GapFillFolderStats, GapFillState},
    },
    archive::imap::mailbox::MailBox,
    error::MailboxdResult,
    imap::executor::{compress_uid_list, ImapExecutor, DEFAULT_BATCH_SIZE},
    store::tantivy::envelope::EnvelopeSnapshot,
};

/// Number of times a batch download is retried with a fresh connection before
/// it is counted as failed (mirrors the incremental sync path).
const MAX_NETWORK_RETRIES: u32 = 3;

/// Lightweight header metadata for one remote message, fetched via
/// `FETCH (UID RFC822.SIZE INTERNALDATE BODY.PEEK[HEADER.FIELDS (MESSAGE-ID)])`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteHeader {
    pub uid: u32,
    pub message_id: Option<String>,
    pub size: u64,
    /// Epoch millis (internal date).
    pub internal_date: i64,
}

/// Which remote uids are missing locally. A remote message is "present" if its
/// message-id exists locally. The (size, internal_date) fingerprint is a
/// fallback for every remote message — not only those without a message-id —
/// because some paths store a different message-id locally than the remote
/// header carries (e.g. the SMTP path generates a random one), and servers
/// like Zoho/163 reuse the same message-id across different messages. A
/// duplicated local message-id still counts as present: re-downloading would
/// be deduplicated away anyway, so it can never repair the duplication.
pub fn compute_missing_uids(remote: &[RemoteHeader], local: &[EnvelopeSnapshot]) -> Vec<u32> {
    let mut local_by_msg_id: HashMap<&str, usize> = HashMap::new();
    let mut local_by_fingerprint: HashSet<(u64, i64)> = HashSet::new();
    for snap in local {
        if !snap.message_id.is_empty() {
            *local_by_msg_id.entry(snap.message_id.as_str()).or_insert(0) += 1;
        }
        local_by_fingerprint.insert((snap.size, snap.internal_date));
    }

    let mut missing = Vec::new();
    for header in remote {
        let present = match &header.message_id {
            Some(msg_id) => local_by_msg_id.get(msg_id.as_str()).is_some_and(|&c| c > 0),
            None => false,
        };
        if !present {
            // Fingerprint fallback for every remote message, not just those
            // without a message-id: the remote message-id may not exist
            // locally even though the message is already stored (random
            // synthetic ids on the SMTP path).
            let fp_present = local_by_fingerprint.contains(&(header.size, header.internal_date));
            if !fp_present {
                missing.push(header.uid);
            }
        }
    }
    missing
}

/// Runs the gap-fill phase for one mailbox: enumerates every remote UID,
/// diffs against local envelopes, downloads the missing ones and returns the
/// per-folder outcome. Handles per-batch network errors by counting them as
/// failed (retryable on a later gap-fill run) instead of aborting the phase.
pub async fn gap_fill_mailbox(
    account: &AccountModel,
    local_mailbox: &MailBox,
    remote_mailbox: &MailBox,
    token: tokio_util::sync::CancellationToken,
) -> MailboxdResult<GapFillFolderStats> {
    let account_id = account.id;
    let mut stats = GapFillFolderStats::default();

    let mut session = ImapExecutor::create_connection(account_id).await?;
    session
        .examine(&remote_mailbox.encoded_name())
        .await
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

    // Phase 1: enumerate every remote UID. A huge mailbox can make the server
    // take a while to answer `UID SEARCH ALL`; keep the UI informed instead of
    // appearing stuck (the socket read timeout is the final backstop).
    let search_started = std::time::Instant::now();
    // Build the search future ONCE and keep polling it: the stall-report
    // loop used to construct a fresh `uid_search("ALL")` future on every
    // 5 s timeout, dropping the in-flight command (desyncing the session
    // and restarting the server's work each time), so any mailbox whose
    // search takes longer than one stall interval could never finish (#373).
    // The block scope drops the pinned future before the logout on cancel —
    // it holds the `&mut session` borrow.
    let search_outcome = {
        let search_fut = session.uid_search("ALL");
        tokio::pin!(search_fut);
        loop {
            match tokio::time::timeout(std::time::Duration::from_secs(5), &mut search_fut).await {
                Ok(res) => break Some(res),
                Err(_) => {
                    let stall = search_started.elapsed().as_secs_f64();
                    if token.is_cancelled() {
                        break None;
                    }
                    let _ = GapFillState::update_folder_progress(
                        account_id,
                        remote_mailbox.name.clone(),
                        0,
                        0,
                        crate::imap::executor::slow_server_message(None, Some(stall)),
                    );
                    tracing::warn!(
                        account_id,
                        mailbox = %remote_mailbox.name,
                        stall_secs = format!("{:.0}", stall),
                        "gap-fill: UID SEARCH ALL taking long, still waiting"
                    );
                }
            }
        }
    };
    let results = match search_outcome {
        Some(res) => {
            res.map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
        }
        None => {
            session.logout().await.ok();
            return Ok(stats);
        }
    };
    let mut remote_uids: Vec<u32> = results.into_iter().collect();
    remote_uids.sort();
    if remote_uids.is_empty() {
        session.logout().await.ok();
        return Ok(stats);
    }

    // Phase 2: fetch header metadata for all remote uids in batches.
    // A failed batch counts its uids as failed (they cannot be diffed) but
    // does not abort the phase. A cancellation, however, leaves the header
    // list incomplete so the diff would be unreliable — return immediately.
    let mut remote_headers: Vec<RemoteHeader> = Vec::with_capacity(remote_uids.len());
    let batch_size = account.download_batch_size.unwrap_or(DEFAULT_BATCH_SIZE) as usize;
    let mut cancelled = false;
    // Progress reported so far across all header batches, so the UI can show
    // enumeration progress (and slow-server hints) while a huge mailbox is
    // being scanned.
    let headers_fetched = std::sync::Mutex::new(0u64);
    // Header chunks share one progress callback (it only reads outer state).
    let header_progress = |count: u64, stall_secs: Option<f64>| {
        *headers_fetched.lock().unwrap() = count;
        GapFillState::update_folder_progress(
            account_id,
            remote_mailbox.name.clone(),
            remote_uids.len() as u64,
            count,
            crate::imap::executor::slow_server_message(None, stall_secs),
        )
    };
    for chunk in remote_uids.chunks(batch_size) {
        if token.is_cancelled() {
            cancelled = true;
            break;
        }
        let seq_set = compress_uid_list(chunk.to_vec());
        // A network error mid-phase used to leave the (now dead) session in
        // place: every later chunk silently returned no rows, so most of the
        // folder was never compared while the run reported a tiny (or zero)
        // candidate count. Reconnect and retry the chunk, the same way the
        // download phase does, before giving up on it.
        let mut header_retries = 0u32;
        let header_result = loop {
            match ImapExecutor::fetch_uid_headers(
                &mut session,
                &seq_set,
                token.clone(),
                Some(&header_progress),
            )
            .await
            {
                Ok(headers) => break Ok(headers),
                Err(e)
                    if header_retries < MAX_NETWORK_RETRIES
                        && e.code() == ErrorCode::NetworkError =>
                {
                    header_retries += 1;
                    warn!(
                        account_id,
                        mailbox = remote_mailbox.name,
                        retries = header_retries,
                        "Gap-fill: network error on header batch, reconnecting ({}/{})",
                        header_retries,
                        MAX_NETWORK_RETRIES
                    );
                    match ImapExecutor::create_connection(account_id).await {
                        Ok(new_session) => {
                            session = new_session;
                            if let Err(e2) = session.examine(&remote_mailbox.encoded_name()).await {
                                let err_msg = format!(
                                    "Gap-fill: re-examine failed after reconnect: {:#?}",
                                    e2
                                );
                                let _ = DownloadState::append_session_error(account_id, err_msg);
                                break Err(e);
                            }
                            let backoff = [5u64, 15, 30][(header_retries - 1) as usize];
                            warn!(
                                account_id,
                                mailbox = remote_mailbox.name,
                                "Gap-fill: backing off {}s before retrying header batch",
                                backoff
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                            continue;
                        }
                        Err(e2) => {
                            tracing::error!(account_id, "Gap-fill: reconnection failed: {:#?}", e2);
                            break Err(e);
                        }
                    }
                }
                Err(e) => break Err(e),
            }
        };
        match header_result {
            Ok(headers) => {
                *headers_fetched.lock().unwrap() += headers.len() as u64;
                remote_headers.extend(headers);
            }
            Err(e) => {
                // Count the whole chunk as failed; the user can re-run
                // gap-fill to retry. Do not abort the phase.
                stats.failed += chunk.len() as u64;
                let err_msg = format!("Gap-fill header batch failed: {:#?}", e);
                warn!(account_id, mailbox = remote_mailbox.name, "{}", err_msg);
                let _ = DownloadState::append_session_error(account_id, err_msg);
            }
        }
    }
    if cancelled {
        session.logout().await.ok();
        GapFillState::update_folder_progress(account_id, remote_mailbox.name.clone(), 0, 0, None)?;
        return Ok(stats);
    }
    remote_headers.sort_by_key(|h| h.uid);
    session.logout().await.ok();

    // Phase 3: local snapshot
    let local_snapshots = crate::store::tantivy::envelope::ENVELOPE_MANAGER
        .get_envelope_snapshots_for_mailbox(account_id, local_mailbox.id)?;

    // Phase 4: diff
    let missing_uids = compute_missing_uids(&remote_headers, &local_snapshots);
    stats.candidate_count = missing_uids.len() as u64;
    if missing_uids.is_empty() {
        info!(
            account_id,
            mailbox = remote_mailbox.name,
            "Gap-fill: no missing emails"
        );
        GapFillState::update_folder_progress(account_id, remote_mailbox.name.clone(), 0, 0, None)?;
        return Ok(stats);
    }

    // Phase 5: download missing in batches
    let planned = missing_uids.len() as u64;
    GapFillState::update_folder_progress(
        account_id,
        remote_mailbox.name.clone(),
        planned,
        0,
        None,
    )?;

    let mut session2 = ImapExecutor::create_connection(account_id).await?;
    session2
        .examine(&remote_mailbox.encoded_name())
        .await
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

    let batches =
        crate::imap::executor::generate_uid_sequence_hashset(missing_uids.clone(), batch_size);
    let mut downloaded = 0u64;
    let mut failed = 0u64;
    // Messages skipped because they could not be parsed (not archived): they
    // were candidates that did not land, so they count as failed — that keeps
    // the watermark from advancing and guarantees the next gap-fill run
    // re-detects them (#377).
    let mut parse_skipped = 0u64;
    let mut cancelled = false;
    for (index, batch) in batches.into_iter().enumerate() {
        if token.is_cancelled() {
            cancelled = true;
            break;
        }
        // A slow server can stall a batch past the socket read timeout, same
        // as in the incremental path. Retry such batches on a fresh connection
        // instead of counting them as failed outright.
        let mut retries = 0u32;
        // Tracks the last cumulative count the progress callback reported. When
        // a batch fails mid-stream the executor reports the already-stored
        // count one final time before returning the error, so this is the
        // number of emails of this batch that actually made it to disk.
        let last_reported = std::sync::Mutex::new(0u64);
        let batch_result = loop {
            match ImapExecutor::uid_batch_retrieve_emails(
                &mut session2,
                account_id,
                remote_mailbox.id,
                &batch.0,
                account.max_email_size_bytes,
                token.clone(),
                Some(&|cumulative, avg_secs, stall_secs| {
                    *last_reported.lock().unwrap() = cumulative;
                    let _ = GapFillState::update_folder_progress(
                        account_id,
                        remote_mailbox.name.clone(),
                        planned,
                        downloaded + cumulative,
                        crate::imap::executor::slow_server_message(avg_secs, stall_secs),
                    );
                    Ok(())
                }),
            )
            .await
            {
                Ok(outcome) => break Ok(outcome),
                Err(e) if retries < MAX_NETWORK_RETRIES && e.code() == ErrorCode::NetworkError => {
                    retries += 1;
                    warn!(
                        account_id,
                        mailbox = remote_mailbox.name,
                        index,
                        retries,
                        "Gap-fill: network error on batch, reconnecting ({}/{})",
                        retries,
                        MAX_NETWORK_RETRIES
                    );
                    match ImapExecutor::create_connection(account_id).await {
                        Ok(new_session) => {
                            session2 = new_session;
                            if let Err(e2) = session2.examine(&remote_mailbox.encoded_name()).await
                            {
                                let err_msg = format!(
                                    "Gap-fill: re-examine failed after reconnect: {:#?}",
                                    e2
                                );
                                DownloadState::append_session_error(account_id, err_msg)?;
                                break Err(e);
                            }
                            // Longer backoff than the original 1s/2s/4s: a
                            // throttling server needs time to recover.
                            let backoff = [5u64, 15, 30][(retries - 1) as usize];
                            warn!(
                                account_id,
                                mailbox = remote_mailbox.name,
                                "Gap-fill: backing off {}s before retrying batch",
                                backoff
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                            continue;
                        }
                        Err(e2) => {
                            tracing::error!(account_id, "Gap-fill: reconnection failed: {:#?}", e2);
                            break Err(e);
                        }
                    }
                }
                Err(e) => break Err(e),
            }
        };
        match batch_result {
            Ok(outcome) => {
                downloaded += outcome.stored;
                parse_skipped += outcome.parse_skipped;
                GapFillState::update_folder_progress(
                    account_id,
                    remote_mailbox.name.clone(),
                    planned,
                    downloaded,
                    None,
                )?;
            }
            Err(e) => {
                // The batch may have partially succeeded: emails already stored
                // before the failure are counted as downloaded, only the rest
                // of the batch is failed. The user can re-run gap-fill to
                // retry the remainder; dedup makes re-downloading the stored
                // ones harmless. Do not abort the phase.
                let processed = *last_reported.lock().unwrap();
                downloaded += processed;
                let remaining = batch.1.saturating_sub(processed);
                failed += remaining;
                let err_msg = format!(
                    "Gap-fill batch {} failed after {} processed: {:#?}",
                    index, processed, e
                );
                warn!(account_id, mailbox = remote_mailbox.name, "{}", err_msg);
                let _ = DownloadState::append_session_error(account_id, err_msg);
            }
        }
    }
    session2.logout().await.ok();

    stats.downloaded = downloaded;
    // Accumulate rather than overwrite: phase 2 (header batch) failures already
    // counted into stats.failed and must survive alongside phase 5 failures.
    // Parse-skipped messages count as failed too — they are never stored, so
    // every later gap-fill run re-detects them (and warns about them again).
    stats.failed += failed + parse_skipped;
    // Surface the skips and the incomplete header enumeration in the folder's
    // final message so the gap-fill running state carries a visible warning.
    stats.message = if parse_skipped > 0 {
        Some(format!(
            "{parse_skipped} email(s) could not be parsed and were skipped (not archived); \
             they will be re-detected on the next gap-fill run"
        ))
    } else if stats.failed > 0 {
        Some(format!(
            "{} email(s) could not be compared or downloaded; re-run gap-fill to retry them",
            stats.failed
        ))
    } else {
        None
    };
    // Final progress write into the independent gap-fill state (the folder is
    // done; the outcome lands in GapFillRun.folders via add_folder_result).
    GapFillState::update_folder_progress(
        account_id,
        remote_mailbox.name.clone(),
        planned,
        downloaded,
        None,
    )?;

    // Advance the mailbox's highest_uid so subsequent incremental syncs
    // start after the newly downloaded messages. Only do this on a complete,
    // uncancelled run where every planned message was downloaded and nothing
    // failed (phase-2 header batch failures leave uids outside `planned` that
    // must still be picked up by a later gap-fill run).
    if !cancelled && downloaded == planned && stats.failed == 0 {
        if let Some(max_uid) = missing_uids.last().copied() {
            let mut updated = remote_mailbox.clone();
            updated.highest_uid = Some(max_uid.max(local_mailbox.highest_uid.unwrap_or(0)));
            crate::archive::imap::mailbox::MailBox::batch_upsert(&[updated])?;
        }
    }

    Ok(stats)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::account::state::TriggerType;

    fn rh(uid: u32, message_id: Option<&str>, size: u64, internal_date: i64) -> RemoteHeader {
        RemoteHeader {
            uid,
            message_id: message_id.map(|s| s.to_string()),
            size,
            internal_date,
        }
    }

    fn snap(message_id: &str, uid: u64, size: u64, internal_date: i64) -> EnvelopeSnapshot {
        EnvelopeSnapshot {
            message_id: message_id.to_string(),
            uid,
            size,
            internal_date,
            //subject: String::new(),
        }
    }

    #[test]
    fn compute_missing_uids_message_id_diff() {
        let remote = vec![
            rh(1, Some("a"), 10, 1000),
            rh(2, Some("b"), 20, 2000),
            rh(3, Some("c"), 30, 3000),
        ];
        let local = vec![snap("a", 1, 10, 1000), snap("c", 3, 30, 3000)];
        let missing = compute_missing_uids(&remote, &local);
        assert_eq!(missing, vec![2]);
    }

    #[test]
    fn compute_missing_uids_fingerprint_fallback() {
        let remote = vec![rh(1, None, 10, 1000), rh(2, None, 20, 2000)];
        let local = vec![snap("generated-x", 1, 10, 1000)];
        let missing = compute_missing_uids(&remote, &local);
        assert_eq!(missing, vec![2]);
    }

    #[test]
    fn compute_missing_uids_remote_duplicates_all_present() {
        let remote = vec![rh(1, Some("dup"), 10, 1000), rh(2, Some("dup"), 10, 1000)];
        let local = vec![snap("dup", 1, 10, 1000)];
        let missing = compute_missing_uids(&remote, &local);
        assert!(missing.is_empty());
    }

    #[test]
    fn compute_missing_uids_local_duplicate_not_re_downloaded() {
        // A message-id appearing more than once locally is NOT a reason to
        // re-download: servers (Zoho, 163) legitimately reuse message-ids
        // across different messages, and re-downloading would be deduplicated
        // away anyway, so it can never repair the duplication.
        let remote = vec![rh(1, Some("dup"), 10, 1000), rh(2, Some("dup"), 10, 1000)];
        let local = vec![snap("dup", 1, 10, 1000), snap("dup", 2, 10, 1000)];
        let missing = compute_missing_uids(&remote, &local);
        assert!(missing.is_empty());
    }

    #[test]
    fn compute_missing_uids_msgid_mismatch_but_fingerprint_hit() {
        // The remote message-id does not exist locally (e.g. a different
        // message-id was stored by the SMTP path) but the fingerprint matches:
        // the message is already stored and must NOT be re-downloaded.
        let remote = vec![rh(1, Some("remote-id@x.com"), 10, 1000)];
        let local = vec![snap("generated-random-id", 1, 10, 1000)];
        let missing = compute_missing_uids(&remote, &local);
        assert!(missing.is_empty());
    }

    #[test]
    fn compute_missing_uids_msgid_mismatch_and_fingerprint_miss() {
        let remote = vec![
            rh(1, Some("remote-id@x.com"), 10, 1000),
            rh(2, Some("remote-id-2@x.com"), 20, 2000),
        ];
        let local = vec![snap("generated-random-id", 1, 10, 1000)];
        let missing = compute_missing_uids(&remote, &local);
        assert_eq!(missing, vec![2]);
    }

    // ------------------------------------------------------------------
    // gap_fill_mailbox against a scripted IMAP server
    // ------------------------------------------------------------------

    use crate::imap::mock_server::{
        examine_response, malformed_response, mock_mails as mails, register_mock_account,
        uid_fetch_message_id_response, uid_search_response, MockImapServer, MockImapServerHandle,
    };
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    struct Fixture {
        account: AccountModel,
        mailbox: MailBox,
        _server: MockImapServerHandle,
    }

    /// Registers an account on `server` with an empty, run-unique INBOX (so no
    /// local envelope of an earlier run can match) and an active download
    /// session plus gap-fill run to record into.
    async fn fixture(account_id: u64, server: MockImapServer) -> Fixture {
        let handle = server.start().await;
        let account = register_mock_account(&handle, account_id);
        DownloadState::init(account_id, TriggerType::Scheduled)
            .await
            .unwrap();
        GapFillState::start_run(account_id).unwrap();
        let mailbox = MailBox {
            id: 1_000_000 + (uuid::Uuid::new_v4().as_u128() % 1_000_000_000) as u64,
            account_id,
            name: "INBOX".into(),
            ..Default::default()
        };
        MailBox::batch_upsert(std::slice::from_ref(&mailbox)).unwrap();
        Fixture {
            account,
            mailbox,
            _server: handle,
        }
    }

    impl Fixture {
        async fn run(&self, token: CancellationToken) -> MailboxdResult<GapFillFolderStats> {
            gap_fill_mailbox(&self.account, &self.mailbox, &self.mailbox, token).await
        }

        fn stored_highest_uid(&self) -> Option<u32> {
            MailBox::get(self.mailbox.id).unwrap().highest_uid
        }

        fn live_message(&self) -> Option<String> {
            GapFillState::get(self.account.id)
                .unwrap()
                .and_then(|state| state.active)
                .and_then(|run| run.folders.get("INBOX").cloned())
                .and_then(|folder| folder.message)
        }

        fn session_errors(&self) -> Vec<String> {
            DownloadState::get(self.account.id)
                .unwrap()
                .and_then(|state| state.active_session)
                .map(|session| session.errors.into_iter().map(|e| e.error).collect())
                .unwrap_or_default()
        }
    }

    #[tokio::test]
    async fn gap_fill_downloads_missing_mail_and_advances_highest_uid() {
        let mails = mails(&[1, 2]);
        let f = fixture(59_342_301, MockImapServer::new().serving(&mails)).await;

        let stats = f.run(CancellationToken::new()).await.unwrap();

        assert_eq!(
            stats,
            GapFillFolderStats {
                downloaded: 2,
                failed: 0,
                candidate_count: 2,
                message: None,
            }
        );
        assert_eq!(f.stored_highest_uid(), Some(2));
    }

    #[tokio::test]
    async fn unparseable_mail_counts_as_failed_and_keeps_the_watermark() {
        let mut mails = mails(&[1, 2]);
        mails[1].body = Vec::new(); // mail-parser refuses an empty body
        let f = fixture(59_342_302, MockImapServer::new().serving(&mails)).await;

        let stats = f.run(CancellationToken::new()).await.unwrap();

        assert_eq!((stats.downloaded, stats.failed), (1, 1));
        assert_eq!(stats.candidate_count, 2);
        let message = stats.message.expect("parse skips are reported");
        assert!(
            message.starts_with("1 email(s) could not be parsed"),
            "{message}"
        );
        assert_eq!(f.stored_highest_uid(), None);
    }

    #[tokio::test]
    async fn failed_body_batch_is_reported_for_a_retry() {
        let mails = mails(&[1, 2]);
        let server = MockImapServer::new().respond("BODY.PEEK[]", malformed_response());
        let f = fixture(59_342_303, server.serving(&mails)).await;

        let stats = f.run(CancellationToken::new()).await.unwrap();

        assert_eq!((stats.downloaded, stats.failed), (0, 2));
        assert_eq!(
            stats.message.as_deref(),
            Some("2 email(s) could not be compared or downloaded; re-run gap-fill to retry them")
        );
        assert_eq!(f.stored_highest_uid(), None);
        assert!(f
            .session_errors()
            .iter()
            .any(|e| e.starts_with("Gap-fill batch 0 failed after 0 processed")));
    }

    #[tokio::test]
    async fn broken_header_batch_counts_its_uids_as_failed() {
        let mails = mails(&[1, 2, 3]);
        let server = MockImapServer::new().respond("HEADER.FIELDS", malformed_response());
        let f = fixture(59_342_304, server.serving(&mails)).await;

        let stats = f.run(CancellationToken::new()).await.unwrap();

        // Nothing could be compared, so nothing is a download candidate.
        assert_eq!(stats.failed, 3);
        assert_eq!((stats.downloaded, stats.candidate_count), (0, 0));
        assert!(f
            .session_errors()
            .iter()
            .any(|e| e.starts_with("Gap-fill header batch failed")));
    }

    #[tokio::test]
    async fn dropped_connection_during_header_fetch_is_retried_on_a_new_session() {
        let mails = mails(&[1, 2]);
        let server = MockImapServer::new().close_on("HEADER.FIELDS", 1);
        let f = fixture(59_342_305, server.serving(&mails)).await;

        // Includes the first 5 s reconnect backoff.
        let stats = f.run(CancellationToken::new()).await.unwrap();

        assert_eq!(
            (stats.downloaded, stats.failed, stats.candidate_count),
            (2, 0, 2)
        );
        assert_eq!(f.stored_highest_uid(), Some(2));
    }

    #[tokio::test]
    async fn header_batch_fails_when_the_reconnect_is_refused() {
        let mails = mails(&[1, 2]);
        let server = MockImapServer::new()
            .close_on("HEADER.FIELDS", 1)
            .respond_times("LOGIN", "{TAG} OK LOGIN completed\r\n", 1)
            .respond("LOGIN", "{TAG} NO [AUTHENTICATIONFAILED] denied\r\n");
        let f = fixture(59_342_306, server.serving(&mails)).await;

        let stats = f.run(CancellationToken::new()).await.unwrap();

        assert_eq!((stats.failed, stats.downloaded), (2, 0));
        assert!(f
            .session_errors()
            .iter()
            .any(|e| e.starts_with("Gap-fill header batch failed")));
    }

    #[tokio::test]
    async fn header_batch_fails_when_the_mailbox_cannot_be_reopened() {
        let mails = mails(&[1, 2]);
        let server = MockImapServer::new()
            .close_on("HEADER.FIELDS", 1)
            .respond_times("EXAMINE", examine_response("INBOX", 2, 42, 3), 1)
            .respond("EXAMINE", "{TAG} NO mailbox gone\r\n");
        let f = fixture(59_342_307, server.serving(&mails)).await;

        let stats = f.run(CancellationToken::new()).await.unwrap();

        assert_eq!((stats.failed, stats.downloaded), (2, 0));
        let errors = f.session_errors();
        assert!(errors
            .iter()
            .any(|e| e.starts_with("Gap-fill: re-examine failed after reconnect")));
        assert!(errors
            .iter()
            .any(|e| e.starts_with("Gap-fill header batch failed")));
    }

    #[tokio::test]
    async fn slow_header_fetch_shows_a_live_hint_and_still_completes() {
        let mails = mails(&[1, 2]);
        let headers: Vec<(u32, u32, &str)> = mails
            .iter()
            .map(|m| (m.uid, m.body.len() as u32, m.message_id.as_str()))
            .collect();
        let server = MockImapServer::new().respond_delayed(
            "HEADER.FIELDS",
            Duration::from_secs(6),
            uid_fetch_message_id_response(&headers),
        );
        let f = fixture(59_342_308, server.serving(&mails)).await;

        let (stats, hint) = tokio::join!(f.run(CancellationToken::new()), async {
            tokio::time::sleep(Duration::from_millis(5_500)).await;
            f.live_message()
        });

        let hint = hint.expect("a stalled header fetch is reported");
        assert!(hint.starts_with("IMAP server is slow"), "{hint}");
        let stats = stats.unwrap();
        assert_eq!((stats.downloaded, stats.failed), (2, 0));
    }

    #[tokio::test]
    async fn cancelling_a_stalled_search_stops_without_results() {
        let mails = mails(&[1, 2]);
        let server = MockImapServer::new().respond_delayed(
            "SEARCH",
            Duration::from_millis(10_500),
            uid_search_response(&[1, 2]),
        );
        let f = fixture(59_342_309, server.serving(&mails)).await;
        let token = CancellationToken::new();
        let logs = CapturedLogs::default();
        let _subscriber = tracing::subscriber::set_default(
            tracing_subscriber::fmt()
                .with_writer(logs.clone())
                .with_ansi(false)
                .finish(),
        );

        let (stats, hint) = tokio::join!(f.run(token.clone()), async {
            tokio::time::sleep(Duration::from_millis(5_500)).await;
            let hint = f.live_message();
            token.cancel();
            hint
        });

        let hint = hint.expect("a stalled search is reported");
        assert!(hint.starts_with("IMAP server is slow"), "{hint}");
        assert_eq!(stats.unwrap(), GapFillFolderStats::default());
        assert_eq!(f.stored_highest_uid(), None);
        assert!(
            logs.contents()
                .contains("gap-fill: UID SEARCH ALL taking long, still waiting"),
            "operators are warned about the slow search"
        );
    }

    /// Collects the formatted log output of a thread-local subscriber.
    #[derive(Clone, Default)]
    struct CapturedLogs(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl CapturedLogs {
        fn contents(&self) -> String {
            std::io::Write::flush(&mut self.clone()).unwrap();
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }
    }

    impl std::io::Write for CapturedLogs {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedLogs {
        type Writer = CapturedLogs;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }
}
