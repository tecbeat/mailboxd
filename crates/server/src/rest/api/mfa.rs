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

use crate::common::auth::WrappedContext;
use crate::rest::api::ApiTags;
use crate::rest::ApiResult;
use mailboxd_core::error::code::ErrorCode;
use mailboxd_core::error::MailboxdResult;
use mailboxd_core::ext::event_bus::{emit, Event};
use mailboxd_core::raise_error;
use mailboxd_core::users::permissions::Permission;
use mailboxd_core::users::UserModel;
use mailboxd_core::utils::rate_limit::MFA_RATE_LIMITER;
use poem::web::Path;
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use std::net::IpAddr;

pub struct MfaApi;

#[derive(Object)]
pub struct MfaStatus {
    /// Whether 2FA is on for the calling user.
    pub enabled: bool,
    /// Whether the user may enrol; false for SSO users.
    pub available: bool,
    /// Unused recovery codes.
    pub recovery_codes_left: u32,
}

#[derive(Object)]
pub struct MfaEnrolment {
    /// Base32 secret for manual entry.
    pub secret: String,
    /// `otpauth://` URI for the QR code.
    pub otpauth_uri: String,
}

#[derive(Object)]
pub struct MfaCode {
    /// A 6-digit TOTP code, or a recovery code where accepted.
    pub code: String,
}

#[derive(Object)]
pub struct MfaRecoveryCodes {
    /// One-time recovery codes; shown only once.
    pub recovery_codes: Vec<String>,
}

/// Run `f` on the blocking pool; argon2 is CPU-bound.
async fn blocking<T, F>(f: F) -> MailboxdResult<T>
where
    F: FnOnce() -> MailboxdResult<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f).await.map_err(|e| {
        raise_error!(
            format!("Two-factor task failed: {e:?}"),
            ErrorCode::InternalError
        )
    })?
}

fn limit_code_attempts(ip: Option<IpAddr>) -> MailboxdResult<()> {
    match ip {
        Some(ip) if !MFA_RATE_LIMITER.check(ip) => Err(raise_error!(
            "Too many verification attempts. Please try again later.".into(),
            ErrorCode::TooManyRequest
        )),
        _ => Ok(()),
    }
}

fn invalid_code() -> mailboxd_core::error::MailboxdError {
    raise_error!(
        "Invalid verification code.".into(),
        ErrorCode::InvalidParameter
    )
}

#[OpenApi(prefix_path = "/api/v1", tag = "ApiTags::Mfa")]
impl MfaApi {
    /// Two-factor status of the calling user.
    #[oai(path = "/mfa/status", method = "get", operation_id = "mfa_status")]
    async fn status(&self, context: WrappedContext) -> ApiResult<Json<MfaStatus>> {
        let user = &context.user;
        Ok(Json(MfaStatus {
            enabled: user.mfa.totp_enabled,
            available: user.can_enrol_mfa(),
            recovery_codes_left: user.mfa.recovery_codes.len() as u32,
        }))
    }

    /// Start enrolling: a fresh secret and its otpauth URI. 2FA is only on
    /// after `/mfa/confirm`. Refused while 2FA is on and for SSO users.
    #[oai(path = "/mfa/enroll", method = "post", operation_id = "mfa_enroll")]
    async fn enroll(&self, context: WrappedContext) -> ApiResult<Json<MfaEnrolment>> {
        let enrolment = UserModel::start_totp_enrolment(context.user.id)?;
        Ok(Json(MfaEnrolment {
            secret: enrolment.secret,
            otpauth_uri: enrolment.otpauth_uri,
        }))
    }

    /// Confirm enrolment with a current code from the authenticator. Turns
    /// 2FA on and returns the recovery codes.
    #[oai(path = "/mfa/confirm", method = "post", operation_id = "mfa_confirm")]
    async fn confirm(
        &self,
        context: WrappedContext,
        payload: Json<MfaCode>,
    ) -> ApiResult<Json<MfaRecoveryCodes>> {
        limit_code_attempts(context.ip_addr)?;
        let user_id = context.user.id;
        let code = payload.0.code;
        let codes = blocking(move || UserModel::confirm_totp_enrolment(user_id, &code)).await?;
        emit(Event::MfaEnabled {
            user: context.user.username.clone(),
            ip: context.ip_addr,
            success: codes.is_some(),
        });
        match codes {
            Some(recovery_codes) => Ok(Json(MfaRecoveryCodes { recovery_codes })),
            None => Err(invalid_code().into()),
        }
    }

    /// Turn 2FA off. Requires a current TOTP code or a recovery code.
    #[oai(path = "/mfa/disable", method = "post", operation_id = "mfa_disable")]
    async fn disable(&self, context: WrappedContext, payload: Json<MfaCode>) -> ApiResult<()> {
        limit_code_attempts(context.ip_addr)?;
        let user_id = context.user.id;
        let code = payload.0.code;
        let disabled = blocking(move || UserModel::disable_totp(user_id, &code)).await?;
        emit(Event::MfaDisabled {
            user: context.user.username.clone(),
            ip: context.ip_addr,
            success: disabled,
        });
        if !disabled {
            return Err(invalid_code().into());
        }
        Ok(())
    }

    /// Remove another user's 2FA, e.g. after they lost the authenticator and
    /// the recovery codes. They can enrol again afterwards.
    #[oai(
        path = "/mfa/admin-reset/:user_id",
        method = "post",
        operation_id = "mfa_admin_reset"
    )]
    async fn admin_reset(&self, context: WrappedContext, user_id: Path<u64>) -> ApiResult<()> {
        context.require_permission(None, Permission::USER_MANAGE)?;
        let user_id = user_id.0;
        if user_id == context.user.id {
            return Err(raise_error!(
                "Use the two-factor settings to turn off your own 2FA.".into(),
                ErrorCode::InvalidParameter
            )
            .into());
        }
        let Some(target) = UserModel::find(user_id)? else {
            return Err(raise_error!(
                format!("User {user_id} not found."),
                ErrorCode::ResourceNotFound
            )
            .into());
        };
        UserModel::reset_totp(user_id)?;
        emit(Event::MfaReset {
            reset_by: context.user.username.clone(),
            target_user: target.username,
            ip: context.ip_addr,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_failing_task_becomes_an_internal_error() {
        let err = blocking(|| -> MailboxdResult<()> { panic!("argon2 blew up") })
            .await
            .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InternalError);
    }

    #[tokio::test]
    async fn task_results_are_passed_through() {
        assert_eq!(blocking(|| Ok(7)).await.unwrap(), 7);
        let err = blocking(|| -> MailboxdResult<()> {
            Err(raise_error!("nope".into(), ErrorCode::Forbidden))
        })
        .await
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::Forbidden);
    }
}
