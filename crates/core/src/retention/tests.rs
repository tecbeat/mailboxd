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

use std::collections::HashSet;

use tokio::sync::Mutex;

use super::*;
use crate::{
    error::code::ErrorCode,
    raise_error,
    store::{
        blob::BLOB_MANAGER,
        tantivy::{
            envelope::test_support::{
                blob_key, commit_documents, envelope, envelope_document, init_globals,
                register_mailbox, store_email_blob, unique_id,
            },
            model::AttachmentModel,
        },
    },
};

/// `run_sweep` covers every stored account, so the sweep tests take turns.
static SWEEP_LOCK: Mutex<()> = Mutex::const_new(());

fn account_with(days: Option<u64>) -> AccountModel {
    AccountModel {
        id: unique_id(),
        email: "retention@example.com".into(),
        retention_days: days,
        ..Default::default()
    }
}

#[test]
fn retention_is_off_without_a_positive_window() {
    assert_eq!(cutoff_ms(&account_with(None)), None);
    assert_eq!(cutoff_ms(&account_with(Some(0))), None);

    let before = utc_now!();
    let cutoff = cutoff_ms(&account_with(Some(2))).unwrap();
    let after = utc_now!();
    assert!((before - 2 * DAY_MS..=after - 2 * DAY_MS).contains(&cutoff));
}

#[test]
fn the_cutoff_saturates_instead_of_overflowing() {
    assert_eq!(cutoff_for(1, 10 * DAY_MS), 9 * DAY_MS);
    assert_eq!(cutoff_for(u64::MAX, 0), -i64::MAX);
    assert_eq!(cutoff_for(36_500, i64::MIN + 1), i64::MIN);
}

#[test]
fn the_effective_date_is_the_older_known_timestamp() {
    assert_eq!(effective_date_ms(0, 0), None);
    assert_eq!(effective_date_ms(5, 0), Some(5));
    assert_eq!(effective_date_ms(0, 7), Some(7));
    assert_eq!(effective_date_ms(5, 7), Some(5));
    assert_eq!(effective_date_ms(9, 7), Some(7));
}

#[test]
fn only_mail_before_the_cutoff_is_expired() {
    assert!(is_expired(99, 500, 100));
    assert!(is_expired(500, 99, 100));
    assert!(is_expired(0, 99, 100));
    assert!(!is_expired(100, 100, 100), "the cutoff itself is kept");
    assert!(!is_expired(0, 0, 100), "mail without a date is kept");
    assert!(!is_expired(200, 300, 100));
}

#[test]
fn imap_searches_are_narrowed_to_the_window() {
    let off = account_with(None);
    assert_eq!(imap_search_criteria(&off, "ALL"), "ALL");
    assert_eq!(
        imap_search_criteria(&off, "SINCE 01-Jan-2024"),
        "SINCE 01-Jan-2024"
    );

    let on = account_with(Some(30));
    let floor = chrono::DateTime::from_timestamp_millis(cutoff_ms(&on).unwrap())
        .unwrap()
        .format("SINCE %d-%b-%Y")
        .to_string();
    assert_eq!(imap_search_criteria(&on, "ALL"), floor);
    assert_eq!(
        imap_search_criteria(&on, "BEFORE 01-Jan-2024"),
        format!("BEFORE 01-Jan-2024 {floor}")
    );
}

#[test]
fn imap_searches_are_not_narrowed_when_the_cutoff_has_no_date() {
    // A window reaching before the representable dates keeps everything.
    let account = account_with(Some(u64::MAX));
    assert_eq!(imap_search_criteria(&account, "ALL"), "ALL");
}

#[test]
fn a_sweep_that_deleted_nothing_is_not_audited() {
    let account = account_with(Some(30));
    assert!(purge_event(&account, 30, 0, Ok(())).is_none());
}

