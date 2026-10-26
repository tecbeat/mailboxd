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

//! Persisted records of blob repairs.
//!
//! Stored in the memdb `blob_repair_reports` collection as a bounded ring:
//! the [`BlobRepairRecord::HISTORY_LIMIT`] most recent attempts, pruned on
//! every write. It is a diagnostic history, not the audit trail (that is the
//! `email.repaired` audit event): repairs that found nothing wrong are not
//! recorded, and anything older than the ring is dropped.

use serde::{Deserialize, Serialize};

use crate::database::{
    batch_delete_impl, list_all_impl, manager::DB_MANAGER, upsert_impl, MemDbModel,
};
use crate::envelope::extractor::{BlobRepairReport, STATUS_COMPLETE};
use crate::error::MailboxdResult;
use crate::ext::event_bus::{emit, Event};

/// One blob-repair attempt persisted in the memdb.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct BlobRepairRecord {
    pub id: String,
    /// UTC millis when the repair finished.
    pub created_at: i64,
    /// Id of the user who triggered the repair.
    pub operator_id: u64,
    /// Name of the user who triggered the repair.
    pub operator: String,
    /// Account that owns the repaired record.
    pub account_id: u64,
    pub mailbox_id: u64,
    pub subject: String,
    pub message_id: String,
    pub report: BlobRepairReport,
}

impl MemDbModel for BlobRepairRecord {
    fn collection() -> &'static str {
        "blob_repair_reports"
    }

    /// Time-ordered key: the zero-padded millis prefix makes memdb's key
    /// order chronological, so [`BlobRepairRecord::list_recent`] needs no
    /// sort and [`BlobRepairRecord::prune_beyond`] evicts oldest first.
    /// Records of the same millisecond are ordered by their random id.
    fn key(&self) -> String {
        format!("{:013}-{}", self.created_at, self.id)
    }
}

impl BlobRepairRecord {
    /// Cap on persisted records.
    pub const HISTORY_LIMIT: usize = 200;

    /// Persists one repair attempt and prunes the ring. Returns `false`,
    /// writing nothing, when the record was already intact: such a no-op
    /// would otherwise push a real failure out of the ring.
    pub fn persist(
        operator_id: u64,
        operator: &str,
        report: &BlobRepairReport,
    ) -> MailboxdResult<bool> {
        if report.status == STATUS_COMPLETE {
            return Ok(false);
        }
        let record = Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: crate::utc_now!(),
            operator_id,
            operator: operator.to_string(),
            account_id: report.account_id,
            mailbox_id: report.mailbox_id,
            subject: report.subject.clone(),
            message_id: report.message_id.clone(),
            report: report.clone(),
        };
        upsert_impl::<Self>(DB_MANAGER.db(), record)?;
        Self::prune_beyond(Self::HISTORY_LIMIT)?;
        Ok(true)
    }

    /// Records a finished repair: persists it in the history and writes the
    /// `email.repaired` audit event. A repair that found the record intact
    /// leaves no trace in either. The history is best-effort, so a failed
    /// write is logged and does not fail the repair.
    pub fn record_attempt(operator_id: u64, operator: &str, report: &BlobRepairReport) {
        if let Err(e) = Self::persist(operator_id, operator, report) {
            tracing::warn!(
                "Failed to persist the blob repair report of {}/{}: {e:#?}",
                report.account_id,
                report.envelope_id
            );
        }
        if let Some(event) = audit_event(operator, report) {
            emit(event);
        }
    }

    /// The most recent `limit` records the caller may see, newest first,
    /// optionally only those of `account_id`. `may_see` decides per account.
    pub fn visible(
        limit: usize,
        account_id: Option<u64>,
        may_see: impl Fn(u64) -> bool,
    ) -> MailboxdResult<Vec<BlobRepairRecord>> {
        Ok(Self::list_recent()?
            .into_iter()
            .filter(|r| account_id.is_none_or(|id| r.account_id == id))
            .filter(|r| may_see(r.account_id))
            .take(limit)
            .collect())
    }

    /// All records, newest first.
    pub fn list_recent() -> MailboxdResult<Vec<BlobRepairRecord>> {
        let mut all: Vec<BlobRepairRecord> = list_all_impl(DB_MANAGER.db())?;
        all.reverse();
        Ok(all)
    }

    /// Deletes the records beyond the most recent `limit`, oldest first, in
    /// one transaction. Returns how many were removed.
    pub fn prune_beyond(limit: usize) -> MailboxdResult<usize> {
        let all: Vec<BlobRepairRecord> = list_all_impl(DB_MANAGER.db())?;
        if all.len() <= limit {
            return Ok(0);
        }
        let excess = all.len() - limit;
        let keys: Vec<String> = all.into_iter().take(excess).map(|r| r.key()).collect();
        batch_delete_impl::<Self>(DB_MANAGER.db(), keys)
    }
}

