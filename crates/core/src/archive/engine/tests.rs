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
