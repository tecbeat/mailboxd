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

//! Unit tests for the source-generic engine's pure selection logic.

use crate::account::migration::{Account, AccountType};
use crate::archive::imap::mailbox::MailBox;

use super::select_mailboxes;

fn mailbox(account_id: u64, name: &str) -> MailBox {
    MailBox {
        account_id,
        name: name.to_string(),
        ..Default::default()
    }
}

fn account_with_folders(folders: Option<Vec<String>>) -> Account {
    Account {
        id: 1,
        account_type: AccountType::JMAP,
        download_folders: folders,
        ..Default::default()
    }
}

#[test]
fn selects_only_subscribed_folders_when_set() {
    let remote = vec![
        mailbox(1, "Inbox"),
        mailbox(1, "Sent"),
        mailbox(1, "Archive"),
    ];
    let account = account_with_folders(Some(vec!["Inbox".into(), "Archive".into()]));
    let selected = select_mailboxes(&account, &remote);
    let names: Vec<_> = selected.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, vec!["Inbox", "Archive"]);
}

#[test]
fn selects_all_folders_when_subscription_is_none() {
    let remote = vec![mailbox(1, "Inbox"), mailbox(1, "Sent")];
    let account = account_with_folders(None);
    let selected = select_mailboxes(&account, &remote);
    assert_eq!(selected.len(), 2);
}

#[test]
fn selects_all_folders_when_subscription_is_empty() {
    let remote = vec![mailbox(1, "Inbox"), mailbox(1, "Sent")];
    let account = account_with_folders(Some(vec![]));
    let selected = select_mailboxes(&account, &remote);
    assert_eq!(selected.len(), 2);
}

#[test]
fn ignores_subscribed_folders_not_present_remotely() {
    let remote = vec![mailbox(1, "Inbox")];
    let account = account_with_folders(Some(vec!["Inbox".into(), "DoesNotExist".into()]));
    let selected = select_mailboxes(&account, &remote);
    let names: Vec<_> = selected.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, vec!["Inbox"]);
}

mod cursor_persistence {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use tokio_util::sync::CancellationToken;

    use crate::account::migration::{Account, AccountType};
    use crate::account::state::TriggerType;
    use crate::archive::engine::run_sync;
    use crate::archive::imap::mailbox::MailBox;
    use crate::archive::source::{
        MailSession, MailSource, MailboxChanges, MessageRef, RawMessage, SyncCursor,
    };
    use crate::error::MailboxdResult;

    /// A source with one empty mailbox that hands out a fixed cursor and
    /// records the cursor the engine passes into every `changes_since` call.
    struct RecordingSource {
        mailbox: MailBox,
        seen_cursors: Arc<Mutex<Vec<Option<String>>>>,
    }

    struct RecordingSession {
        mailbox: MailBox,
        seen_cursors: Arc<Mutex<Vec<Option<String>>>>,
    }

    #[async_trait]
    impl MailSource for RecordingSource {
        fn name(&self) -> &'static str {
            "recording"
        }

        async fn connect(&self, _account_id: u64) -> MailboxdResult<Box<dyn MailSession>> {
            Ok(Box::new(RecordingSession {
                mailbox: self.mailbox.clone(),
                seen_cursors: self.seen_cursors.clone(),
            }))
        }
    }

    #[async_trait]
    impl MailSession for RecordingSession {
        async fn list_mailboxes(&mut self) -> MailboxdResult<Vec<MailBox>> {
            Ok(vec![self.mailbox.clone()])
        }

        async fn changes_since(
            &mut self,
            _mailbox: &MailBox,
            cursor: Option<SyncCursor>,
            _token: CancellationToken,
        ) -> MailboxdResult<MailboxChanges> {
            self.seen_cursors
                .lock()
                .unwrap()
                .push(cursor.map(|c| c.as_str().to_string()));
            Ok(MailboxChanges {
                cursor: Some(SyncCursor::new("state-1")),
                ..Default::default()
            })
        }

        async fn load_raw(
            &mut self,
            _mailbox: &MailBox,
            _msg: &MessageRef,
        ) -> MailboxdResult<RawMessage> {
            unreachable!("the recording mailbox has no messages")
        }

        async fn append(&mut self, _mailbox: &str, _eml: &[u8]) -> MailboxdResult<()> {
            Ok(())
        }

        async fn logout(&mut self) -> MailboxdResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn second_sync_resumes_from_the_stored_cursor() {
        // Fresh ids: the test database outlives a single test run.
        let account_id = crate::id!(64);
        let seen_cursors = Arc::new(Mutex::new(Vec::new()));
        let source = RecordingSource {
            mailbox: MailBox {
                id: crate::id!(64),
                account_id,
                name: "INBOX".to_string(),
                ..Default::default()
            },
            seen_cursors: seen_cursors.clone(),
        };
        let account = Account {
            id: account_id,
            account_type: AccountType::JMAP,
            ..Default::default()
        };

        for _ in 0..2 {
            run_sync(&account, &source, CancellationToken::new(), TriggerType::Manual)
                .await
                .unwrap();
        }

        assert_eq!(
            *seen_cursors.lock().unwrap(),
            vec![None, Some("state-1".to_string())]
        );
    }
}
