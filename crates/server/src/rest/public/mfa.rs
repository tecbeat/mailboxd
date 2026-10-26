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

use crate::rest::public::login::{rate_limit_rejection, run_authentication};
use mailboxd_core::ext::event_bus::{emit, Event};
use mailboxd_core::users::mfa::{
    MfaChallenge, MfaVerification, MFA_ERROR_CHALLENGE_INVALID, MFA_ERROR_INVALID_CODE,
    MFA_ERROR_TOO_MANY_ATTEMPTS,
};
use mailboxd_core::users::LoginResult;
use mailboxd_core::utils::rate_limit::MFA_RATE_LIMITER;
use poem::web::{Json, RealIp};
use poem::{handler, FromRequest, IntoResponse, Request, Response};
use serde::Deserialize;
use tracing::error;

#[derive(Deserialize)]
pub struct MfaVerifyPayload {
    pub challenge: String,
    pub code: String,
}

/// Second step of a password login with 2FA: checks a TOTP or recovery
/// code against the challenge from `/api/login` and returns the access
/// token. A wrong code keeps the challenge open until its attempts are
/// used up.
#[handler]
pub async fn mfa_verify(payload: Json<MfaVerifyPayload>, req: &Request) -> Response {
    let ip = RealIp::from_request_without_body(req)
        .await
        .ok()
        .and_then(|real_ip| real_ip.0);
    if let Some(rejection) = rate_limit_rejection(&MFA_RATE_LIMITER, ip) {
        return rejection;
    }
    let MfaVerifyPayload { challenge, code } = payload.0;
    let token = challenge.clone();
    let outcome = match run_authentication(move || MfaChallenge::verify(&token, &code)).await {
        Ok(outcome) => outcome,
        Err(failure) => return *failure,
    };
    let result = match outcome {
        Ok(MfaVerification::InvalidChallenge) => LoginResult {
            error_message: Some(
                "The sign-in has expired or is no longer valid. Please sign in again.".into(),
            ),
            error_code: Some(MFA_ERROR_CHALLENGE_INVALID.into()),
            ..Default::default()
        },
        Ok(MfaVerification::WrongCode {
            user,
            attempted,
            attempts_left,
        }) => {
            emit(Event::MfaLogin {
                user: user.username,
                ip,
                success: false,
                method: attempted.map(|factor| factor.as_str()),
            });
            if attempts_left > 0 {
                LoginResult {
                    error_message: Some("Invalid verification code.".into()),
                    error_code: Some(MFA_ERROR_INVALID_CODE.into()),
                    mfa_required: true,
                    mfa_challenge: Some(challenge),
                    ..Default::default()
                }
            } else {
                LoginResult {
                    error_message: Some("Too many invalid codes. Please sign in again.".into()),
                    error_code: Some(MFA_ERROR_TOO_MANY_ATTEMPTS.into()),
                    ..Default::default()
                }
            }
        }
        Ok(MfaVerification::Success {
            user,
            factor,
            access_token,
        }) => {
            emit(Event::MfaLogin {
                user: user.username,
                ip,
                success: true,
                method: Some(factor.as_str()),
            });
            LoginResult {
                success: true,
                access_token: Some(access_token),
                theme: user.theme,
                language: user.language,
                ..Default::default()
            }
        }
        Err(e) => {
            error!(
                "Second-factor verification failed with system error: {:?}",
                e
            );
            return Response::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .body("Authentication system failed.".to_string())
                .into_response();
        }
    };
    Json(result).into_response()
}