/// The audit event of a repair, or `None` when there was nothing to repair.
fn audit_event(operator: &str, report: &BlobRepairReport) -> Option<Event> {
    (report.status != STATUS_COMPLETE).then(|| Event::EmailRepaired {
        email_id: format!("{}/{}", report.account_id, report.envelope_id),
        user: operator.to_string(),
        status: report.status.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::extractor::{STATUS_FAILED, STATUS_PARTIAL, STATUS_REPAIRED};
    use std::sync::Mutex;

    /// The tests share the one history collection and clear it.
    static HISTORY: Mutex<()> = Mutex::new(());

    fn lock_cleared() -> std::sync::MutexGuard<'static, ()> {
        let guard = HISTORY.lock().unwrap_or_else(|e| e.into_inner());
        // The database's WAL writer needs the shared test runtime.
        crate::store::tantivy::envelope::test_support::init_globals();
        BlobRepairRecord::prune_beyond(0).unwrap();
        guard
    }

    fn report(account_id: u64, envelope_id: &str, status: &str) -> BlobRepairReport {
        BlobRepairReport {
            envelope_id: envelope_id.into(),
            account_id,
            mailbox_id: 9,
            subject: format!("Subject of {envelope_id}"),
            message_id: format!("<{envelope_id}@example.com>"),
            status: status.into(),
            ..Default::default()
        }
    }

    fn record_at(created_at: i64, account_id: u64, envelope_id: &str) -> BlobRepairRecord {
        BlobRepairRecord {
            id: envelope_id.into(),
            created_at,
            operator_id: 1,
            operator: "admin".into(),
            account_id,
            mailbox_id: 9,
            subject: String::new(),
            message_id: String::new(),
            report: report(account_id, envelope_id, STATUS_FAILED),
        }
    }

    fn insert(records: &[BlobRepairRecord]) {
        for record in records {
            upsert_impl(DB_MANAGER.db(), record.clone()).unwrap();
        }
    }

    fn envelope_ids(records: &[BlobRepairRecord]) -> Vec<&str> {
        records
            .iter()
            .map(|r| r.report.envelope_id.as_str())
            .collect()
    }

    #[test]
    fn an_intact_record_is_not_persisted() {
        let _guard = lock_cleared();
        let persisted = BlobRepairRecord::persist(1, "admin", &report(1, "e1", STATUS_COMPLETE));
        assert!(!persisted.unwrap());
        assert!(BlobRepairRecord::list_recent().unwrap().is_empty());
    }

    #[test]
    fn an_attempted_repair_is_persisted_with_its_operator_and_report() {
        let _guard = lock_cleared();
        for status in [STATUS_REPAIRED, STATUS_PARTIAL, STATUS_FAILED] {
            let report = report(4, status, status);
            assert!(BlobRepairRecord::persist(7, "dave", &report).unwrap());
        }

        let records = BlobRepairRecord::list_recent().unwrap();
        assert_eq!(records.len(), 3);
        let record = records
            .iter()
            .find(|r| r.report.status == STATUS_PARTIAL)
            .unwrap();
        assert_eq!(record.operator_id, 7);
        assert_eq!(record.operator, "dave");
        assert_eq!(record.account_id, 4);
        assert_eq!(record.mailbox_id, 9);
        assert_eq!(record.subject, "Subject of partial");
        assert_eq!(record.message_id, "<partial@example.com>");
        assert_eq!(record.report, report(4, STATUS_PARTIAL, STATUS_PARTIAL));
        assert!(record.created_at > 0);
    }

    #[test]
    fn records_are_listed_newest_first() {
        let _guard = lock_cleared();
        insert(&[
            record_at(2_000, 1, "middle"),
            record_at(3_000, 1, "newest"),
            record_at(1_000, 1, "oldest"),
        ]);
        let records = BlobRepairRecord::list_recent().unwrap();
        assert_eq!(envelope_ids(&records), ["newest", "middle", "oldest"]);
    }

    #[test]
    fn the_key_orders_by_time_across_digit_counts() {
        // Without the zero padding "999-" would sort after "1000-".
        assert!(record_at(999, 1, "a").key() < record_at(1_000, 1, "a").key());
    }

    #[test]
    fn pruning_drops_the_oldest_records() {
        let _guard = lock_cleared();
        insert(&[
            record_at(1_000, 1, "first"),
            record_at(2_000, 1, "second"),
            record_at(3_000, 1, "third"),
        ]);
        assert_eq!(BlobRepairRecord::prune_beyond(3).unwrap(), 0);
        assert_eq!(BlobRepairRecord::prune_beyond(1).unwrap(), 2);
        let records = BlobRepairRecord::list_recent().unwrap();
        assert_eq!(envelope_ids(&records), ["third"]);
    }

    #[test]
    fn persisting_keeps_the_history_at_its_limit() {
        let _guard = lock_cleared();
        let old: Vec<_> = (0..BlobRepairRecord::HISTORY_LIMIT as i64)
            .map(|i| record_at(1_000 + i, 1, &format!("old-{i}")))
            .collect();
        insert(&old);

        BlobRepairRecord::persist(1, "admin", &report(1, "fresh", STATUS_FAILED)).unwrap();

        let records = BlobRepairRecord::list_recent().unwrap();
        assert_eq!(records.len(), BlobRepairRecord::HISTORY_LIMIT);
        assert_eq!(records[0].report.envelope_id, "fresh");
        assert!(
            !envelope_ids(&records).contains(&"old-0"),
            "the oldest record is dropped"
        );
        assert!(envelope_ids(&records).contains(&"old-1"));
    }

    #[test]
    fn visible_filters_by_account_and_permission_before_the_limit() {
        let _guard = lock_cleared();
        insert(&[
            record_at(1_000, 1, "a1-old"),
            record_at(2_000, 2, "a2"),
            record_at(3_000, 1, "a1-new"),
            record_at(4_000, 3, "a3-hidden"),
        ]);
        let not_account_3 = |id: u64| id != 3;

        let all = BlobRepairRecord::visible(10, None, not_account_3).unwrap();
        assert_eq!(envelope_ids(&all), ["a1-new", "a2", "a1-old"]);

        // The hidden newest record must not use up the limit.
        let two = BlobRepairRecord::visible(2, None, not_account_3).unwrap();
        assert_eq!(envelope_ids(&two), ["a1-new", "a2"]);

        let account_1 = BlobRepairRecord::visible(10, Some(1), not_account_3).unwrap();
        assert_eq!(envelope_ids(&account_1), ["a1-new", "a1-old"]);

        let forbidden = BlobRepairRecord::visible(10, Some(3), not_account_3).unwrap();
        assert!(forbidden.is_empty());
    }

    #[test]
    fn record_attempt_persists_attempts_only() {
        let _guard = lock_cleared();
        BlobRepairRecord::record_attempt(1, "admin", &report(5, "intact", STATUS_COMPLETE));
        BlobRepairRecord::record_attempt(1, "admin", &report(5, "broken", STATUS_FAILED));
        let records = BlobRepairRecord::list_recent().unwrap();
        assert_eq!(envelope_ids(&records), ["broken"]);
    }

    #[test]
    fn an_attempted_repair_has_an_audit_event() {
        for status in [STATUS_REPAIRED, STATUS_PARTIAL, STATUS_FAILED] {
            match audit_event("dave", &report(5, "e1", status)) {
                Some(Event::EmailRepaired {
                    email_id,
                    user,
                    status: event_status,
                }) => {
                    assert_eq!(email_id, "5/e1");
                    assert_eq!(user, "dave");
                    assert_eq!(event_status, status);
                }
                other => panic!("unexpected event for {status}: {other:?}"),
            }
        }
    }

    #[test]
    fn an_intact_record_has_no_audit_event() {
        assert!(audit_event("dave", &report(5, "e1", STATUS_COMPLETE)).is_none());
    }
}
