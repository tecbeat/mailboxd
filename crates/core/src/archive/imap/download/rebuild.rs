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

use crate::{
    account::{
        migration::AccountModel,
        state::{DownloadState, DownloadStatus, FolderStatus},
    },
    archive::{
        imap::{
            download::flow::{fetch_and_save_by_date, fetch_and_save_full_mailbox, FetchDirection},
            mailbox::MailBox,
        },
        SEMAPHORE,
    },
    error::{code::ErrorCode, MailboxdResult},
    raise_error,
    store::tantivy::{attachment::ATTACHMENT_MANAGER, envelope::ENVELOPE_MANAGER},
};

use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub async fn rebuild_cache(
    account: &AccountModel,
    remote_mailboxes: &[MailBox],
    token: CancellationToken,
) -> MailboxdResult<()> {
    MailBox::batch_insert(remote_mailboxes)?;
    DownloadState::init_folder_details(
        account.id,
        remote_mailboxes.iter().map(|m| m.name.clone()).collect(),
    )?;

    let mut has_error = false;
    let mut last_err = None;

    for mailbox in remote_mailboxes {
        if token.is_cancelled() {
            DownloadState::update_session_status(
                account.id,
                DownloadStatus::Cancelled,
                Some("Received termination signal (User stop or System shutdown)".to_string()),
            )?;
            break;
        }
        if mailbox.exists == 0 {
            info!(
                "Account {}: Mailbox '{}' on the remote server has no emails. Skipping fetch for this mailbox.",
                account.id, &mailbox.name
            );
            DownloadState::update_folder_progress(
                account.id,
                mailbox.name.clone(),
                0,
                0,
                FolderStatus::Success,
                None,
            )?;
            continue;
        }
        let account = account.clone();
        let mailbox = mailbox.clone();

        let _global_permit = match SEMAPHORE.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(err) => {
                error!(
                    "Failed to acquire global semaphore permit for account {} mailbox '{}': {:#?}",
                    account.id, &mailbox.name, err
                );
                continue;
            }
        };

        match fetch_and_save_full_mailbox(&account, &mailbox, token.clone()).await {
            Ok(new_highest_uid) => {
                // Only persist a watermark when the folder actually
                // completed: Ok(None) means the download broke mid-way (or
                // the folder is empty), and writing it would clear the row
                // and hide the un-downloaded range from later syncs (#377).
                if let Some(uid) = new_highest_uid {
                    let mut updated = mailbox.clone();
                    updated.highest_uid = Some(uid);
                    MailBox::batch_upsert(&[updated])?;
                }
            }
            Err(err) => {
                has_error = true;
                tracing::error!("Folder sync task failed: {:#?}", err);
                last_err = Some(err);
            }
        }
    }

    if has_error {
        if let Some(e) = last_err {
            return Err(e);
        }
        return Err(raise_error!(
            "Some tasks failed".into(),
            ErrorCode::InternalError
        ));
    }
    Ok(())
}

pub async fn rebuild_cache_by_date(
    account: &AccountModel,
    remote_mailboxes: &[MailBox],
    date: &str,
    direction: FetchDirection,
    token: CancellationToken,
) -> MailboxdResult<()> {
    MailBox::batch_insert(remote_mailboxes)?;
    DownloadState::init_folder_details(
        account.id,
        remote_mailboxes.iter().map(|m| m.name.clone()).collect(),
    )?;

    let mut has_error = false;
    let mut last_err = None;

    for mailbox in remote_mailboxes {
        if token.is_cancelled() {
            DownloadState::update_session_status(
                account.id,
                DownloadStatus::Cancelled,
                Some("Received termination signal (User stop or System shutdown)".to_string()),
            )?;
            break;
        }
        if mailbox.exists == 0 {
            info!(
                "Account {}: Mailbox '{}' on the remote server has no emails. Skipping fetch for this mailbox.",
                account.id, &mailbox.name
            );

            DownloadState::update_folder_progress(
                account.id,
                mailbox.name.clone(),
                0,
                0,
                FolderStatus::Success,
                None,
            )?;
            continue;
        }
        let account = account.clone();
        let mailbox = mailbox.clone();
        let date = date.to_string();
        let direction = direction.clone();

        let _global_permit = match SEMAPHORE.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(err) => {
                error!(
                    "Failed to acquire global semaphore permit for account {} mailbox '{}': {:#?}",
                    account.id, &mailbox.name, err
                );
                continue;
            }
        };
        match fetch_and_save_by_date(&account, date.as_str(), &mailbox, direction, token.clone())
            .await
        {
            Ok(new_highest_uid) => {
                // Only persist a watermark when the folder actually
                // completed: Ok(None) means the download broke mid-way (or
                // the folder is empty), and writing it would clear the row
                // and hide the un-downloaded range from later syncs (#377).
                if let Some(uid) = new_highest_uid {
                    let mut updated = mailbox.clone();
                    updated.highest_uid = Some(uid);
                    MailBox::batch_upsert(&[updated])?;
                }
            }
            Err(err) => {
                has_error = true;
                tracing::error!("Folder sync task failed: {:#?}", err);
                last_err = Some(err);
            }
        }
    }

    if has_error {
        if let Some(e) = last_err {
            return Err(e);
        }
        return Err(raise_error!(
            "Some tasks failed".into(),
            ErrorCode::InternalError
        ));
    }

    Ok(())
}

