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

//! TOTP two-factor authentication (RFC 6238) for password logins.
//!
//! - Codes are 6 digits, SHA-1, 30 s steps, one step of clock drift either
//!   way. Each accepted time step is remembered per user, so a code can be
//!   used only once (RFC 6238 §5.2).
//! - The secret is 20 random bytes, stored encrypted.
//! - Recovery codes are one-time, stored as argon2id hashes.
//! - The second login step runs against an [`MfaChallenge`], which expires
//!   after five minutes or five wrong codes.
//! - SSO users authenticate at their identity provider and cannot enrol.

use crate::{
    database::{filter_impl, insert_impl, manager::DB_MANAGER, update_atomic_impl, MemDbModel},
    decrypt, encrypt,
    error::{code::ErrorCode, MailboxdResult},
    generate_token, raise_error,
    token::AccessTokenModel,
    users::{check_stored_password, hash_login_password, PasswordCheck, UserModel},
    utc_now,
};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, Builder, Secret, Totp};

/// Issuer shown in authenticator apps.
pub const MFA_ISSUER: &str = "mailboxd";
/// Number of recovery codes handed out at enrolment.
pub const RECOVERY_CODE_COUNT: usize = 10;
/// How long the second login step may take.
pub const MFA_CHALLENGE_TTL_MS: i64 = 5 * 60 * 1000;
/// Codes that may be tried against one login challenge.
pub const MFA_MAX_ATTEMPTS: u32 = 5;

/// The code was wrong; the challenge stays open.
pub const MFA_ERROR_INVALID_CODE: &str = "mfa_invalid_code";
/// The last attempt of the challenge failed; sign in again.
pub const MFA_ERROR_TOO_MANY_ATTEMPTS: &str = "mfa_too_many_attempts";
/// The challenge is unknown, expired or used up; sign in again.
pub const MFA_ERROR_CHALLENGE_INVALID: &str = "mfa_challenge_invalid";

const TOTP_DIGITS: usize = 6;
const TOTP_SKEW: u16 = 1;
const TOTP_STEP_SECS: u64 = 30;
const SECRET_BYTES: usize = 20;
const RECOVERY_CODE_LEN: usize = 8;
/// 32 symbols without 0/O and 1/I: a random byte modulo 32 is unbiased.
const RECOVERY_ALPHABET: &[u8; 32] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

/// The two-factor state of a user.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MfaState {
    /// Base32 TOTP secret, encrypted. Set while enrolling and while enabled.
    #[serde(default)]
    pub totp_secret: Option<String>,
    #[serde(default)]
    pub totp_enabled: bool,
    /// argon2id hashes of the unused recovery codes.
    #[serde(default)]
    pub recovery_codes: Vec<String>,
    /// The last accepted TOTP time step; only later steps are accepted.
    #[serde(default)]
    pub last_step: Option<u64>,
}

/// A started enrolment: the secret for manual entry and the otpauth URI for
/// the QR code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TotpEnrolment {
    pub secret: String,
    pub otpauth_uri: String,
}

/// The factor that passed the second login step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecondFactor {
    Totp,
    RecoveryCode,
}

impl SecondFactor {
    pub fn as_str(self) -> &'static str {
        match self {
            SecondFactor::Totp => "totp",
            SecondFactor::RecoveryCode => "recovery_code",
        }
    }
}

/// What a submitted code looks like.
#[derive(Debug, PartialEq, Eq)]
enum CodeInput {
    /// Six digits, whitespace removed.
    Totp(String),
    /// Eight recovery-code symbols, upper-cased, separators removed.
    Recovery(String),
    Invalid,
}

fn classify_code(input: &str) -> CodeInput {
    let compact: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.len() == TOTP_DIGITS && compact.bytes().all(|b| b.is_ascii_digit()) {
        return CodeInput::Totp(compact);
    }
    let normalized = normalize_recovery_code(&compact);
    if normalized.len() == RECOVERY_CODE_LEN
        && normalized.bytes().all(|b| RECOVERY_ALPHABET.contains(&b))
    {
        return CodeInput::Recovery(normalized);
    }
    CodeInput::Invalid
}

