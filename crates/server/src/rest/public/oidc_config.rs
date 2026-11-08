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
    /// Whether the username/password form is offered at all.
    local_login: bool,
}

/// Public, no-auth endpoint. Returns just enough information for the
/// sign-in screen to decide how to render itself; nothing sensitive.
#[handler]
pub async fn get_oidc_config() -> impl IntoResponse {
    let enabled = SETTINGS.oidc_configured();

    Json(OidcClientConfig {
        enabled,
        auto_redirect: enabled && SETTINGS.mailboxd_oidc_auto_redirect,
        local_login: SETTINGS.mailboxd_local_login_enabled,
    })
}