pub async fn rebuild_mailbox_cache(
    account: &AccountModel,
    local_mailbox: &MailBox,
    remote_mailbox: &MailBox,
    token: CancellationToken,
) -> MailboxdResult<Option<u32>> {
    ENVELOPE_MANAGER
        .delete_mailbox_envelopes(account.id, vec![local_mailbox.id])
        .await?;
    ATTACHMENT_MANAGER
        .delete_mailbox_attachments(account.id, vec![local_mailbox.id])
        .await?;
    if remote_mailbox.exists == 0 {
        info!(
            "Account {}: Mailbox '{}' has no emails on the remote server. The mailbox is empty, no envelopes to fetch.",
            account.id,
            &local_mailbox.name
        );
        DownloadState::update_folder_progress(
            account.id,
            remote_mailbox.name.clone(),
            0,
            0,
            FolderStatus::Success,
            None,
        )?;
        return Ok(None);
    }

    let result = fetch_and_save_full_mailbox(account, remote_mailbox, token).await?;
    Ok(result)
}

pub async fn rebuild_mailbox_cache_by_date(
    account: &AccountModel,
    local_mailbox_id: u64,
    date: &str,
    remote: &MailBox,
    direction: FetchDirection,
    token: CancellationToken,
) -> MailboxdResult<Option<u32>> {
    ENVELOPE_MANAGER
        .delete_mailbox_envelopes(account.id, vec![local_mailbox_id])
        .await?;
    ATTACHMENT_MANAGER
        .delete_mailbox_attachments(account.id, vec![local_mailbox_id])
        .await?;
    if remote.exists == 0 {
        info!(
            "Account {}: Mailbox '{}' has no emails on the remote server. The mailbox is empty, no envelopes to fetch.",
            account.id,
            &remote.name
        );
        DownloadState::update_folder_progress(
            account.id,
            remote.name.clone(),
            0,
            0,
            FolderStatus::Success,
            None,
        )?;
        return Ok(None);
    }

    let result = fetch_and_save_by_date(account, date, remote, direction, token).await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::state::TriggerType;
    use crate::imap::mock_server::{
        malformed_response, mock_mails, register_mock_account, uid_fetch_body_response,
        MockImapServer, MockImapServerHandle,
    };

    /// Two non-empty folders: the first downloads completely, the body fetch
    /// of the second fails.
    async fn first_folder_completes_second_breaks(
        account_id: u64,
    ) -> (AccountModel, [MailBox; 2], MockImapServerHandle) {
        let mails = mock_mails(&[1, 2]);
        let bodies: Vec<(u32, &[u8])> = mails.iter().map(|m| (m.uid, m.body.as_slice())).collect();
        let handle = MockImapServer::new()
            .respond_times("BODY.PEEK[]", uid_fetch_body_response(&bodies), 1)
            .respond("BODY.PEEK[]", malformed_response())
            .serving(&mails)
            .start()
            .await;
        let account = register_mock_account(&handle, account_id);
        DownloadState::init(account_id, TriggerType::Scheduled)
            .await
            .unwrap();
        let folder = |name: &str| MailBox {
            id: 1_000_000 + (uuid::Uuid::new_v4().as_u128() % 1_000_000_000) as u64,
            account_id,
            name: name.into(),
            exists: 2,
            uid_validity: Some(42),
            ..Default::default()
        };
        (account, [folder("INBOX"), folder("Archive")], handle)
    }

    fn stored_highest_uid(mailbox: &MailBox) -> Option<u32> {
        MailBox::get(mailbox.id).unwrap().highest_uid
    }

    #[tokio::test]
    async fn rebuild_persists_the_watermark_of_completed_folders_only() {
        let (account, folders, _server) = first_folder_completes_second_breaks(59_342_451).await;

        rebuild_cache(&account, &folders, CancellationToken::new())
            .await
            .unwrap();

        assert_eq!(stored_highest_uid(&folders[0]), Some(2));
        assert_eq!(stored_highest_uid(&folders[1]), None);
    }

    #[tokio::test]
    async fn date_rebuild_persists_the_watermark_of_completed_folders_only() {
        let (account, folders, _server) = first_folder_completes_second_breaks(59_342_452).await;

        rebuild_cache_by_date(
            &account,
            &folders,
            "01-Jan-2020",
            FetchDirection::Since,
            CancellationToken::new(),
        )
        .await
        .unwrap();

        assert_eq!(stored_highest_uid(&folders[0]), Some(2));
        assert_eq!(stored_highest_uid(&folders[1]), None);
    }
}
