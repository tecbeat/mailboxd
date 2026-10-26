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
pub mod startup;

use std::path::PathBuf;
use std::sync::LazyLock;

use mailboxd_core::{
    archive::imap::task::SYNC_TASKS,
    common::{rustls::MailboxdTls, signal::SignalManager},
    context::{executors::MailboxdContext, Initialize},
    database::manager::DB_MANAGER,
    error::{code::ErrorCode, MailboxdResult},
    logger, mailboxd_version,
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
use mailboxd_migrate::{reindex::reindex, run_auto_migrate, AutoMigratePaths};
use mailboxd_smtp::server::{start_smtp_server, SmtpServer};
use rest::maintenance::serve_maintenance;
use startup::{prepare_storage, StorageSteps};
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

    // Restore, migrate, check and reindex the data volume before any
    // database, blob or index handle is opened. When a step fails, stay up
    // with a maintenance page instead of exiting into a restart loop.
    let migrate_paths = AutoMigratePaths {
        root_dir: PathBuf::from(&SETTINGS.mailboxd_root_dir),
        index_dir: SETTINGS.mailboxd_index_dir.as_ref().map(PathBuf::from),
        data_dir: SETTINGS.mailboxd_data_dir.as_ref().map(PathBuf::from),
    };
    if let Some(maintenance) = prepare_storage(StorageSteps {
        restore: mailboxd_core::backup::restore::apply_pending_restore,
        migrate: || run_auto_migrate(&migrate_paths),
        check_layout: check_data_status,
        reindex: || reindex(&migrate_paths, false),
    }) {
        serve_maintenance(maintenance).await;
        return Ok(());
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
    mailboxd_core::audit::install();
    mailboxd_core::ext::default_extractor::install();
    LazyLock::force(&BLOB_MANAGER);
    LazyLock::force(&ENVELOPE_MANAGER);
    LazyLock::force(&ATTACHMENT_MANAGER);
    Ok(())
}

#[cfg(test)]
mod tests;
