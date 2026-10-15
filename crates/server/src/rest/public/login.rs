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

use mailboxd_core::ext::event_bus::{emit, Event};
use mailboxd_core::users::{LoginResult, UserModel};
use mailboxd_core::utils::rate_limit::{LoginRateLimiter, LOGIN_RATE_LIMITER};
use poem::web::{Json, RealIp};
use poem::{handler, FromRequest, IntoResponse, Request, Response};
use serde::Deserialize;
use std::net::IpAddr;
use tracing::{error, warn};

#[derive(Deserialize)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}

/// Login endpoint
///
/// Accepts a plain text password and returns the `root_token`
/// on successful authentication.
#[handler]
pub async fn login(payload: Json<LoginPayload>, req: &Request) -> Response {
    let ip = RealIp::from_request_without_body(req)
        .await
        .ok()
        .and_then(|real_ip| real_ip.0);
    if let Some(rejection) = rate_limit_rejection(&LOGIN_RATE_LIMITER, ip) {
        return rejection;
    }
    let username = payload.0.username.clone();
    let LoginPayload {
        username: login_name,
        password,
    } = payload.0;
    // argon2 verification is CPU-bound; keep it off the async workers.
    let outcome =
        tokio::task::spawn_blocking(move || UserModel::authenticate_user(login_name, password))
            .await;
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(e) => {
            error!("Authentication task failed: {:?}", e);
            return Response::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .body("Authentication system failed.".to_string())
                .into_response();
        }
    };
    match outcome {
        Ok(result) => {
            emit(Event::UserLoggedIn {
                user: username,
                ip,
                success: result.success,
            });
            match serde_json::to_string(&result) {
                Ok(json_string) => Response::builder()
                    .status(http::StatusCode::OK)
                    .content_type("application/json")
                    .body(json_string)
                    .into_response(),
                Err(_) => Response::builder()
                    .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                    .body("Internal server error during response serialization.")
                    .into_response(),
            }
        }
        Err(e) => {
            emit(Event::UserLoggedIn {
                user: username,
                ip,
                success: false,
            });
            error!("Authentication failed with system error: {:?}", e);
            Response::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .body("Authentication system failed.".to_string())
                .into_response()
        }
    }
}

/// Reject the request with 429 when `ip` has used up its login attempts.
///
/// Requests without a known client IP are not limited.
fn rate_limit_rejection(limiter: &LoginRateLimiter, ip: Option<IpAddr>) -> Option<Response> {
    let ip = ip?;
    if limiter.check(ip) {
        return None;
    }
    warn!("Login rate limit exceeded for client {}", ip);
    let body = LoginResult {
        success: false,
        error_message: Some("Too many login attempts. Please try again later.".to_string()),
        access_token: None,
        theme: None,
        language: None,
    };
    Some(
        Response::builder()
            .status(http::StatusCode::TOO_MANY_REQUESTS)
            .content_type("application/json")
            .body(serde_json::to_string(&body).unwrap_or_default())
            .into_response(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{net::Ipv4Addr, time::Duration};

    const CLIENT: Option<IpAddr> = Some(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10)));

    #[test]
    fn allows_attempts_within_the_quota() {
        let limiter = LoginRateLimiter::with_quota(2, Duration::from_secs(600));
        assert!(rate_limit_rejection(&limiter, CLIENT).is_none());
        assert!(rate_limit_rejection(&limiter, CLIENT).is_none());
    }

    #[tokio::test]
    async fn rejects_attempts_beyond_the_quota_with_429_login_result() {
        let limiter = LoginRateLimiter::with_quota(1, Duration::from_secs(600));
        assert!(rate_limit_rejection(&limiter, CLIENT).is_none());
        let response = rate_limit_rejection(&limiter, CLIENT).expect("second attempt is limited");
        assert_eq!(response.status(), http::StatusCode::TOO_MANY_REQUESTS);
        let body = response.into_body().into_string().await.unwrap();
        let result: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(result["success"], false);
        assert!(result["access_token"].is_null());
    }

    #[test]
    fn does_not_limit_requests_without_client_ip() {
        let limiter = LoginRateLimiter::with_quota(1, Duration::from_secs(600));
        for _ in 0..3 {
            assert!(rate_limit_rejection(&limiter, None).is_none());
        }
    }
}
