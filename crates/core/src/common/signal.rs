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

use std::sync::LazyLock;

use crate::{context::Initialize, error::MailboxdResult, utils::shutdown::shutdown_signal};
use tokio::sync::broadcast;

pub static SIGNAL_MANAGER: LazyLock<SignalManager> = LazyLock::new(SignalManager::new);

pub struct SignalManager {
    sender: broadcast::Sender<()>,
}

impl SignalManager {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1);
        SignalManager { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.sender.subscribe()
    }

    /// Broadcast a shutdown signal, triggering the same graceful shutdown as a
    /// SIGTERM/Ctrl-C. Used to restart the process after staging a restore so
    /// the staged archive is applied on the next boot (the container's
    /// `restart: unless-stopped` policy relaunches it).
    pub fn trigger_shutdown(&self) {
        let _ = self.sender.send(());
    }
}

impl Initialize for SignalManager {
    async fn initialize() -> MailboxdResult<()> {
        tokio::spawn({
            async move {
                shutdown_signal().await;
                println!("\nSending shutdown signal...");
                let _ = SIGNAL_MANAGER.sender.send(());
            }
        });
        Ok(())
    }
}
