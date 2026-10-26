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

use crate::error::code::ErrorCode;
use crate::error::MailboxdResult;
use crate::oidc::id_token::IdTokenClaims;
use crate::raise_error;
use serde::Deserialize;

/// Claims read from the UserInfo endpoint. Extra claims are ignored.
#[derive(Debug, Clone, Deserialize)]
pub struct UserInfoClaims {
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: Option<bool>,
    #[serde(default)]
    pub preferred_username: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub given_name: Option<String>,
}

/// Whether the verified ID token lacks a claim mailboxd needs to log a user
/// in, so the UserInfo endpoint has to be asked (e.g. Authelia only serves
/// `email` there by default).
pub fn needs_userinfo(claims: &IdTokenClaims) -> bool {
    claims.email.as_deref().is_none_or(str::is_empty)
}

/// Fill the claims missing from the ID token with the UserInfo response. The
/// ID token wins where both carry a value.
pub fn merge_userinfo(claims: &mut IdTokenClaims, info: UserInfoClaims) -> MailboxdResult<()> {
    // https://openid.net/specs/openid-connect-core-1_0.html#UserInfoResponse
    if info.sub != claims.sub {
        return Err(raise_error!(
            "OIDC UserInfo 'sub' does not match the ID token subject".into(),
            ErrorCode::PermissionDenied
        ));
    }

    if needs_userinfo(claims) {
        if let Some(email) = info.email.filter(|e| !e.is_empty()) {
            claims.email = Some(email);
            claims.email_verified = info.email_verified;
        }
    }
    fill_missing(&mut claims.preferred_username, info.preferred_username);
    fill_missing(&mut claims.name, info.name);
    fill_missing(&mut claims.given_name, info.given_name);
    Ok(())
}

fn fill_missing(target: &mut Option<String>, value: Option<String>) {
    if target.as_deref().is_none_or(str::is_empty) {
        if let Some(v) = value.filter(|v| !v.is_empty()) {
            *target = Some(v);
        }
    }
}

pub async fn fetch_userinfo(
    http: &reqwest::Client,
    endpoint: &str,
    access_token: &str,
) -> MailboxdResult<UserInfoClaims> {
    let resp = http
        .get(endpoint)
        .bearer_auth(access_token)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| {
            raise_error!(
                format!("OIDC UserInfo request to {} failed: {}", endpoint, e),
                ErrorCode::HttpResponseError
            )
        })?;

    if !resp.status().is_success() {
        return Err(raise_error!(
            format!(
                "OIDC UserInfo endpoint {} returned status {}",
                endpoint,
                resp.status()
            ),
            ErrorCode::HttpResponseError
        ));
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if content_type.starts_with("application/jwt") {
        return Err(raise_error!(
            "OIDC UserInfo endpoint returned a signed (JWT) response, which is not supported; \
             configure the provider to return plain JSON"
                .into(),
            ErrorCode::HttpResponseError
        ));
    }

    resp.json::<UserInfoClaims>().await.map_err(|e| {
        raise_error!(
            format!("Failed to parse OIDC UserInfo response: {}", e),
            ErrorCode::HttpResponseError
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn id_claims(email: Option<&str>) -> IdTokenClaims {
        serde_json::from_value(json!({
            "iss": "https://idp.example.test",
            "aud": "mailboxd",
            "sub": "user-1",
            "exp": 1_800_000_000,
            "email": email,
        }))
        .expect("claims")
    }

    fn info(value: serde_json::Value) -> UserInfoClaims {
        serde_json::from_value(value).expect("userinfo")
    }

    #[test]
    fn id_token_without_email_needs_userinfo() {
        assert!(needs_userinfo(&id_claims(None)));
        assert!(needs_userinfo(&id_claims(Some(""))));
        assert!(!needs_userinfo(&id_claims(Some("a@example.test"))));
    }

    #[test]
    fn merge_takes_email_from_userinfo_when_id_token_has_none() {
        let mut claims = id_claims(None);
        merge_userinfo(
            &mut claims,
            info(json!({
                "sub": "user-1",
                "email": "a@example.test",
                "email_verified": true,
                "preferred_username": "alice",
            })),
        )
        .expect("merge");

        assert_eq!(claims.email.as_deref(), Some("a@example.test"));
        assert_eq!(claims.email_verified, Some(true));
        assert_eq!(claims.preferred_username.as_deref(), Some("alice"));
    }

    #[test]
    fn merge_keeps_id_token_values() {
        let mut claims = id_claims(Some("token@example.test"));
        claims.name = Some("Token Name".into());
        merge_userinfo(
            &mut claims,
            info(json!({
                "sub": "user-1",
                "email": "userinfo@example.test",
                "name": "Userinfo Name",
            })),
        )
        .expect("merge");

        assert_eq!(claims.email.as_deref(), Some("token@example.test"));
        assert_eq!(claims.name.as_deref(), Some("Token Name"));
    }

    #[test]
    fn merge_rejects_userinfo_for_another_subject() {
        let mut claims = id_claims(None);
        let err = merge_userinfo(
            &mut claims,
            info(json!({ "sub": "someone-else", "email": "evil@example.test" })),
        );

        assert!(err.is_err());
        assert_eq!(claims.email, None);
    }

    #[test]
    fn merge_without_email_in_userinfo_leaves_email_missing() {
        let mut claims = id_claims(None);
        merge_userinfo(&mut claims, info(json!({ "sub": "user-1", "email": "" }))).expect("merge");

        assert!(needs_userinfo(&claims));
    }

    /// Serves exactly one HTTP response and returns the raw request it got.
    async fn serve_once(
        status: &'static str,
        content_type: &'static str,
        body: &'static str,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let url = format!("http://{}/userinfo", listener.local_addr().expect("addr"));
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buf = vec![0u8; 8192];
            let n = socket.read(&mut buf).await.expect("read");
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.expect("write");
            String::from_utf8_lossy(&buf[..n]).into_owned()
        });
        (url, handle)
    }

    #[tokio::test]
    async fn fetch_sends_bearer_token_and_parses_json() {
        let (url, server) = serve_once(
            "200 OK",
            "application/json",
            r#"{"sub":"user-1","email":"a@example.test","email_verified":true,"groups":["x"]}"#,
        )
        .await;

        let info = fetch_userinfo(&reqwest::Client::new(), &url, "access-123")
            .await
            .expect("userinfo");
        let request = server.await.expect("server").to_ascii_lowercase();

        assert_eq!(info.sub, "user-1");
        assert_eq!(info.email.as_deref(), Some("a@example.test"));
        assert!(request.starts_with("get /userinfo "));
        assert!(request.contains("authorization: bearer access-123"));
    }

    #[tokio::test]
    async fn fetch_fails_on_error_status() {
        let (url, _server) = serve_once("401 Unauthorized", "application/json", "{}").await;

        assert!(fetch_userinfo(&reqwest::Client::new(), &url, "t")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn fetch_rejects_signed_jwt_response() {
        let (url, _server) = serve_once("200 OK", "application/jwt", "a.b.c").await;

        let err = fetch_userinfo(&reqwest::Client::new(), &url, "t")
            .await
            .expect_err("jwt must be rejected");
        assert!(format!("{err:?}").contains("JWT"));
    }
}
