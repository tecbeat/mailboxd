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
    database::{
        delete_impl, filter_impl, find_impl, list_all_impl, manager::DB_MANAGER, update_impl,
        with_transaction, MemDbModel,
    },
    decrypt,
    error::{code::ErrorCode, MailboxdResult},
    generate_token, id, raise_error,
    token::{AccessTokenModel, TokenType},
    users::{
        acl::AccessControl,
        mfa::{MfaChallenge, MfaState},
        payload::{UserCreateRequest, UserUpdateRequest},
        permissions::Permission,
        role::{UserRole, DEFAULT_ADMIN_ROLE_ID},
        view::UserView,
    },
    utc_now,
};
use argon2::{
    password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use subtle::ConstantTimeEq;
use tracing::warn;

pub mod acl;
pub mod manager;
pub mod mfa;
pub mod minimal;
pub mod payload;
pub mod permissions;
pub mod role;
pub mod view;

pub type UserModel = MailboxdUserV2;

fn derive_username_base(email: &str, display_name: Option<&str>) -> String {
    if let Some(name) = display_name.map(str::trim).filter(|s| !s.is_empty()) {
        let sanitized: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if !sanitized.is_empty() {
            return sanitized;
        }
    }
    email
        .split('@')
        .next()
        .unwrap_or("user")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn allocate_unique_username(base: &str) -> MailboxdResult<String> {
    let base = if base.is_empty() { "user" } else { base };
    let base_owned = base.to_string();
    let candidate_taken = |candidate: &str| -> MailboxdResult<bool> {
        let c = candidate.to_string();
        let existing = filter_impl::<UserModel, _>(DB_MANAGER.db(), move |u| u.username == c)?;
        Ok(existing.into_iter().next().is_some())
    };
    if !candidate_taken(&base_owned)? {
        return Ok(base_owned);
    }
    for i in 2..1000 {
        let candidate = format!("{}{}", base_owned, i);
        if !candidate_taken(&candidate)? {
            return Ok(candidate);
        }
    }
    Err(raise_error!(
        "Failed to allocate a unique username for auto-provisioned OIDC user".into(),
        ErrorCode::InternalError
    ))
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoginResult {
    pub success: bool,
    pub error_message: Option<String>,
    pub access_token: Option<String>,
    pub theme: Option<String>,
    pub language: Option<String>,
    /// The password was right; the login continues with a second factor
    /// against `mfa_challenge`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mfa_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mfa_challenge: Option<String>,
    /// Machine-readable reason a second login step failed, one of the
    /// `MFA_ERROR_*` constants in [`mfa`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

pub const DEFAULT_ADMIN_USER_ID: u64 = 100000000000000;

/// The password the default administrator is created with. Admins are
/// reminded to change it for as long as it still works.
pub const DEFAULT_ADMIN_PASSWORD: &str = "admin@mailboxd";

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MailboxdUserV2 {
    pub id: u64,
    pub username: String,
    pub email: String,

    pub password: Option<String>,

    /// Scoped Access: Defines per-account permissions.
    /// Example:
    /// { account_id: 1, role_id: role_manager_id } -> Manager on Account 1
    /// { account_id: 2, role_id: role_viewer_id }  -> Viewer on Account 2
    pub account_access_map: BTreeMap<u64, u64>,

    pub description: Option<String>,

    /// System Roles: Permissions that apply to the whole system
    /// (e.g., system settings, creating new users).
    pub global_roles: Vec<u64>,

    pub avatar: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    /// Optional access control settings
    pub acl: Option<AccessControl>,

    pub theme: Option<String>,
    pub language: Option<String>,

    /// SSO identity: unique subject ID from the external IdP (e.g. OIDC `sub` claim).
    pub sso_id: Option<String>,
    /// SSO provider identifier: `"oidc"` or future `"saml"` / `"ldap"`.
    pub sso_provider: Option<String>,

    /// Two-factor authentication; absent in records from before #76.
    #[serde(default)]
    pub mfa: MfaState,
}

impl MemDbModel for MailboxdUserV2 {
    fn collection() -> &'static str {
        "users"
    }
    fn key(&self) -> String {
        self.id.to_string()
    }
}

/// A local password must never be set for an SSO-managed user; their
/// credentials are owned by the external identity provider.
fn ensure_password_change_allowed(
    sso_provider: Option<&str>,
    password_requested: bool,
) -> MailboxdResult<()> {
    if password_requested && sso_provider.is_some() {
        return Err(raise_error!(
            "Cannot set a local password for an SSO-managed user; credentials are managed by the identity provider.".to_string(),
            ErrorCode::Forbidden
        ));
    }
    Ok(())
}

/// Apply an update request onto an existing user, enforcing the SSO
/// password-change guard. Pure transform (no persistence) so it can be
/// exercised directly in tests without a live database.
fn apply_user_update(current: &UserModel, request: UserUpdateRequest) -> MailboxdResult<UserModel> {
    ensure_password_change_allowed(current.sso_provider.as_deref(), request.password.is_some())?;

    let mut updated = current.clone();
    if let Some(username) = request.username {
        updated.username = username;
    }
    if let Some(email) = request.email {
        updated.email = email;
    }
    if let Some(desc) = request.description {
        updated.description = Some(desc);
    }
    if let Some(password) = request.password {
        updated.password = Some(hash_login_password(&password)?);
    }
    if let Some(global_roles) = request.global_roles {
        updated.global_roles = global_roles;
    }
    if let Some(acl) = request.acl {
        updated.acl = Some(acl);
    }
    if let Some(account_access_map) = request.account_access_map {
        updated.account_access_map = account_access_map;
    }
    if let Some(avatar_base64) = request.avatar_base64 {
        updated.avatar = Some(avatar_base64);
    }
    if let Some(theme) = request.theme {
        updated.theme = Some(theme);
    }
    if let Some(language) = request.language {
        updated.language = Some(language);
    }

    updated.updated_at = utc_now!();

    Ok(updated)
}

impl MailboxdUserV2 {
    pub fn is_using_role(&self, role_id: u64) -> bool {
        if self.global_roles.contains(&role_id) {
            return true;
        }

        if self.account_access_map.values().any(|&id| id == role_id) {
            return true;
        }
        false
    }

    pub fn list_all() -> MailboxdResult<Vec<UserModel>> {
        Ok(list_all_impl::<UserModel>(DB_MANAGER.db())?)
    }

    fn get_all_permissions(&self) -> HashSet<String> {
        let mut all_perms = HashSet::new();

        for &role_id in &self.global_roles {
            if let Ok(Some(role)) = UserRole::find(role_id) {
                for perm in role.permissions {
                    all_perms.insert(perm);
                }
            }
        }

        all_perms
    }

    pub fn to_view(self, role_lookup: &BTreeMap<u64, UserRole>) -> UserView {
        let global_roles_names = self
            .global_roles
            .iter()
            .filter_map(|role_id| role_lookup.get(role_id))
            .map(|role| role.name.clone())
            .collect();

        let account_roles_summary = self
            .account_access_map
            .iter()
            .map(|(acc_id, role_id)| {
                let role_name = role_lookup
                    .get(role_id)
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| "Unknown Role".to_string());
                (*acc_id, role_name)
            })
            .collect();

        let global_permissions = {
            let mut perms = BTreeSet::new();

            for role_id in &self.global_roles {
                if let Some(role) = role_lookup.get(role_id) {
                    perms.extend(role.permissions.iter().cloned());
                }
            }

            perms.into_iter().collect()
        };

        let account_permissions = {
            let mut map: BTreeMap<u64, BTreeSet<String>> = BTreeMap::new();

            for (account_id, role_id) in &self.account_access_map {
                if let Some(role) = role_lookup.get(role_id) {
                    let entry = map.entry(*account_id).or_default();
                    entry.extend(role.permissions.iter().cloned());
                }
            }

            map.into_iter()
                .map(|(acc_id, perms)| (acc_id, perms.into_iter().collect()))
                .collect()
        };
        UserView {
            id: self.id,
            username: self.username,
            email: self.email,
            password: self.password.map(|_| "************".to_string()),
            account_access_map: self.account_access_map,
            account_roles_summary,
            description: self.description,
            global_roles: self.global_roles,
            global_roles_names,
            avatar: self.avatar,
            created_at: self.created_at,
            updated_at: self.updated_at,
            acl: self.acl,
            account_permissions,
            global_permissions,
            theme: self.theme,
            language: self.language,
            sso_id: self.sso_id,
            sso_provider: self.sso_provider,
            totp_enabled: self.mfa.totp_enabled,
        }
    }

    pub fn is_admin(&self) -> bool {
        self.get_all_permissions().contains(Permission::ROOT)
    }

    pub fn ensure_default_admin_exists() -> MailboxdResult<()> {
        let now = utc_now!();

        // 1. Try to get the existing admin user
        let admin = find_impl::<UserModel>(DB_MANAGER.db(), &DEFAULT_ADMIN_USER_ID.to_string())?;

        if admin.is_none() {
            // 2. Insert the MailboxdUser with the updated schema
            let user = UserModel {
                id: DEFAULT_ADMIN_USER_ID,
                username: "admin".into(),
                email: "placeholder@example.com".into(),
                password: Some(hash_login_password(DEFAULT_ADMIN_PASSWORD)?),

                // Use global_roles as defined in our new schema
                global_roles: vec![DEFAULT_ADMIN_ROLE_ID],

                // Admin usually doesn't need specific scoped access
                account_access_map: BTreeMap::new(),

                avatar: None,
                created_at: now,
                updated_at: now,
                description: Some("System default administrator".into()),
                acl: None,
                theme: None,
                language: None,
                sso_id: None,
                sso_provider: None,
                mfa: MfaState::default(),
            };

            // 3. Generate and insert an initial access token for the first-time setup
            let access_token = AccessTokenModel {
                token: generate_token!(128),
                created_at: now,
                updated_at: now,
                last_access_at: Default::default(),
                name: Some("Initial Setup Token".into()),
                user_id: DEFAULT_ADMIN_USER_ID,
                token_type: TokenType::WebUI,
                expire_at: None, // Admin setup token usually persistent until changed
            };

            with_transaction(DB_MANAGER.db(), move |txn| {
                let txn = txn
                    .insert("users", DEFAULT_ADMIN_USER_ID.to_string(), &user)
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
                    .upsert("tokens", access_token.token.clone(), &access_token)
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
                Ok(txn)
            })?;
        }

        Ok(())
    }

    pub fn authenticate_user(username: String, password: String) -> MailboxdResult<LoginResult> {
        // Find by username
        let username_for_first = username.clone();
        let users = filter_impl::<UserModel, _>(DB_MANAGER.db(), move |u| {
            u.username == username_for_first
        })?;
        let user = match users.into_iter().next() {
            Some(u) => u,
            None => {
                // Fallback: find by email
                let users =
                    filter_impl::<UserModel, _>(DB_MANAGER.db(), move |u| u.email == username)?;
                match users.into_iter().next() {
                    Some(u) => u,
                    None => {
                        return Ok(LoginResult {
                            success: false,
                            error_message: Some("User or email not found.".to_string()),
                            access_token: None,
                            theme: None,
                            language: None,
                            ..Default::default()
                        });
                    }
                }
            }
        };

        match user.password.as_ref() {
            Some(stored_password) => {
                if let PasswordCheck::Match { needs_upgrade } =
                    check_stored_password(stored_password, &password)?
                {
                    if needs_upgrade {
                        upgrade_password_storage(&user, &password);
                    }
                    if user.mfa.totp_enabled {
                        return Ok(LoginResult {
                            success: true,
                            mfa_required: true,
                            mfa_challenge: Some(MfaChallenge::issue(user.id)?),
                            ..Default::default()
                        });
                    }
                    let new_token = AccessTokenModel::reset_webui_token(user.id)?;
                    Ok(LoginResult {
                        success: true,
                        error_message: None,
                        access_token: Some(new_token),
                        theme: user.theme,
                        language: user.language,
                        ..Default::default()
                    })
                } else {
                    warn!(
                        "Login failed: Incorrect password for user '{}'.",
                        user.username
                    );
                    Ok(LoginResult {
                        success: false,
                        error_message: Some("Incorrect password.".to_string()),
                        access_token: None,
                        theme: None,
                        language: None,
                        ..Default::default()
                    })
                }
            }
            None => {
                warn!(
                    "Login failed: User '{}' has no password set.",
                    user.username
                );
                Ok(LoginResult {
                    success: false,
                    error_message: Some(
                        format!(
                            "User '{}' has no password set. Please try logging in with an alternative method (e.g., OAuth/SSO).",
                            user.username
                        )
                    ),
                    access_token: None,
                    theme: None,
                    language: None,
                    ..Default::default()
                })
            }
        }
    }

    /// Whether the default administrator still accepts
    /// [`DEFAULT_ADMIN_PASSWORD`]. Only verifies the stored value: no login,
    /// no password upgrade, no audit event.
    pub fn default_admin_password_in_use() -> bool {
        admin_accepts_default_password(Self::find(DEFAULT_ADMIN_USER_ID))
    }

    pub fn find(user_id: u64) -> MailboxdResult<Option<UserModel>> {
        find_impl::<UserModel>(DB_MANAGER.db(), &user_id.to_string())
    }

    /// Resolve or auto-provision a mailboxd user for an OIDC login.
    ///
    /// Resolution order:
    ///   1. Match on `(sso_provider, sso_id)` — same subject that logged in before.
    ///   2. Match on `email` — bind the existing user to this SSO identity.
    ///   3. Auto-provision a new user with the configured default global role.
    ///
    /// Returns the resolved/created user. The caller is expected to issue a
    /// WebUI access token for that user.
    pub fn find_or_provision_sso_user(
        provider: &str,
        subject: &str,
        email: &str,
        display_name: Option<&str>,
        default_role_id: u64,
    ) -> MailboxdResult<UserModel> {
        let subject_owned = subject.to_string();
        let provider_owned = provider.to_string();

        let matches_by_sso = filter_impl::<UserModel, _>(DB_MANAGER.db(), move |u| {
            u.sso_id.as_deref() == Some(&subject_owned)
                && u.sso_provider.as_deref() == Some(&provider_owned)
        })?;
        if let Some(user) = matches_by_sso.into_iter().next() {
            return Ok(user);
        }

        let email_owned = email.to_string();
        let matches_by_email =
            filter_impl::<UserModel, _>(DB_MANAGER.db(), move |u| u.email == email_owned)?;
        if let Some(mut user) = matches_by_email.into_iter().next() {
            let now = utc_now!();
            user.sso_id = Some(subject.to_string());
            user.sso_provider = Some(provider.to_string());
            user.updated_at = now;
            let user_clone = user.clone();
            update_impl(
                DB_MANAGER.db(),
                &user.id.to_string(),
                move |_current: UserModel| Ok(user_clone.clone()),
            )?;
            return Ok(user);
        }

        let now = utc_now!();
        let base = derive_username_base(email, display_name);
        let username = allocate_unique_username(&base)?;
        let new_user = UserModel {
            id: id!(96),
            username,
            email: email.to_string(),
            password: None,
            account_access_map: BTreeMap::new(),
            description: Some(format!("Auto-provisioned via OIDC ({})", provider)),
            global_roles: vec![default_role_id],
            avatar: None,
            created_at: now,
            updated_at: now,
            acl: None,
            theme: None,
            language: None,
            sso_id: Some(subject.to_string()),
            sso_provider: Some(provider.to_string()),
            mfa: MfaState::default(),
        };

        let user_clone = new_user.clone();
        with_transaction(DB_MANAGER.db(), move |txn| {
            txn.insert("users", new_user.key(), &new_user)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
        })?;
        Ok(user_clone)
    }

    pub fn check_username_conflict(username: &str) -> MailboxdResult<()> {
        let username_clone = username.to_string();
        let users =
            filter_impl::<UserModel, _>(DB_MANAGER.db(), move |u| u.username == username_clone)?;

        if users.into_iter().next().is_some() {
            return Err(raise_error!(
                format!("Username '{}' is already taken.", username).into(),
                ErrorCode::AlreadyExists
            ));
        }

        Ok(())
    }

    pub fn check_email_conflict(email: &str) -> MailboxdResult<()> {
        let email_clone = email.to_string();
        let users = filter_impl::<UserModel, _>(DB_MANAGER.db(), move |u| u.email == email_clone)?;

        if users.into_iter().next().is_some() {
            return Err(raise_error!(
                format!("Email '{}' is already registered.", email).into(),
                ErrorCode::AlreadyExists
            ));
        }

        Ok(())
    }

    pub fn create(request: UserCreateRequest) -> MailboxdResult<UserModel> {
        request.validate()?;
        Self::check_username_conflict(&request.username)?;
        Self::check_email_conflict(&request.email)?;

        let password_hash = Some(hash_login_password(&request.password)?);
        let now = utc_now!();

        let user = UserModel {
            id: id!(96),
            username: request.username,
            email: request.email,
            password: password_hash,
            global_roles: request.global_roles,
            avatar: request.avatar_base64,
            description: request.description,
            acl: request.acl,
            created_at: now,
            updated_at: now,
            account_access_map: request.account_access_map,
            theme: request.theme,
            language: request.language,
            sso_id: None,
            sso_provider: None,
            mfa: MfaState::default(),
        };

        let user_clone = user.clone();

        // 4. Atomic transaction for User and Initial Token
        let access_token = AccessTokenModel {
            token: generate_token!(128),
            created_at: now,
            updated_at: now,
            last_access_at: Default::default(),
            name: Some("Default WebUI Token".into()),
            user_id: user.id,
            token_type: TokenType::WebUI,
            expire_at: None,
        };

        with_transaction(DB_MANAGER.db(), move |txn| {
            let txn = txn
                .insert("users", user.key(), &user)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
                .insert("tokens", access_token.token.clone(), &access_token)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            Ok(txn)
        })?;

        Ok(user_clone)
    }

    //delete user，
    pub fn remove(id: u64) -> MailboxdResult<()> {
        if DEFAULT_ADMIN_USER_ID == id {
            return Err(raise_error!(
                format!("The default admin user (id={}) cannot be removed", id),
                ErrorCode::PermissionDenied
            ));
        }

        delete_impl::<UserModel>(DB_MANAGER.db(), &id.to_string())?;

        // Find and delete tokens belonging to this user
        let uid = id;

        let coll = DB_MANAGER.db().collection("tokens");
        let all_tokens: Vec<AccessTokenModel> = coll
            .list_all()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        let token_keys: Vec<String> = all_tokens
            .into_iter()
            .filter(|t| t.user_id == uid)
            .map(|t| t.token)
            .collect();

        if !token_keys.is_empty() {
            with_transaction(DB_MANAGER.db(), move |txn| {
                let mut txn = txn;
                for key in token_keys {
                    txn = txn.delete("tokens", key);
                }
                Ok(txn)
            })?;
        }

        Ok(())
    }

    pub fn update(id: u64, request: UserUpdateRequest) -> MailboxdResult<()> {
        let _ = &request.validate()?;
        let password_changed = request.password.is_some();
        let is_default_admin = id == DEFAULT_ADMIN_USER_ID;

        if is_default_admin {
            if let Some(roles) = request.global_roles.as_deref() {
                let is_valid = matches!(
                    roles,
                    [role] if *role == DEFAULT_ADMIN_ROLE_ID
                );

                if !is_valid {
                    return Err(raise_error!(
                        format!(
                            "The role assignments for default admin (id={}) are immutable to ensure system accessibility.",
                            id
                        ),
                        ErrorCode::Forbidden
                    ));
                }
            }
        }

        if let Some(username) = &request.username {
            let username_clone = username.clone();
            let users = filter_impl::<UserModel, _>(DB_MANAGER.db(), move |u| {
                u.username == username_clone
            })?;

            if let Some(u) = users.into_iter().next() {
                if u.id != id {
                    return Err(raise_error!(
                        format!("Username '{}' is already taken.", username).into(),
                        ErrorCode::AlreadyExists
                    ));
                }
            }
        }

        if let Some(email) = &request.email {
            let email_clone = email.clone();
            let users =
                filter_impl::<UserModel, _>(DB_MANAGER.db(), move |u| u.email == email_clone)?;

            if let Some(u) = users.into_iter().next() {
                if u.id != id {
                    return Err(raise_error!(
                        format!("Email '{}' is already registered.", email).into(),
                        ErrorCode::AlreadyExists
                    ));
                }
            }
        }

        update_impl::<UserModel>(DB_MANAGER.db(), &id.to_string(), move |current| {
            apply_user_update(&current, request)
        })?;

        if password_changed {
            AccessTokenModel::reset_webui_token(id)?;
        }

        Ok(())
    }

    fn list_authorized_users(account_id: u64) -> MailboxdResult<Vec<UserModel>> {
        let all = Self::list_all()?;
        let result: Vec<UserModel> = all
            .into_iter()
            .filter(|e| e.account_access_map.contains_key(&account_id))
            .collect();
        Ok(result)
    }

    pub fn cleanup_account(account_id: u64) -> MailboxdResult<()> {
        let users = Self::list_authorized_users(account_id)?;
        if users.is_empty() {
            return Ok(());
        }

        let now = utc_now!();
        for user in users {
            let key = user.id.to_string();
            update_impl::<UserModel>(DB_MANAGER.db(), &key, move |current| {
                let mut updated = current.clone();
                if updated.account_access_map.remove(&account_id).is_some() {
                    updated.updated_at = now;
                }
                Ok(updated)
            })?;
        }

        Ok(())
    }
}

/// PHC prefix of the argon2id hashes written by [`hash_login_password`].
/// Anything else in `UserModel::password` is a legacy, reversibly encrypted
/// password from before #70.
const ARGON2ID_PREFIX: &str = "$argon2id$";

#[derive(Debug, PartialEq, Eq)]
enum PasswordCheck {
    /// `needs_upgrade` is set when the stored value is a legacy encrypted
    /// password that should be replaced by an argon2id hash.
    Match {
        needs_upgrade: bool,
    },
    Mismatch,
}

/// Hash a login password as an argon2id PHC string with a random salt.
pub fn hash_login_password(password: &str) -> MailboxdResult<String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|e| {
            raise_error!(
                format!("Failed to hash password: {e}"),
                ErrorCode::InternalError
            )
        })
}

