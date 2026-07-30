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

use crate::account::migration::AccountType;
use crate::context::Initialize;
use crate::{
    {
        account::migration::AccountModel, context::controller::DOWNLOAD_CONTROLLER, error::MailboxdResult,
    },
    utc_now,
};
use std::sync::LazyLock;
use tracing::info;

pub static MAILBOXD_CONTEXT: LazyLock<MailboxdContext> = LazyLock::new(MailboxdContext::new);

pub struct MailboxdContext {
    start_at: i64,
}

impl Initialize for MailboxdContext {
    async fn initialize() -> MailboxdResult<()> {
        MAILBOXD_CONTEXT.start_account_downloader().await
    }
}

impl MailboxdContext {
    pub fn new() -> Self {
        Self {
            start_at: utc_now!(),
        }
    }
    pub fn uptime_ms(&self) -> i64 {
        utc_now!() - self.start_at
    }

    pub async fn start_account_downloader(&self) -> MailboxdResult<()> {
        let accounts = AccountModel::list_all()?;
        let active_accounts: Vec<AccountModel> = accounts
            .into_iter()
            .filter(|a| a.enabled && matches!(a.account_type, AccountType::IMAP))
            .collect();

        if active_accounts.is_empty() {
            info!("No active accounts found for account initialization.");
            return Ok(());
        }
        info!(
            "System has {} active IMAP accounts to initialize.",
            active_accounts.len()
        );
        for account in active_accounts {
            DOWNLOAD_CONTROLLER
                .trigger_schedule(account.id, account.email)
                .await
        }

        Ok(())
    }
}