fn normalize_recovery_code(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn random_bytes<const N: usize>() -> MailboxdResult<[u8; N]> {
    let mut bytes = [0u8; N];
    SystemRandom::new().fill(&mut bytes).map_err(|_| {
        raise_error!(
            "The system random number generator failed".into(),
            ErrorCode::InternalError
        )
    })?;
    Ok(bytes)
}

/// The name an authenticator app lists the entry under. otpauth labels use
/// ':' as the issuer separator, so it is replaced.
fn account_label(user: &UserModel) -> String {
    let name = if user.email.is_empty() {
        &user.username
    } else {
        &user.email
    };
    name.replace(':', "_")
}

fn build_totp(secret_base32: &str, account: &str) -> MailboxdResult<Totp> {
    let secret = Secret::try_from_base32(secret_base32).map_err(|e| {
        raise_error!(
            format!("Stored TOTP secret is invalid: {e}"),
            ErrorCode::InternalError
        )
    })?;
    Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_digits(TOTP_DIGITS as u8)
        .with_skew(TOTP_SKEW)
        .with_step_duration(TOTP_STEP_SECS)
        .with_secret(secret)
        .with_issuer(Some(MFA_ISSUER))
        .with_account_name(account)
        .build()
        .map_err(|e| {
            raise_error!(
                format!("Failed to set up TOTP: {e}"),
                ErrorCode::InternalError
            )
        })
}

/// The time step `code` is valid for at `now_secs`, if it is later than
/// `last_step`.
fn accepted_step(totp: &Totp, code: &str, now_secs: u64, last_step: Option<u64>) -> Option<u64> {
    totp.check(code, now_secs)
        .filter(|step| last_step.is_none_or(|last| *step > last))
}

fn now_secs() -> u64 {
    (utc_now!() / 1000) as u64
}

/// Recovery codes as shown to the user, e.g. `K7QM-3XPA`.
fn generate_recovery_codes() -> MailboxdResult<Vec<String>> {
    (0..RECOVERY_CODE_COUNT)
        .map(|_| {
            let bytes = random_bytes::<RECOVERY_CODE_LEN>()?;
            let symbols: String = bytes
                .iter()
                .map(|b| RECOVERY_ALPHABET[(*b as usize) % RECOVERY_ALPHABET.len()] as char)
                .collect();
            Ok(format!("{}-{}", &symbols[..4], &symbols[4..]))
        })
        .collect()
}

fn recovery_code_matches(stored: &str, normalized: &str) -> bool {
    matches!(
        check_stored_password(stored, normalized),
        Ok(PasswordCheck::Match { .. })
    )
}

fn not_found(user_id: u64) -> crate::error::MailboxdError {
    raise_error!(
        format!("User {user_id} not found."),
        ErrorCode::ResourceNotFound
    )
}

fn already_enabled() -> crate::error::MailboxdError {
    raise_error!(
        "Two-factor authentication is already enabled. Disable it first.".into(),
        ErrorCode::AlreadyExists
    )
}

/// SSO users never pass the password login, so a second factor would never
/// be asked for.
fn ensure_can_enrol(user: &UserModel) -> MailboxdResult<()> {
    if user.sso_provider.is_some() {
        return Err(raise_error!(
            "Two-factor authentication is managed by the identity provider for SSO users.".into(),
            ErrorCode::Forbidden
        ));
    }
    if user.mfa.totp_enabled {
        return Err(already_enabled());
    }
    Ok(())
}

fn load_user(user_id: u64) -> MailboxdResult<UserModel> {
    UserModel::find(user_id)?.ok_or_else(|| not_found(user_id))
}

/// The outcome of an [`update_atomic_impl`] on a user record, with a missing
/// record as "not found".
fn updated<R>(user_id: u64, outcome: MailboxdResult<Option<R>>) -> MailboxdResult<R> {
    outcome?.ok_or_else(|| not_found(user_id))
}

fn otpauth_uri(totp: &Totp) -> MailboxdResult<String> {
    totp.to_url().map_err(|e| {
        raise_error!(
            format!("Failed to build the otpauth URI: {e}"),
            ErrorCode::InternalError
        )
    })
}

/// Store a fresh enrolment secret, unless 2FA was turned on meanwhile.
fn begin_enrolment(mut current: UserModel, encrypted: String) -> (UserModel, MailboxdResult<()>) {
    if current.mfa.totp_enabled {
        return (current, Err(already_enabled()));
    }
    current.mfa = MfaState {
        totp_secret: Some(encrypted),
        ..MfaState::default()
    };
    current.updated_at = utc_now!();
    (current, Ok(()))
}

/// Turn 2FA on for the secret the code was checked against, unless a
/// parallel request enabled 2FA or restarted the enrolment meanwhile.
fn finish_enrolment(
    mut current: UserModel,
    encrypted: &str,
    recovery_hashes: Vec<String>,
    step: u64,
) -> (UserModel, MailboxdResult<()>) {
    if current.mfa.totp_enabled {
        return (current, Err(already_enabled()));
    }
    if current.mfa.totp_secret.as_deref() != Some(encrypted) {
        return (
            current,
            Err(raise_error!(
                "The two-factor enrolment changed meanwhile. Please start again.".into(),
                ErrorCode::InvalidParameter
            )),
        );
    }
    current.mfa.totp_enabled = true;
    current.mfa.recovery_codes = recovery_hashes;
    current.mfa.last_step = Some(step);
    current.updated_at = utc_now!();
    (current, Ok(()))
}

impl UserModel {
    /// Whether this user may enrol in two-factor authentication.
    pub fn can_enrol_mfa(&self) -> bool {
        self.sso_provider.is_none()
    }

    /// Start (or restart) enrolment with a fresh secret. 2FA stays off until
    /// [`Self::confirm_totp_enrolment`] proves the authenticator works.
    pub fn start_totp_enrolment(user_id: u64) -> MailboxdResult<TotpEnrolment> {
        let user = load_user(user_id)?;
        ensure_can_enrol(&user)?;

        let secret = Secret::from(random_bytes::<SECRET_BYTES>()?).to_base32();
        let otpauth_uri = otpauth_uri(&build_totp(&secret, &account_label(&user))?)?;
        let encrypted = encrypt!(&secret)?;

        let stored = update_atomic_impl(DB_MANAGER.db(), &user_id.to_string(), |current| {
            begin_enrolment(current, encrypted)
        });
        updated(user_id, stored)??;
        Ok(TotpEnrolment {
            secret,
            otpauth_uri,
        })
    }

    /// Finish enrolment with a current code from the authenticator. Turns
    /// 2FA on and returns the recovery codes, which are shown only once.
    /// `Ok(None)` when the code is wrong.
    pub fn confirm_totp_enrolment(user_id: u64, code: &str) -> MailboxdResult<Option<Vec<String>>> {
        let user = load_user(user_id)?;
        ensure_can_enrol(&user)?;
        let Some(encrypted) = user.mfa.totp_secret.clone() else {
            return Err(raise_error!(
                "No two-factor enrolment in progress.".into(),
                ErrorCode::InvalidParameter
            ));
        };
        let CodeInput::Totp(code) = classify_code(code) else {
            return Ok(None);
        };
        let totp = build_totp(&decrypt!(&encrypted)?, &account_label(&user))?;
        let Some(step) = accepted_step(&totp, &code, now_secs(), user.mfa.last_step) else {
            return Ok(None);
        };

        let codes = generate_recovery_codes()?;
        let hashes = codes
            .iter()
            .map(|code| hash_login_password(&normalize_recovery_code(code)))
            .collect::<MailboxdResult<Vec<_>>>()?;

        let enabled = update_atomic_impl(DB_MANAGER.db(), &user_id.to_string(), |current| {
            finish_enrolment(current, &encrypted, hashes, step)
        });
        updated(user_id, enabled)??;
        Ok(Some(codes))
    }

    /// Check a TOTP or recovery code of a user with 2FA enabled and consume
    /// it. `Ok(None)` for a wrong, reused or malformed code, or when 2FA is
    /// off.
    pub fn verify_second_factor(user_id: u64, code: &str) -> MailboxdResult<Option<SecondFactor>> {
        let user = load_user(user_id)?;
        if !user.mfa.totp_enabled {
            return Ok(None);
        }
        let key = user_id.to_string();
        match classify_code(code) {
            CodeInput::Totp(code) => {
                let Some(encrypted) = user.mfa.totp_secret.clone() else {
                    return Ok(None);
                };
                let totp = build_totp(&decrypt!(&encrypted)?, &account_label(&user))?;
                let Some(step) = accepted_step(&totp, &code, now_secs(), user.mfa.last_step) else {
                    return Ok(None);
                };
                // Accept the step only once, even for parallel requests.
                let accepted =
                    update_atomic_impl(DB_MANAGER.db(), &key, move |mut current: UserModel| {
                        let fresh = current.mfa.totp_enabled
                            && current.mfa.totp_secret.as_deref() == Some(encrypted.as_str())
                            && current.mfa.last_step.is_none_or(|last| step > last);
                        if fresh {
                            current.mfa.last_step = Some(step);
                        }
                        (current, fresh)
                    })?;
                Ok((accepted == Some(true)).then_some(SecondFactor::Totp))
            }
            CodeInput::Recovery(code) => {
                // argon2 runs outside the database lock.
                let Some(matched) = user
                    .mfa
                    .recovery_codes
                    .iter()
                    .find(|stored| recovery_code_matches(stored, &code))
                    .cloned()
                else {
                    return Ok(None);
                };
                let consumed =
                    update_atomic_impl(DB_MANAGER.db(), &key, move |mut current: UserModel| {
                        let before = current.mfa.recovery_codes.len();
                        current
                            .mfa
                            .recovery_codes
                            .retain(|stored| *stored != matched);
                        let removed =
                            current.mfa.totp_enabled && current.mfa.recovery_codes.len() < before;
                        if removed {
                            current.updated_at = utc_now!();
                        }
                        (current, removed)
                    })?;
                Ok((consumed == Some(true)).then_some(SecondFactor::RecoveryCode))
            }
            CodeInput::Invalid => Ok(None),
        }
    }

    /// Turn 2FA off after checking a current TOTP or recovery code.
    /// `Ok(false)` when the code is wrong.
    pub fn disable_totp(user_id: u64, code: &str) -> MailboxdResult<bool> {
        let user = load_user(user_id)?;
        if !user.mfa.totp_enabled {
            return Err(raise_error!(
                "Two-factor authentication is not enabled.".into(),
                ErrorCode::InvalidParameter
            ));
        }
        if Self::verify_second_factor(user_id, code)?.is_none() {
            return Ok(false);
        }
        Self::reset_totp(user_id)?;
        Ok(true)
    }

    /// Remove all 2FA state, e.g. when an administrator resets it for a user
    /// who lost the authenticator.
    pub fn reset_totp(user_id: u64) -> MailboxdResult<()> {
        let reset = update_atomic_impl(
            DB_MANAGER.db(),
            &user_id.to_string(),
            |mut current: UserModel| {
                current.mfa = MfaState::default();
                current.updated_at = utc_now!();
                (current, ())
            },
        );
        updated(user_id, reset)
    }
}

/// The pending second step of a password login.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MfaChallenge {
    pub token: String,
    pub user_id: u64,
    pub expire_at: i64,
    /// Codes tried so far.
    pub attempts: u32,
}

