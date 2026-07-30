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

use mailboxd_core::settings::cli::SETTINGS;
use poem::{handler, web::Json, IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
struct OidcClientConfig {
    /// Whether OIDC login is fully configured on this server. The SPA uses
    /// this flag to decide whether to render the "Sign in with SSO" button.
    enabled: bool,
    /// When true and enabled=true, the sign-in page should redirect to the
    /// OIDC provider automatically. Users can bypass with ?local=1.
    auto_redirect: bool,
}

/// Public, no-auth endpoint. Returns just enough information for the
/// sign-in screen to decide how to render itself; nothing sensitive.
#[handler]
pub async fn get_oidc_config() -> impl IntoResponse {
    let enabled = SETTINGS.mailboxd_oidc_enabled
        && SETTINGS.mailboxd_oidc_issuer_url.is_some()
        && SETTINGS.mailboxd_oidc_client_id.is_some()
        && SETTINGS.mailboxd_oidc_client_secret.is_some()
        && SETTINGS.mailboxd_oidc_redirect_uri.is_some();

    Json(OidcClientConfig {
        enabled,
        auto_redirect: enabled && SETTINGS.mailboxd_oidc_auto_redirect,
    })
}