/// Compare a login attempt with the stored password value.
///
/// argon2id hashes are verified by argon2; legacy encrypted values are
/// decrypted and compared in constant time.
fn check_stored_password(stored: &str, password: &str) -> MailboxdResult<PasswordCheck> {
    if stored.starts_with(ARGON2ID_PREFIX) {
        let matches = match PasswordHash::new(stored) {
            Ok(parsed) => Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok(),
            Err(e) => {
                warn!("Stored password hash is malformed: {e}");
                false
            }
        };
        return Ok(if matches {
            PasswordCheck::Match {
                needs_upgrade: false,
            }
        } else {
            PasswordCheck::Mismatch
        });
    }

    let decrypted = decrypt!(stored)?;
    Ok(
        if bool::from(password.as_bytes().ct_eq(decrypted.as_bytes())) {
            PasswordCheck::Match {
                needs_upgrade: true,
            }
        } else {
            PasswordCheck::Mismatch
        },
    )
}

/// Whether the looked-up default administrator still accepts
/// [`DEFAULT_ADMIN_PASSWORD`]; a missing admin or a failed lookup counts as
/// "not in use".
fn admin_accepts_default_password(admin: MailboxdResult<Option<UserModel>>) -> bool {
    match admin {
        Ok(Some(admin)) => admin
            .password
            .as_deref()
            .is_some_and(accepts_default_password),
        Ok(None) => false,
        Err(e) => {
            warn!("Failed to load the default administrator: {:#?}", e);
            false
        }
    }
}