impl MemDbModel for MfaChallenge {
    fn collection() -> &'static str {
        "mfa_challenges"
    }
    fn key(&self) -> String {
        self.token.clone()
    }
}

/// The outcome of a second login step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MfaVerification {
    /// Unknown, expired or used-up challenge, or the user is gone or no
    /// longer has 2FA.
    InvalidChallenge,
    /// The code was wrong. `attempts_left` is 0 when the challenge is gone.
    /// `attempted` is the kind of code that was tried, if it was well-formed.
    WrongCode {
        user: Box<UserModel>,
        attempted: Option<SecondFactor>,
        attempts_left: u32,
    },
    Success {
        user: Box<UserModel>,
        factor: SecondFactor,
        access_token: String,
    },
}

impl MfaChallenge {
    /// Open a second login step for `user_id` and return its token.
    /// Expired challenges of all users are removed on the way.
    pub fn issue(user_id: u64) -> MailboxdResult<String> {
        let now = utc_now!();
        let expired = filter_impl::<MfaChallenge, _>(DB_MANAGER.db(), move |c| c.expire_at <= now)?;
        for challenge in expired {
            Self::discard(&challenge.token)?;
        }

        let challenge = MfaChallenge {
            token: generate_token!(128),
            user_id,
            expire_at: now + MFA_CHALLENGE_TTL_MS,
            attempts: 0,
        };
        let token = challenge.token.clone();
        insert_impl(DB_MANAGER.db(), challenge)?;
        Ok(token)
    }

