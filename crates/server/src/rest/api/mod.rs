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

use access_token::AccessTokenApi;
use account::AccountApi;
use audit::AuditApi;
use auto_config::AutoConfigApi;
use mailboxd_core::mailboxd_version;
use mailbox::MailBoxApi;
use message::MessageApi;
use oauth2::OAuth2Api;
use poem_openapi::{OpenApiService, Tags};
use crate::rest::api::{attachment::AttachmentApi, import::ImportApi, users::UsersApi};
use system::SystemApi;

pub mod access_token;
pub mod account;
pub mod audit;
pub mod attachment;
pub mod auto_config;
pub mod import;
pub mod mailbox;
pub mod message;
pub mod oauth2;
pub mod system;
pub mod users;

#[derive(Tags)]
pub enum ApiTags {
    AccessToken,
    Attachment,
    Audit,
    AutoConfig,
    Account,
    Mailbox,
    OAuth2,
    Message,
    System,
    Import,
    Users,
}

type RustMailOpenApi = (
    AccessTokenApi,
    AttachmentApi,
    AuditApi,
    AutoConfigApi,
    AccountApi,
    SystemApi,
    MailBoxApi,
    OAuth2Api,
    MessageApi,
    ImportApi,
    UsersApi,
);

pub fn create_openapi_service() -> OpenApiService<RustMailOpenApi, ()> {
    OpenApiService::new(
        (
            AccessTokenApi,
            AttachmentApi,
            AuditApi,
            AutoConfigApi,
            AccountApi,
            SystemApi,
            MailBoxApi,
            OAuth2Api,
            MessageApi,
            ImportApi,
            UsersApi,
        ),
        "MailboxdApi",
        mailboxd_version!(),
    )
}
