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

use crate::{encrypt, error::MailboxdResult};

//use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ImapConfig {
    /// IMAP server hostname or IP address
    #[cfg_attr(
        feature = "web-api",
        oai(validator(max_length = 253, pattern = r"^[a-zA-Z0-9\-\.]+$"))
    )]
    pub host: String,
    /// IMAP server port number
    #[cfg_attr(
        feature = "web-api",
        oai(validator(minimum(value = "1"), maximum(value = "65535")))
    )]
    pub port: u16,
    /// Connection encryption method
    pub encryption: Encryption,
    /// Authentication configuration
    pub auth: AuthConfig,
    /// Optional proxy ID for establishing the connection.
    /// - If `None` or not provided, the client will connect directly to the IMAP server.
    /// - If `Some(proxy_id)`, the client will use the pre-configured proxy with the given ID.
    pub use_proxy: Option<u64>,
}

impl ImapConfig {
    pub fn try_encrypt_password(self) -> MailboxdResult<Self> {
        Ok(Self {
            host: self.host,
            port: self.port,
            encryption: self.encryption,
            auth: self.auth.encrypt()?,
            use_proxy: self.use_proxy,
        })
    }
}

#[derive(Default, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum AuthType {
    /// Standard password authentication (PLAIN/LOGIN)
    #[default]
    Password,
    /// OAuth 2.0 authentication (SASL XOAUTH2)
    OAuth2,
}

#[derive(Default, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct AuthConfig {
    ///Authentication method to use
    pub auth_type: AuthType,
    /// Credential secret for Password authentication.
    ///
    /// Users should provide a plaintext password (1 to 256 characters).
    /// The server will encrypt the password using AES-256-GCM and securely store it.
    #[cfg_attr(feature = "web-api", oai(validator(max_length = 256, min_length = 1)))]
    pub password: Option<String>,
}

impl AuthConfig {
    pub fn encrypt(self) -> MailboxdResult<Self> {
        match self.password {
            Some(password) => Ok(Self {
                auth_type: self.auth_type,
                password: Some(encrypt!(&password)?),
            }),
            None => Ok(self),
        }
    }
}

impl AuthConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        match self.auth_type {
            AuthType::Password if self.password.is_none() => {
                Err("When auth_type is Passwd, password must not be None.")
            }
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum Encryption {
    /// SSL/TLS encrypted connection
    #[default]
    Ssl,
    /// StartTLS encryption
    StartTls,
    /// Unencrypted connection
    None,
}

impl From<bool> for Encryption {
    fn from(value: bool) -> Self {
        if value {
            Self::Ssl
        } else {
            Self::None
        }
    }
}

// ---------------------------------------------------------------------------
// JMAP (RFC 8620 / RFC 8621) source configuration
//
// Model-only support (epic #44, issue #47): this defines how a JMAP account is
// configured and stored. No sync logic is wired to it yet (that lands in #49).
// Secrets (password / bearer token) are encrypted at rest with the same
// AES-256-GCM `encrypt!` path used by `ImapConfig`.
// ---------------------------------------------------------------------------

#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct JmapConfig {
    /// Explicit JMAP Session resource URL (e.g. `https://api.fastmail.com/jmap/session`).
    ///
    /// Optional: when omitted, the session URL is resolved from the account's
    /// email address via autodiscovery (`/.well-known/jmap`, handled by a later
    /// issue). At least one of `session_url` or a resolvable email must exist;
    /// this cross-field rule is enforced in the payload validation.
    #[cfg_attr(feature = "web-api", oai(validator(max_length = 2048)))]
    pub session_url: Option<String>,
    /// Authentication configuration for the JMAP endpoint.
    pub auth: JmapAuthConfig,
    /// Optional proxy ID for establishing the connection.
    /// - If `None`, the client connects directly to the JMAP server.
    /// - If `Some(proxy_id)`, the pre-configured proxy with the given ID is used.
    pub use_proxy: Option<u64>,
}

impl JmapConfig {
    /// Encrypt the embedded secret (password / bearer token) for storage.
    pub fn try_encrypt_secret(self) -> MailboxdResult<Self> {
        Ok(Self {
            session_url: self.session_url,
            auth: self.auth.encrypt()?,
            use_proxy: self.use_proxy,
        })
    }
}

#[derive(Default, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum JmapAuthType {
    /// HTTP Basic authentication (username + password).
    #[default]
    Basic,
    /// Bearer / API token (e.g. a Fastmail app-specific token).
    Bearer,
    /// OAuth 2.0, reusing the existing OAuth2 infrastructure.
    OAuth2,
}

#[derive(Default, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct JmapAuthConfig {
    /// Authentication method to use.
    pub auth_type: JmapAuthType,
    /// Login/username for `Basic` authentication. Ignored for other methods.
    #[cfg_attr(feature = "web-api", oai(validator(max_length = 320, min_length = 1)))]
    pub username: Option<String>,
    /// Secret for `Basic` (password) or `Bearer` (API token) authentication.
    ///
    /// Users provide a plaintext value (1 to 4096 characters). The server
    /// encrypts it with AES-256-GCM before storing. Not used for `OAuth2`,
    /// where tokens are managed by the existing OAuth2 subsystem.
    #[cfg_attr(feature = "web-api", oai(validator(max_length = 4096, min_length = 1)))]
    pub secret: Option<String>,
}

impl JmapAuthConfig {
    pub fn encrypt(self) -> MailboxdResult<Self> {
        match self.secret {
            Some(secret) => Ok(Self {
                auth_type: self.auth_type,
                username: self.username,
                secret: Some(encrypt!(&secret)?),
            }),
            None => Ok(self),
        }
    }

    /// Validate that the credentials required for the chosen auth method are present.
    ///
    /// - `Basic`  → username and secret (password) required.
    /// - `Bearer` → secret (token) required.
    /// - `OAuth2` → no inline secret; tokens come from the OAuth2 subsystem.
    pub fn validate(&self) -> Result<(), &'static str> {
        match self.auth_type {
            JmapAuthType::Basic => {
                if self.username.is_none() {
                    return Err("When auth_type is Basic, username must not be None.");
                }
                if self.secret.is_none() {
                    return Err("When auth_type is Basic, secret (password) must not be None.");
                }
                Ok(())
            }
            JmapAuthType::Bearer => {
                if self.secret.is_none() {
                    return Err("When auth_type is Bearer, secret (API token) must not be None.");
                }
                Ok(())
            }
            JmapAuthType::OAuth2 => Ok(()),
        }
    }
}
