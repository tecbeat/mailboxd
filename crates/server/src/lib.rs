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


pub mod common;
pub mod error;
pub mod rest;

use std::sync::LazyLock;

use mailboxd_core::{
    mailboxd_version,
    cache::imap::task::SYNC_TASKS,
    common::{rustls::MailboxdTls, signal::SignalManager},
    context::{executors::MailboxdContext, Initialize},
    database::manager::DB_MANAGER,
    error::{code::ErrorCode, MailboxdResult},
    logger,
    migrate::check_data_status,
    raise_error,
    settings::{cli::SETTINGS, dir::DataDirManager},
    store::{
        blob::BLOB_MANAGER,
        tantivy::{attachment::ATTACHMENT_MANAGER, envelope::ENVELOPE_MANAGER},
    },
    tasks::PeriodicTasks,
    users::manager::UserManager,
};
use mailboxd_smtp::server::{start_smtp_server, SmtpServer};
use tracing::{error, info};

pub async fn run() -> MailboxdResult<()> {
    logger::initialize_logging();
    info!(
        r#"
                       _  _  _                       _
     _ __ ___    __ _ (_)| || |__    ___  __  __  __| |
    | '_ ` _ \  / _` || || || '_ \  / _ \ \ \/ / / _` |
    | | | | | || (_| || || || |_) || (_) | >  < | (_| |
    |_| |_| |_| \__,_||_||_||_.__/  \___/ /_/\_\ \__,_|

    "#
    );
    info!("Starting mailboxd-server");
    info!("Version:  {}", mailboxd_version!());
    info!("Git:      [{}]", env!("GIT_HASH"));

    match check_data_status() {
        Ok(false) => {
            error!("Incompatible data format detected.");
            error!("Please run: mailboxd-admin");
            return Err(raise_error!(
                "Legacy data layout detected".into(),
                ErrorCode::InternalError
            ));
        }
        Err(e) => {
            error!("Failed to check data layout: {:#?}", e);
            return Err(raise_error!(format!("{:#?}", e), ErrorCode::InternalError));
        }
        Ok(true) => {}
    }

    if let Err(error) = initialize().await {
        eprintln!("{:?}", error);
        return Err(error);
    }

    let periodic_tasks = PeriodicTasks::setup();
    let mut smtp_service: Option<SmtpServer> = None;
    if SETTINGS.mailboxd_enable_smtp {
        info!("SMTP service is enabled, starting...");
        match start_smtp_server().await {
            Ok(server) => {
                info!("SMTP server listening on: {}", server.smtp_addr);
                smtp_service = Some(server);
            }
            Err(e) => {
                error!("Failed to start SMTP server: {}", e);
                return Err(raise_error!(format!("{:#?}", e), ErrorCode::InternalError));
            }
        }
    } else {
        info!("SMTP service is disabled by configuration.");
    }

    rest::start_http_server().await?;
    periodic_tasks.shutdown().await;

    if let Some(server) = smtp_service {
        info!("Shutting down SMTP server...");
        server.stop().await;
        info!("SMTP server stopped.");
    }

    SYNC_TASKS.shutdown().await;
    ENVELOPE_MANAGER.shutdown().await;
    ATTACHMENT_MANAGER.shutdown().await;
    BLOB_MANAGER.shutdown().await;
    DB_MANAGER.flush();
    info!("mailboxd server stopped.");
    Ok(())
}

async fn initialize() -> MailboxdResult<()> {
    SignalManager::initialize().await?;
    DataDirManager::initialize().await?;
    UserManager::initialize().await?;
    MailboxdTls::initialize().await?;
    MailboxdContext::initialize().await?;
    LazyLock::force(&BLOB_MANAGER);
    LazyLock::force(&ENVELOPE_MANAGER);
    LazyLock::force(&ATTACHMENT_MANAGER);
    Ok(())
}