#[test]
fn a_sweep_is_audited_with_its_count_and_outcome() {
    let account = account_with(Some(30));
    let Some(Event::RetentionPurged {
        account: email,
        deleted,
        window_days,
        success,
    }) = purge_event(&account, 30, 4, Ok(()))
    else {
        panic!("expected a retention event");
    };
    assert_eq!(
        (email.as_str(), deleted, window_days, success),
        ("retention@example.com", 4, 30, true)
    );

    let failed = purge_event(
        &account,
        30,
        2,
        Err(raise_error!("boom".into(), ErrorCode::InternalError)),
    );
    assert!(matches!(
        failed,
        Some(Event::RetentionPurged {
            deleted: 2,
            success: false,
            ..
        })
    ));

    // A failure is audited even when nothing was deleted yet.
    let failed_early = purge_event(
        &account,
        30,
        0,
        Err(raise_error!("boom".into(), ErrorCode::InternalError)),
    );
    assert!(matches!(
        failed_early,
        Some(Event::RetentionPurged {
            deleted: 0,
            success: false,
            ..
        })
    ));
}

/// An account with archived mail in the global indexes.
struct Archive {
    account: AccountModel,
    mailbox_id: u64,
}

impl Archive {
    fn new(days: Option<u64>) -> Self {
        let account = account_with(days);
        let mailbox_id = unique_id();
        register_mailbox(account.id, mailbox_id, &account.email);
        Self {
            account,
            mailbox_id,
        }
    }

    /// Archive envelope `id`, `age_days` old, stored in blob `content_hash`.
    async fn add(&self, id: &str, age_days: i64, content_hash: &str) {
        let date = utc_now!() - age_days * DAY_MS;
        let mut envelope = envelope(self.account.id, self.mailbox_id, id);
        envelope.date = date;
        envelope.internal_date = date;
        envelope.content_hash = content_hash.to_string();
        commit_documents(
            &ENVELOPE_MANAGER,
            vec![envelope_document(envelope, None, "")],
        )
        .await;
        let mut writer = ATTACHMENT_MANAGER.index_writer().lock().await;
        writer
            .add_document(
                AttachmentModel {
                    id: id.to_string(),
                    envelope_id: id.to_string(),
                    account_id: self.account.id,
                    mailbox_id: self.mailbox_id,
                    content_hash: blob_key(unique_id()),
                    date,
                    ..Default::default()
                }
                .into_document(),
            )
            .unwrap();
        writer.commit().unwrap();
    }

    fn accounts(&self) -> Option<HashSet<u64>> {
        Some(HashSet::from([self.account.id]))
    }

    fn emails(&self) -> u64 {
        ENVELOPE_MANAGER.total_emails(&self.accounts()).unwrap()
    }

    fn attachments(&self) -> u64 {
        ATTACHMENT_MANAGER
            .total_attachments(&self.accounts())
            .unwrap()
    }

    fn remaining_ids(&self) -> Vec<String> {
        ENVELOPE_MANAGER
            .retention_expired_ids(self.account.id, i64::MAX, 100)
            .unwrap()
    }
}

async fn blob(eml: &str) -> String {
    let hash = blob_key(unique_id());
    store_email_blob(&hash, eml.as_bytes()).await;
    hash
}