    /// Check `code` against the challenge `token`; on success the challenge
    /// is consumed and a WebUI token issued.
    pub fn verify(token: &str, code: &str) -> MailboxdResult<MfaVerification> {
        let now = utc_now!();
        // Count the attempt before checking the code, so parallel requests
        // cannot try more than MFA_MAX_ATTEMPTS codes.
        let claimed =
            update_atomic_impl(DB_MANAGER.db(), token, move |mut current: MfaChallenge| {
                if current.expire_at <= now || current.attempts >= MFA_MAX_ATTEMPTS {
                    return (current, None);
                }
                current.attempts += 1;
                let attempts = current.attempts;
                let user_id = current.user_id;
                (current, Some((user_id, attempts)))
            })?;
        let Some((user_id, attempts)) = claimed.flatten() else {
            Self::discard(token)?;
            return Ok(MfaVerification::InvalidChallenge);
        };

        let user = match UserModel::find(user_id)? {
            Some(user) if user.mfa.totp_enabled => user,
            _ => {
                Self::discard(token)?;
                return Ok(MfaVerification::InvalidChallenge);
            }
        };

        match UserModel::verify_second_factor(user_id, code)? {
            Some(factor) => {
                if !Self::discard(token)? {
                    // A parallel request finished this challenge first.
                    return Ok(MfaVerification::InvalidChallenge);
                }
                let access_token = AccessTokenModel::reset_webui_token(user_id)?;
                Ok(MfaVerification::Success {
                    user: Box::new(user),
                    factor,
                    access_token,
                })
            }
            None => {
                let attempts_left = MFA_MAX_ATTEMPTS - attempts;
                if attempts_left == 0 {
                    Self::discard(token)?;
                }
                let attempted = match classify_code(code) {
                    CodeInput::Totp(_) => Some(SecondFactor::Totp),
                    CodeInput::Recovery(_) => Some(SecondFactor::RecoveryCode),
                    CodeInput::Invalid => None,
                };
                Ok(MfaVerification::WrongCode {
                    user: Box::new(user),
                    attempted,
                    attempts_left,
                })
            }
        }
    }

    /// Remove a challenge; whether it still existed.
    fn discard(token: &str) -> MailboxdResult<bool> {
        DB_MANAGER
            .db()
            .collection(Self::collection())
            .delete(token)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
    }
}

#[cfg(test)]
#[path = "mfa_tests.rs"]
mod tests;