fn accepts_default_password(stored: &str) -> bool {
    match check_stored_password(stored, DEFAULT_ADMIN_PASSWORD) {
        Ok(check) => matches!(check, PasswordCheck::Match { .. }),
        Err(e) => {
            warn!(
                "Failed to check the default administrator password: {:#?}",
                e
            );
            false
        }
    }
}

/// Replace a legacy encrypted password with an argon2id hash after a
/// successful login. Failures are logged; the login itself still succeeds
/// and the upgrade is retried on the next login.
fn upgrade_password_storage(user: &UserModel, password: &str) {
    let hashed = match hash_login_password(password) {
        Ok(hashed) => hashed,
        Err(e) => {
            warn!(
                "Login succeeded but hashing the password for user '{}' failed: {:#?}",
                user.username, e
            );
            return;
        }
    };
    if let Err(e) =
        update_impl::<UserModel>(DB_MANAGER.db(), &user.id.to_string(), move |current| {
            let mut updated = current.clone();
            updated.password = Some(hashed);
            updated.updated_at = utc_now!();
            Ok(updated)
        })
    {
        warn!(
            "Login succeeded but upgrading the password storage for user '{}' failed: {:#?}",
            user.username, e
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_initial_password_counts_as_default() {
        let initial = hash_login_password(DEFAULT_ADMIN_PASSWORD).unwrap();
        assert!(accepts_default_password(&initial));
        let changed = hash_login_password("admin@mailboxd2").unwrap();
        assert!(!accepts_default_password(&changed));
        assert!(!accepts_default_password("$argon2id$mangled"));

        // Not logged in since the switch to hashes: still encrypted.
        let legacy = crate::encrypt!(DEFAULT_ADMIN_PASSWORD).unwrap();
        assert!(accepts_default_password(&legacy));
        let legacy_changed = crate::encrypt!("something else").unwrap();
        assert!(!accepts_default_password(&legacy_changed));
    }

    #[test]
    fn hash_login_password_produces_salted_argon2id_hashes() {
        let first = hash_login_password("s3cret").unwrap();
        let second = hash_login_password("s3cret").unwrap();
        assert!(first.starts_with(ARGON2ID_PREFIX));
        assert_ne!(first, second, "each hash must use a fresh salt");
    }

    #[test]
    fn check_stored_password_accepts_matching_argon2_hash() {
        let stored = hash_login_password("s3cret").unwrap();
        assert_eq!(
            check_stored_password(&stored, "s3cret").unwrap(),
            PasswordCheck::Match {
                needs_upgrade: false
            }
        );
    }

    #[test]
    fn check_stored_password_rejects_wrong_password_for_argon2_hash() {
        let stored = hash_login_password("s3cret").unwrap();
        assert_eq!(
            check_stored_password(&stored, "wrong").unwrap(),
            PasswordCheck::Mismatch
        );
    }

    #[test]
    fn check_stored_password_rejects_malformed_argon2_hash() {
        assert_eq!(
            check_stored_password("$argon2id$garbage", "s3cret").unwrap(),
            PasswordCheck::Mismatch
        );
    }

    #[test]
    fn check_stored_password_accepts_legacy_encrypted_password_and_requests_upgrade() {
        let stored = crate::encrypt!("s3cret").unwrap();
        assert_eq!(
            check_stored_password(&stored, "s3cret").unwrap(),
            PasswordCheck::Match {
                needs_upgrade: true
            }
        );
    }

    #[test]
    fn check_stored_password_rejects_wrong_password_for_legacy_encryption() {
        let stored = crate::encrypt!("s3cret").unwrap();
        assert_eq!(
            check_stored_password(&stored, "s3cre").unwrap(),
            PasswordCheck::Mismatch
        );
    }

    #[test]
    fn apply_user_update_stores_new_password_as_argon2_hash() {
        let current = UserModel::default();
        let request = UserUpdateRequest {
            password: Some("n3w-password".into()),
            ..Default::default()
        };
        let updated = apply_user_update(&current, request).unwrap();
        let stored = updated.password.unwrap();
        assert!(stored.starts_with(ARGON2ID_PREFIX));
        assert_eq!(
            check_stored_password(&stored, "n3w-password").unwrap(),
            PasswordCheck::Match {
                needs_upgrade: false
            }
        );
    }

    #[test]
    fn rejects_password_change_for_sso_user() {
        assert!(ensure_password_change_allowed(Some("oidc"), true).is_err());
    }

    #[test]
    fn allows_password_change_for_local_user() {
        assert!(ensure_password_change_allowed(None, true).is_ok());
    }

    #[test]
    fn ignores_absent_password_for_sso_user() {
        assert!(ensure_password_change_allowed(Some("oidc"), false).is_ok());
    }

    #[test]
    fn apply_user_update_rejects_password_for_sso_user() {
        let current = UserModel {
            sso_provider: Some("oidc".to_string()),
            ..Default::default()
        };
        let request = UserUpdateRequest {
            password: Some("new-password".to_string()),
            ..Default::default()
        };
        assert!(apply_user_update(&current, request).is_err());
    }

    #[test]
    fn apply_user_update_applies_non_password_changes_for_sso_user() {
        let current = UserModel {
            username: "old".to_string(),
            sso_provider: Some("oidc".to_string()),
            ..Default::default()
        };
        let request = UserUpdateRequest {
            username: Some("new".to_string()),
            ..Default::default()
        };
        let updated = apply_user_update(&current, request)
            .expect("non-password update should be allowed for an SSO user");
        assert_eq!(updated.username, "new");
        assert!(updated.password.is_none());
    }

    #[test]
    fn undecryptable_legacy_password_does_not_count_as_default() {
        assert!(!accepts_default_password("not-an-encrypted-value"));
    }

    #[test]
    fn default_password_check_treats_missing_or_unreadable_admin_as_not_in_use() {
        assert!(!admin_accepts_default_password(Ok(None)));
        assert!(!admin_accepts_default_password(Err(raise_error!(
            "database unavailable".into(),
            ErrorCode::InternalError
        ))));
        let sso_admin = UserModel {
            password: None,
            ..Default::default()
        };
        assert!(!admin_accepts_default_password(Ok(Some(sso_admin))));
        let changed = UserModel {
            password: Some(hash_login_password("changed-password").unwrap()),
            ..Default::default()
        };
        assert!(!admin_accepts_default_password(Ok(Some(changed))));
    }

    /// Roles and the default administrator, as created at startup.
    async fn init_users() {
        use crate::context::Initialize;
        static INIT: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
        INIT.get_or_init(|| async { manager::UserManager::initialize().await.unwrap() })
            .await;
    }

    fn stored_password(user_id: u64) -> String {
        UserModel::find(user_id)
            .unwrap()
            .expect("user exists")
            .password
            .expect("user has a password")
    }

    /// The only test in this crate that touches the default administrator.
    #[tokio::test]
    async fn default_admin_is_created_with_the_default_password() {
        init_users().await;
        assert!(stored_password(DEFAULT_ADMIN_USER_ID).starts_with(ARGON2ID_PREFIX));
        assert!(UserModel::default_admin_password_in_use());
    }

    #[tokio::test]
    async fn login_with_legacy_encrypted_password_upgrades_it_to_argon2id() {
        init_users().await;
        // Id and names used by no other test.
        let user = UserModel {
            id: 71_400_000_000_001,
            username: "legacy-login".into(),
            email: "legacy-login@example.com".into(),
            password: Some(crate::encrypt!("legacy-s3cret").unwrap()),
            global_roles: vec![DEFAULT_ADMIN_ROLE_ID],
            ..Default::default()
        };
        // Upsert: the database survives between runs.
        crate::database::upsert_impl(DB_MANAGER.db(), user.clone()).unwrap();

        let login =
            UserModel::authenticate_user("legacy-login".into(), "legacy-s3cret".into()).unwrap();
        assert!(login.success);
        assert!(login.access_token.is_some());

        let upgraded = stored_password(user.id);
        assert!(upgraded.starts_with(ARGON2ID_PREFIX));
        assert_eq!(
            check_stored_password(&upgraded, "legacy-s3cret").unwrap(),
            PasswordCheck::Match {
                needs_upgrade: false
            }
        );

        // The upgraded hash keeps accepting the password, and only it.
        let again =
            UserModel::authenticate_user("legacy-login".into(), "legacy-s3cret".into()).unwrap();
        assert!(again.success);
        let wrong =
            UserModel::authenticate_user("legacy-login".into(), "legacy-s3cre".into()).unwrap();
        assert!(!wrong.success);
    }

    #[tokio::test]
    async fn password_upgrade_for_a_vanished_user_creates_nothing() {
        // Id used by no other test.
        let user = UserModel {
            id: 71_400_000_000_002,
            username: "vanished".into(),
            ..Default::default()
        };
        upgrade_password_storage(&user, "whatever");
        assert!(UserModel::find(user.id).unwrap().is_none());
    }

    #[tokio::test]
    async fn created_users_get_an_argon2id_password_and_can_log_in() {
        init_users().await;
        // Unique per run: the database survives between runs.
        let username = format!("hashed-{}", id!(64));
        let created = UserModel::create(UserCreateRequest {
            username: username.clone(),
            email: format!("{username}@example.com"),
            password: "create-s3cret".into(),
            global_roles: vec![DEFAULT_ADMIN_ROLE_ID],
            ..Default::default()
        })
        .unwrap();

        let stored = stored_password(created.id);
        assert!(stored.starts_with(ARGON2ID_PREFIX));
        assert!(!stored.contains("create-s3cret"));
        let login = UserModel::authenticate_user(username, "create-s3cret".into()).unwrap();
        assert!(login.success);
    }

    #[tokio::test]
    async fn sso_login_links_an_existing_user_with_the_same_email() {
        init_users().await;
        // Unique per run: the database survives between runs.
        let username = format!("sso-{}", id!(64));
        let email = format!("{username}@example.com");
        let subject = format!("subject-{username}");
        let existing = UserModel::create(UserCreateRequest {
            username: username.clone(),
            email: email.clone(),
            password: "link-s3cret".into(),
            global_roles: vec![DEFAULT_ADMIN_ROLE_ID],
            ..Default::default()
        })
        .unwrap();

        let linked = UserModel::find_or_provision_sso_user(
            "oidc",
            &subject,
            &email,
            None,
            DEFAULT_ADMIN_ROLE_ID,
        )
        .unwrap();

        assert_eq!(linked.id, existing.id);
        let stored = UserModel::find(existing.id).unwrap().unwrap();
        assert_eq!(stored.sso_id.as_deref(), Some(subject.as_str()));
        assert_eq!(stored.sso_provider.as_deref(), Some("oidc"));
        assert_eq!(stored.username, username);
    }
}
