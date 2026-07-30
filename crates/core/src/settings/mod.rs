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

use crate::settings::cli::Settings;
//use poem_openapi::Object;
use serde::{Deserialize, Serialize};

pub mod cli;
pub mod dir;
pub mod io;
pub mod proxy;
pub mod system;
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct SystemConfigurations {
    pub mailboxd_log_level: String,
    pub mailboxd_http_port: i32,
    pub mailboxd_bind_ip: Option<String>,
    pub mailboxd_public_url: String,

    pub mailboxd_cors_origins: Option<Vec<String>>,
    pub mailboxd_cors_max_age: i32,

    pub mailboxd_ansi_logs: bool,
    pub mailboxd_log_to_file: bool,
    pub mailboxd_json_logs: bool,
    pub mailboxd_max_server_log_files: usize,

    pub mailboxd_encrypt_password_set: bool,
    pub mailboxd_webui_token_expiration_hours: u32,

    pub mailboxd_root_dir: String,

    pub mailboxd_enable_rest_https: bool,
    pub mailboxd_http_compression_enabled: bool,
    pub mailboxd_sync_concurrency: Option<u16>,

    pub mailboxd_base_url: String,
    pub mailboxd_index_dir: Option<String>,
    pub mailboxd_data_dir: Option<String>,

    pub mailboxd_enable_smtp: bool,
    pub mailboxd_smtp_port: u16,
    pub mailboxd_smtp_encryption: String,
    pub mailboxd_smtp_auth_required: bool,
    pub mailboxd_smtp_tls_key_path: Option<String>,
    pub mailboxd_smtp_tls_cert_path: Option<String>,

    pub mailboxd_oidc_enabled: bool,
    pub mailboxd_oidc_issuer_url: Option<String>,
    pub mailboxd_oidc_client_id: Option<String>,
    pub mailboxd_oidc_redirect_uri: Option<String>,
    pub mailboxd_oidc_default_role_id: u64,
    pub mailboxd_oidc_auto_redirect: bool,

    pub mailboxd_upload_body_limit_mb: u64,

    pub mailboxd_web_mbox_upload_limit_mb: u64,

    pub mailboxd_web_pst_upload_limit_mb: u64,
}

impl From<&Settings> for SystemConfigurations {
    fn from(s: &Settings) -> Self {
        Self {
            mailboxd_log_level: s.mailboxd_log_level.clone(),
            mailboxd_http_port: s.mailboxd_http_port,
            mailboxd_bind_ip: s.mailboxd_bind_ip.clone(),
            mailboxd_public_url: s.mailboxd_public_url.clone(),
            mailboxd_cors_origins: s
                .mailboxd_cors_origins
                .as_ref()
                .map(|set| set.iter().cloned().collect()),
            mailboxd_cors_max_age: s.mailboxd_cors_max_age,
            mailboxd_ansi_logs: s.mailboxd_ansi_logs,
            mailboxd_log_to_file: s.mailboxd_log_to_file,
            mailboxd_json_logs: s.mailboxd_json_logs,
            mailboxd_max_server_log_files: s.mailboxd_max_server_log_files,
            mailboxd_encrypt_password_set: s.mailboxd_encrypt_password.is_some()
                || s.mailboxd_encrypt_password_file.is_some(),
            mailboxd_webui_token_expiration_hours: s.mailboxd_webui_token_expiration_hours,
            mailboxd_root_dir: s.mailboxd_root_dir.clone(),
            mailboxd_enable_rest_https: s.mailboxd_enable_rest_https,
            mailboxd_http_compression_enabled: s.mailboxd_http_compression_enabled,
            mailboxd_sync_concurrency: s.mailboxd_sync_concurrency,
            mailboxd_base_url: s.mailboxd_base_url.clone(),
            mailboxd_index_dir: s.mailboxd_index_dir.clone(),
            mailboxd_data_dir: s.mailboxd_data_dir.clone(),
            mailboxd_enable_smtp: s.mailboxd_enable_smtp,
            mailboxd_smtp_port: s.mailboxd_smtp_port,
            mailboxd_smtp_encryption: s.mailboxd_smtp_encryption.to_string(),
            mailboxd_smtp_auth_required: s.mailboxd_smtp_auth_required,
            mailboxd_smtp_tls_key_path: s.mailboxd_tls_key_path.clone(),
            mailboxd_smtp_tls_cert_path: s.mailboxd_tls_cert_path.clone(),
            mailboxd_oidc_enabled: s.mailboxd_oidc_enabled,
            mailboxd_oidc_issuer_url: s.mailboxd_oidc_issuer_url.clone(),
            mailboxd_oidc_client_id: s.mailboxd_oidc_client_id.clone(),
            mailboxd_oidc_redirect_uri: s.mailboxd_oidc_redirect_uri.clone(),
            mailboxd_oidc_default_role_id: s.mailboxd_oidc_default_role_id,
            mailboxd_oidc_auto_redirect: s.mailboxd_oidc_auto_redirect,
            mailboxd_upload_body_limit_mb: s.mailboxd_upload_body_limit_mb,
            mailboxd_web_mbox_upload_limit_mb: s.mailboxd_web_mbox_upload_limit_mb,
            mailboxd_web_pst_upload_limit_mb: s.mailboxd_web_pst_upload_limit_mb,
        }
    }
}
