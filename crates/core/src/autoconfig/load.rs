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

use crate::account::entity::Encryption;
use crate::autoconfig::client::{self, MailConfig};
use crate::autoconfig::entity::{JmapServerConfig, MailServerConfig, ServerConfig};
use crate::autoconfig::oauth2_providers::lookup_oauth2;
use crate::autoconfig::CachedMailSettings;
use crate::error::code::ErrorCode;
use crate::error::MailboxdResult;
use crate::raise_error;
use email_address::EmailAddress;
use std::str::FromStr;
use tracing::error;

/// Map an autoconfig XML `socketType` value to our `Encryption` enum.
pub(crate) fn socket_type_to_encryption(raw: &str) -> Encryption {
    match raw.to_ascii_uppercase().as_str() {
        "SSL" | "TLS" => Encryption::Ssl,
        "STARTTLS" => Encryption::StartTls,
        _ => Encryption::None,
    }
}

/// Convert the raw `MailConfig` discovered by `client::fetch` into a
/// `MailServerConfig` suitable for account provisioning.
pub(crate) fn mail_config_to_server_config(config: &MailConfig) -> Option<MailServerConfig> {
    let imap = config.incoming.iter().find(|s| {
        let p = s.protocol.to_ascii_lowercase();
        p == "imap" || p == "imaps"
    })?;

    let encryption = socket_type_to_encryption(&imap.socket_type);
    let port = if imap.port != 0 {
        imap.port
    } else {
        match encryption {
            Encryption::Ssl => 993,
            _ => 143,
        }
    };

    // Detect OAuth2 support: the XML <authentication> field and a known
    // hostname → issuer mapping determine whether the provider supports OAuth2.
    let oauth2 = if imap.authentication.eq_ignore_ascii_case("OAuth2") {
        lookup_oauth2(&imap.hostname)
    } else {
        None
    };

    Some(MailServerConfig {
        imap: ServerConfig::new(imap.hostname.clone(), port, encryption),
        oauth2,
        jmap: None,
    })
}

pub async fn resolve_autoconfig(email: impl AsRef<str>) -> MailboxdResult<Option<MailServerConfig>> {
    let email = email.as_ref();
    let email_address = EmailAddress::from_str(email).map_err(|error| {
        raise_error!(
            format!("Invalid email address: {email:#?}. {error:#?}"),
            ErrorCode::InvalidParameter
        )
    })?;

    let domain = email_address.domain();
    // Try local cache first
    if let Some(cached_entity) = CachedMailSettings::get(domain)? {
        return Ok(Some(cached_entity.config));
    }

    // Probe JMAP autodiscovery (`/.well-known/jmap`) in parallel with the
    // IMAP-oriented cascade. A JMAP-only domain is a valid result (FA-2).
    let jmap = client::fetch_jmap_session_url(domain)
        .await
        .map(|session_url| JmapServerConfig {
            session_url,
            oauth2: None,
        });

    let imap_config = match client::fetch(domain).await {
        Ok(config) => mail_config_to_server_config(&config),
        Err(e) => {
            // An IMAP miss is not fatal when JMAP was found; only log it.
            error!(
                email = %email,
                domain = %domain,
                error = ?e,
                "IMAP autoconfig fetch failed"
            );
            None
        }
    };

    // Combine: carry IMAP (+ its OAuth2) when present, attach JMAP when present.
    let result = match (imap_config, jmap) {
        (Some(mut cfg), jmap) => {
            cfg.jmap = jmap;
            cfg
        }
        (None, Some(jmap)) => MailServerConfig {
            imap: ServerConfig::default(),
            oauth2: None,
            jmap: Some(jmap),
        },
        (None, None) => {
            return Err(raise_error!(
                format!(
                    "No IMAP or JMAP server found in autoconfig for email: {}",
                    email_address.email()
                ),
                ErrorCode::ResourceNotFound
            ));
        }
    };

    CachedMailSettings::add(domain.into(), result.clone())?;
    Ok(Some(result))
}
