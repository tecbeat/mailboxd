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

use crate::common::auth::WrappedContext;
use crate::rest::api::ApiTags;
use crate::rest::ApiResult;
use futures::StreamExt;
use mailboxd_core::backup::{ops, restore, BackupConfig};
use mailboxd_core::common::signal::SIGNAL_MANAGER;
use mailboxd_core::error::code::ErrorCode;
use mailboxd_core::ext::event_bus::{emit, Event};
use mailboxd_core::raise_error;
use mailboxd_core::settings::{cli::SETTINGS, dir::DATA_DIR_MANAGER};
use mailboxd_core::users::permissions::Permission;
use poem::Body;
use poem_openapi::payload::{Attachment, AttachmentType, Binary, Json};
use poem_openapi::{Object, OpenApi};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;

/// Seconds to wait after acknowledging a staged restore before restarting, so
/// the HTTP response reaches the client first.
const RESTART_DELAY: Duration = Duration::from_secs(2);

/// Result of staging a restore. The server restarts shortly after returning.
#[derive(Debug, Object)]
pub struct RestoreStagedResponse {
    /// True when the upload was validated and staged successfully.
    pub staged: bool,
    /// Human-readable status message for the UI.
    pub message: String,
}

pub struct BackupApi;

#[OpenApi(prefix_path = "/api/v1", tag = "ApiTags::Backup")]
impl BackupApi {
    /// Report the backup configuration and current scheduled-backup state.
    ///
    /// Read-only and restricted to administrators.
    #[oai(method = "get", path = "/backup-config", operation_id = "get_backup_config")]
    async fn get_backup_config(&self, context: WrappedContext) -> ApiResult<Json<BackupConfig>> {
        context.require_permission(None, Permission::ROOT)?;
        Ok(Json(ops::config_snapshot()))
    }

    /// Create a full instance backup and stream it to the client for download.
    ///
    /// Restricted to administrators. The archive is written to a temp file,
    /// unlinked immediately, and streamed from the still-open handle so it
    /// never lingers on disk after the download completes.
    #[oai(method = "post", path = "/create-backup", operation_id = "create_backup")]
    async fn create_backup(&self, context: WrappedContext) -> ApiResult<Attachment<Body>> {
        context.require_permission(None, Permission::ROOT)?;

        let manual = ops::create_manual_backup().await?;
        let file = tokio::fs::File::open(&manual.path).await.map_err(|e| {
            raise_error!(format!("open backup archive: {e}"), ErrorCode::InternalError)
        })?;
        // Unlink now; the open handle keeps the bytes readable until the stream
        // is fully consumed, so the temp file cannot leak.
        let _ = tokio::fs::remove_file(&manual.path).await;

        emit(Event::BackupCreated {
            user: context.user.username.clone(),
        });

        let attachment = Attachment::new(Body::from_async_read(file))
            .attachment_type(AttachmentType::Attachment)
            .filename(manual.filename);
        Ok(attachment)
    }

    /// Upload a backup archive and stage it for restore-on-restart.
    ///
    /// Restricted to administrators. The archive is validated, staged, and the
    /// server then restarts to apply it before any data handle is opened. The
    /// archive is only usable with the same `MAILBOXD_ENCRYPT_PASSWORD` that
    /// produced it.
    #[oai(method = "post", path = "/restore-backup", operation_id = "restore_backup")]
    async fn restore_backup(
        &self,
        /// The raw backup archive bytes (`.tar.zst`).
        data: Binary<Body>,
        context: WrappedContext,
    ) -> ApiResult<Json<RestoreStagedResponse>> {
        context.require_permission(None, Permission::ROOT)?;

        let max_bytes = SETTINGS.mailboxd_web_pst_upload_limit_mb as usize * 1024 * 1024;
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_path = DATA_DIR_MANAGER
            .temp_dir
            .join(format!("restore-upload-{unique:x}.tar.zst"));

        tokio::fs::create_dir_all(&DATA_DIR_MANAGER.temp_dir)
            .await
            .map_err(|e| {
                raise_error!(format!("create temp directory: {e}"), ErrorCode::InternalError)
            })?;

        stream_upload_to_temp(data.0, &temp_path, max_bytes).await?;

        // Validate and stage; on success the temp file is moved into staging.
        if let Err(e) = restore::stage_restore_from_file(&temp_path) {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(e.into());
        }

        emit(Event::BackupRestoreStaged {
            user: context.user.username.clone(),
        });

        // Restart shortly so the staged archive is applied on the next boot.
        tokio::spawn(async move {
            tokio::time::sleep(RESTART_DELAY).await;
            tracing::warn!("Restarting to apply staged restore");
            SIGNAL_MANAGER.trigger_shutdown();
        });

        Ok(Json(RestoreStagedResponse {
            staged: true,
            message: "Backup staged. The server is restarting to apply the restore.".to_string(),
        }))
    }
}

/// Stream a request body to `temp_path`, enforcing `max_bytes`. Removes the
/// partial file and errors on overflow or an empty upload.
async fn stream_upload_to_temp(
    body: Body,
    temp_path: &std::path::Path,
    max_bytes: usize,
) -> ApiResult<u64> {
    let mut file = tokio::fs::File::create(temp_path).await.map_err(|e| {
        raise_error!(format!("create temp file: {e}"), ErrorCode::InternalError)
    })?;

    let mut stream = body.into_bytes_stream();
    let mut total: usize = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            raise_error!(format!("read request body: {e}"), ErrorCode::InternalError)
        })?;
        total += chunk.len();
        if total > max_bytes {
            drop(file);
            let _ = tokio::fs::remove_file(temp_path).await;
            let max_mb = max_bytes as f64 / 1024.0 / 1024.0;
            return Err(raise_error!(
                format!("Upload exceeds maximum size of {max_mb:.0} MB."),
                ErrorCode::PayloadTooLarge
            )
            .into());
        }
        file.write_all(&chunk).await.map_err(|e| {
            raise_error!(format!("write temp file: {e}"), ErrorCode::InternalError)
        })?;
    }

    file.flush().await.map_err(|e| {
        raise_error!(format!("flush temp file: {e}"), ErrorCode::InternalError)
    })?;

    if total == 0 {
        let _ = tokio::fs::remove_file(temp_path).await;
        return Err(raise_error!(
            "Empty upload is not allowed.".into(),
            ErrorCode::InvalidParameter
        )
        .into());
    }

    Ok(total as u64)
}
