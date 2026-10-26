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
use poem::{web::Redirect, IntoResponse, Response};

/// Prefix a WebUI path (`/sign-in?…`) with the configured base path, so
/// redirects keep working when mailboxd is served below a subpath.
pub fn ui_path(base_url: &str, path: &str) -> String {
    format!("{}{}", base_url.trim_end_matches('/'), path)
}

pub fn redirect_to_ui(path: &str) -> Response {
    Redirect::temporary(ui_path(&SETTINGS.mailboxd_base_url, path)).into_response()
}

#[cfg(test)]
mod tests {
    use super::ui_path;

    #[test]
    fn root_base_keeps_path_unchanged() {
        assert_eq!(
            ui_path("/", "/oauth2-result?success=true"),
            "/oauth2-result?success=true"
        );
    }

    #[test]
    fn subpath_base_is_prefixed() {
        assert_eq!(
            ui_path("/mail", "/oauth2-result?success=true"),
            "/mail/oauth2-result?success=true"
        );
    }

    #[test]
    fn subpath_base_with_trailing_slash_is_not_doubled() {
        assert_eq!(ui_path("/mail/", "/sign-in"), "/mail/sign-in");
    }

    #[test]
    fn nested_subpath_base_is_prefixed() {
        assert_eq!(
            ui_path("/apps/mail", "/sso-callback?handoff=x"),
            "/apps/mail/sso-callback?handoff=x"
        );
    }
}
