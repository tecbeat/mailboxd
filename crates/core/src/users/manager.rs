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

use crate::{
    context::Initialize,
    error::MailboxdResult,
    settings::cli::SETTINGS,
    token::AccessTokenModel,
    users::{role::UserRole, UserModel},
};
use tracing::{info, warn};

pub struct UserManager;

impl Initialize for UserManager {
    async fn initialize() -> MailboxdResult<()> {
        UserRole::ensure_default_roles_exists()?;
        UserModel::ensure_default_admin_exists()?;
        if !SETTINGS.mailboxd_local_login_enabled {
            enforce_sso_only()?;
        }
        Ok(())
    }
}

/// Ends the password sessions that are still open and warns when no SSO
/// user could administer the server.
fn enforce_sso_only() -> MailboxdResult<()> {
    let revoked = AccessTokenModel::revoke_local_webui_tokens()?;
    info!("Local login is disabled; revoked {revoked} WebUI session(s) of users without SSO");
    if !has_sso_admin(&UserModel::list_all()?) {
        warn!(
            "Local login is disabled but no SSO user has administrator rights yet; \
             grant the admin role to an SSO user or set MAILBOXD_OIDC_DEFAULT_ROLE_ID"
        );
    }
    Ok(())
}

fn has_sso_admin(users: &[UserModel]) -> bool {
    users
        .iter()
        .any(|u| u.sso_provider.is_some() && u.is_admin())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::users::role::DEFAULT_ADMIN_ROLE_ID;

    async fn init_users() {
        static INIT: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
        INIT.get_or_init(|| async { UserManager::initialize().await.unwrap() })
            .await;
    }

    fn user(sso: bool, admin: bool) -> UserModel {
        UserModel {
            sso_provider: sso.then(|| "oidc".into()),
            global_roles: if admin {
                vec![DEFAULT_ADMIN_ROLE_ID]
            } else {
                vec![]
            },
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn an_sso_user_with_the_admin_role_counts_as_sso_admin() {
        init_users().await;
        assert!(has_sso_admin(&[user(false, true), user(true, true)]));
    }

    #[tokio::test]
    async fn local_admins_and_sso_users_without_admin_role_do_not_count() {
        init_users().await;
        assert!(!has_sso_admin(&[user(false, true), user(true, false)]));
        assert!(!has_sso_admin(&[]));
    }
}