fn has_blob(hash: &str) -> bool {
    BLOB_MANAGER.get_email(hash).unwrap().is_some()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_purge_deletes_old_mail_with_its_attachments_and_unshared_blobs() {
    init_globals();
    let _turn = SWEEP_LOCK.lock().await;
    let archive = Archive::new(Some(30));
    let old_own = blob("Subject: old\r\n\r\nold\r\n").await;
    let shared = blob("Subject: shared\r\n\r\nshared\r\n").await;
    let recent_own = blob("Subject: recent\r\n\r\nrecent\r\n").await;
    archive.add("old", 40, &old_own).await;
    archive.add("old-shared", 31, &shared).await;
    archive.add("recent-shared", 29, &shared).await;
    archive.add("recent", 1, &recent_own).await;

    let event = purge_account(&archive.account).await;

    assert!(matches!(
        event,
        Some(Event::RetentionPurged {
            deleted: 2,
            window_days: 30,
            success: true,
            ..
        })
    ));
    assert_eq!(archive.emails(), 2);
    assert_eq!(archive.attachments(), 2);
    let mut remaining = archive.remaining_ids();
    remaining.sort();
    assert_eq!(remaining, ["recent", "recent-shared"]);
    assert!(!has_blob(&old_own));
    assert!(has_blob(&shared), "a blob still referenced survives");
    assert!(has_blob(&recent_own));

    // Nothing is left to delete, so the next run records nothing.
    assert!(purge_account(&archive.account).await.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_purge_runs_in_batches_until_nothing_is_left() {
    init_globals();
    let _turn = SWEEP_LOCK.lock().await;
    let archive = Archive::new(Some(10));
    for id in ["a", "b", "c"] {
        archive.add(id, 20, &blob_key(unique_id())).await;
    }
    archive.add("kept", 5, &blob_key(unique_id())).await;

    let mut deleted = 0;
    let cutoff = cutoff_ms(&archive.account).unwrap();
    purge_before(archive.account.id, cutoff, 2, &mut deleted)
        .await
        .unwrap();

    assert_eq!(deleted, 3);
    assert_eq!(archive.remaining_ids(), ["kept"]);
    assert_eq!(archive.attachments(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_purge_without_a_window_keeps_everything() {
    init_globals();
    let _turn = SWEEP_LOCK.lock().await;
    let archive = Archive::new(None);
    archive.add("ancient", 10_000, &blob_key(unique_id())).await;

    assert!(purge_account(&archive.account).await.is_none());
    assert_eq!(archive.emails(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_sweep_skips_accounts_that_are_being_deleted() {
    init_globals();
    let _turn = SWEEP_LOCK.lock().await;
    let active = Archive::new(Some(30));
    let mut deleting = Archive::new(Some(30));
    deleting.account.deleting = true;
    active.add("active-old", 40, &blob_key(unique_id())).await;
    deleting
        .add("deleting-old", 40, &blob_key(unique_id()))
        .await;

    let events = sweep(&[active.account.clone(), deleting.account.clone()]).await;

    assert_eq!(events.len(), 1);
    assert_eq!(active.emails(), 0);
    assert_eq!(deleting.emails(), 1, "the account deletion owns its mail");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_scheduled_sweep_purges_stored_accounts() {
    init_globals();
    let _turn = SWEEP_LOCK.lock().await;
    let archive = Archive::new(Some(30));
    crate::database::upsert_impl(
        crate::database::manager::DB_MANAGER.db(),
        archive.account.clone(),
    )
    .unwrap();
    archive.add("stored-old", 40, &blob_key(unique_id())).await;
    archive
        .add("stored-recent", 1, &blob_key(unique_id()))
        .await;

    run_sweep().await.unwrap();

    assert_eq!(archive.remaining_ids(), ["stored-recent"]);
}

#[test]
fn the_periodic_task_sweeps_stored_accounts_on_its_interval() {
    init_globals();
    // Held until the runtime and with it the task are gone.
    let _turn = SWEEP_LOCK.blocking_lock();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let archive = runtime.block_on(async {
        let archive = Archive::new(Some(30));
        crate::database::upsert_impl(
            crate::database::manager::DB_MANAGER.db(),
            archive.account.clone(),
        )
        .unwrap();
        archive.add("periodic-old", 40, &blob_key(unique_id())).await;
        archive
    });

    let _task = task::tests::start_on(&runtime, std::time::Duration::from_millis(50));
    runtime.block_on(async {
        for _ in 0..200 {
            if archive.emails() == 0 {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("the periodic sweep never purged the old mail");
    });
    drop(runtime);
}
