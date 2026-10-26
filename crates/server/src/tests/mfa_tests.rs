//
// Copyright (c) 2026 tecbeat
//
// Part of mailboxd, a fork of the Bichon email archiving project.
// Modifications by tecbeat, 2026.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::{AtomicU32, Ordering};

use mailboxd_core::audit::AuditEntry;
use mailboxd_core::database::{list_all_impl, manager::DB_MANAGER, upsert_impl};
use mailboxd_core::users::{UserModel, DEFAULT_ADMIN_USER_ID};
use poem::http::StatusCode;
use poem::test::TestClient;
use serde_json::{json, Value};
use totp_rs::{Algorithm, Builder, Secret};

use super::{admin_token, build_api_route, login, setup};

const MEMBER_ROLE_ID: u64 = 100_200_000_000_000;
const PASSWORD: &str = "mfa-pass-123";

static COUNTER: AtomicU32 = AtomicU32::new(1);

/// A client IP of its own, so the per-IP limiters of one test do not
/// affect another.
fn unique_ip() -> IpAddr {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    IpAddr::V4(Ipv4Addr::new(10, 76, (n >> 8) as u8, n as u8))
}

/// Unique across runs, because the test database persists.
fn unique_name() -> String {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("mfa{}x{n}", chrono_millis())
}

fn chrono_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

fn api(ip: IpAddr) -> TestClient<impl poem::Endpoint> {
    TestClient::new(build_api_route()).default_header("X-Forwarded-For", ip.to_string())
}

fn public(ip: IpAddr) -> TestClient<impl poem::Endpoint> {
    let route = poem::Route::new()
        .at("/api/login", poem::post(crate::rest::public::login::login))
        .at(
            "/api/auth/mfa/verify",
            poem::post(crate::rest::public::mfa::mfa_verify),
        );
    TestClient::new(route).default_header("X-Forwarded-For", ip.to_string())
}

/// A member created by the admin: (id, username).
async fn create_member() -> (u64, String) {
    setup().await;
    let username = unique_name();
    let resp = api(unique_ip())
        .post("/api/v1/users")
        .header("Authorization", bearer(&admin_token().await))
        .body_json(&json!({
            "username": username,
            "email": format!("{username}@example.com"),
            "password": PASSWORD,
            "global_roles": [MEMBER_ROLE_ID],
            "account_access_map": {},
        }))
        .send()
        .await;
    resp.assert_status_is_ok();
    let user: Value = resp.json().await.value().deserialize();
    (user["id"].as_u64().unwrap(), username)
}

fn code_at(secret: &str, offset_steps: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_digits(6)
        .with_skew(1)
        .with_step_duration(30)
        .with_secret(Secret::try_from_base32(secret).unwrap())
        .build()
        .unwrap()
        .generate((now + offset_steps * 30) as u64)
        .to_string()
}

/// A well-formed code outside the accepted drift.
fn wrong_code(secret: &str) -> String {
    let valid: Vec<String> = (-1..=1).map(|o| code_at(secret, o)).collect();
    (0..1_000_000)
        .map(|n| format!("{n:06}"))
        .find(|code| !valid.contains(code))
        .unwrap()
}

struct Enrolled {
    id: u64,
    username: String,
    token: String,
    secret: String,
    recovery_codes: Vec<String>,
}

/// A member with 2FA on, enrolled through the API.
async fn enrolled_member() -> Enrolled {
    let (id, username) = create_member().await;
    let token = login(&username, PASSWORD).await;
    let cli = api(unique_ip());
    let resp = cli
        .post("/api/v1/mfa/enroll")
        .header("Authorization", bearer(&token))
        .send()
        .await;
    resp.assert_status_is_ok();
    let enrolment: Value = resp.json().await.value().deserialize();
    let secret = enrolment["secret"].as_str().unwrap().to_string();
    let uri = enrolment["otpauth_uri"].as_str().unwrap();
    assert!(uri.starts_with("otpauth://totp/mailboxd:"), "{uri}");
    assert!(uri.contains(&format!("secret={secret}")), "{uri}");

    let resp = cli
        .post("/api/v1/mfa/confirm")
        .header("Authorization", bearer(&token))
        .body_json(&json!({ "code": code_at(&secret, 0) }))
        .send()
        .await;
    resp.assert_status_is_ok();
    let codes: Value = resp.json().await.value().deserialize();
    let recovery_codes: Vec<String> = codes["recovery_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect();
    assert_eq!(recovery_codes.len(), 10);
    Enrolled {
        id,
        username,
        token,
        secret,
        recovery_codes,
    }
}

async fn password_login(cli: &TestClient<impl poem::Endpoint>, username: &str) -> Value {
    let resp = cli
        .post("/api/login")
        .body_json(&json!({ "username": username, "password": PASSWORD }))
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.json().await.value().deserialize()
}

async fn verify(cli: &TestClient<impl poem::Endpoint>, challenge: &str, code: &str) -> Value {
    let resp = cli
        .post("/api/auth/mfa/verify")
        .body_json(&json!({ "challenge": challenge, "code": code }))
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.json().await.value().deserialize()
}

/// Audit entries about `username` written at or after `since` (ms).
fn audit_of(username: &str, since: u128) -> Vec<AuditEntry> {
    list_all_impl::<AuditEntry>(DB_MANAGER.db())
        .unwrap()
        .into_iter()
        .filter(|e| e.created_at as u128 >= since)
        .filter(|e| e.actor == username || e.target.as_deref() == Some(username))
        .collect()
}

async fn status(token: &str) -> Value {
    let resp = api(unique_ip())
        .get("/api/v1/mfa/status")
        .header("Authorization", bearer(token))
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.json().await.value().deserialize()
}

// ── Second login step ──────────────────────────────────────────────────────

#[tokio::test]
async fn a_login_with_two_factor_needs_the_second_step() {
    mailboxd_core::audit::install();
    let user = enrolled_member().await;
    let since = chrono_millis();
    let cli = public(unique_ip());

    let first = password_login(&cli, &user.username).await;
    assert_eq!(first["success"], true);
    assert_eq!(first["mfa_required"], true);
    assert!(first["access_token"].is_null(), "{first}");
    let challenge = first["mfa_challenge"].as_str().unwrap().to_string();
    assert!(
        audit_of(&user.username, since)
            .iter()
            .all(|e| e.event_type != "user.login"),
        "the password step alone is no login"
    );

    let wrong = verify(&cli, &challenge, &wrong_code(&user.secret)).await;
    assert_eq!(wrong["success"], false);
    assert_eq!(wrong["error_message"], "Invalid verification code.");
    assert_eq!(wrong["error_code"], "mfa_invalid_code");
    assert_eq!(wrong["mfa_required"], true);
    assert_eq!(wrong["mfa_challenge"], challenge.as_str());

    let done = verify(&cli, &challenge, &code_at(&user.secret, 1)).await;
    assert_eq!(done["success"], true, "{done}");
    assert!(done.get("mfa_required").is_none(), "{done}");
    assert!(done.get("error_code").is_none(), "{done}");
    let token = done["access_token"].as_str().unwrap();
    assert_eq!(status(token).await["enabled"], true, "the token works");

    let logins: Vec<(bool, Option<String>)> = audit_of(&user.username, since)
        .into_iter()
        .filter(|e| e.event_type == "user.login")
        .map(|e| (e.success, e.detail))
        .collect();
    assert_eq!(
        logins,
        vec![(false, Some("totp".into())), (true, Some("totp".into()))]
    );

    let again = verify(&cli, &challenge, &code_at(&user.secret, 1)).await;
    assert_eq!(again["success"], false);
    assert!(again.get("mfa_challenge").is_none(), "{again}");
}

#[tokio::test]
async fn a_recovery_code_completes_the_login() {
    mailboxd_core::audit::install();
    let user = enrolled_member().await;
    let since = chrono_millis();
    let cli = public(unique_ip());
    let challenge = password_login(&cli, &user.username).await["mfa_challenge"]
        .as_str()
        .unwrap()
        .to_string();
    let done = verify(&cli, &challenge, &user.recovery_codes[0]).await;
    assert_eq!(done["success"], true, "{done}");
    let detail = audit_of(&user.username, since)
        .into_iter()
        .find(|e| e.event_type == "user.login")
        .and_then(|e| e.detail);
    assert_eq!(detail.as_deref(), Some("recovery_code"));
    let token = done["access_token"].as_str().unwrap();
    assert_eq!(status(token).await["recovery_codes_left"], 9);
    // The login replaced the previous WebUI token.
    let resp = api(unique_ip())
        .get("/api/v1/mfa/status")
        .header("Authorization", bearer(&user.token))
        .send()
        .await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn five_wrong_codes_end_the_login() {
    let user = enrolled_member().await;
    let cli = public(unique_ip());
    let challenge = password_login(&cli, &user.username).await["mfa_challenge"]
        .as_str()
        .unwrap()
        .to_string();
    for _ in 0..4 {
        let wrong = verify(&cli, &challenge, "nope").await;
        assert_eq!(wrong["mfa_required"], true);
    }
    let last = verify(&cli, &challenge, "nope").await;
    assert_eq!(last["success"], false);
    assert_eq!(
        last["error_message"],
        "Too many invalid codes. Please sign in again."
    );
    assert_eq!(last["error_code"], "mfa_too_many_attempts");
    assert!(last.get("mfa_challenge").is_none(), "{last}");

    let late = verify(&cli, &challenge, &code_at(&user.secret, 1)).await;
    assert_eq!(late["success"], false);
    assert!(late["access_token"].is_null());
}

#[tokio::test]
async fn a_malformed_second_step_is_a_bad_request() {
    setup().await;
    let resp = public(unique_ip())
        .post("/api/auth/mfa/verify")
        .content_type("application/json")
        .body(r#"{"challenge":"c"}"#)
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "{}", resp.0.status());
}

#[tokio::test]
async fn an_unknown_challenge_asks_to_sign_in_again() {
    setup().await;
    let result = verify(&public(unique_ip()), "no-such-challenge", "123456").await;
    assert_eq!(result["success"], false);
    assert_eq!(
        result["error_message"],
        "The sign-in has expired or is no longer valid. Please sign in again."
    );
    assert_eq!(result["error_code"], "mfa_challenge_invalid");
}

#[tokio::test]
async fn a_broken_stored_secret_is_a_server_error() {
    let user = enrolled_member().await;
    let cli = public(unique_ip());
    let challenge = password_login(&cli, &user.username).await["mfa_challenge"]
        .as_str()
        .unwrap()
        .to_string();
    let mut broken = UserModel::find(user.id).unwrap().unwrap();
    broken.mfa.totp_secret = Some("not-encrypted".into());
    upsert_impl(DB_MANAGER.db(), broken).unwrap();

    let resp = cli
        .post("/api/auth/mfa/verify")
        .body_json(&json!({ "challenge": challenge, "code": "123456" }))
        .send()
        .await;
    resp.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn second_step_attempts_are_rate_limited_per_ip() {
    setup().await;
    let cli = public(unique_ip());
    for _ in 0..10 {
        verify(&cli, "no-such-challenge", "123456").await;
    }
    let resp = cli
        .post("/api/auth/mfa/verify")
        .body_json(&json!({ "challenge": "no-such-challenge", "code": "123456" }))
        .send()
        .await;
    resp.assert_status(StatusCode::TOO_MANY_REQUESTS);
    // The password login has a limiter of its own.
    let resp = cli
        .post("/api/login")
        .body_json(&json!({ "username": "nobody", "password": "x" }))
        .send()
        .await;
    resp.assert_status_is_ok();
}

// ── Managing 2FA ───────────────────────────────────────────────────────────

#[tokio::test]
async fn status_reports_off_and_available_before_enrolment() {
    let (_, username) = create_member().await;
    let token = login(&username, PASSWORD).await;
    assert_eq!(
        status(&token).await,
        json!({ "enabled": false, "available": true, "recovery_codes_left": 0 })
    );
}

#[tokio::test]
async fn enrolment_needs_a_valid_code_and_cannot_repeat() {
    mailboxd_core::audit::install();
    let since = chrono_millis();
    let (_, username) = create_member().await;
    let token = login(&username, PASSWORD).await;
    let cli = api(unique_ip());

    let resp = cli
        .post("/api/v1/mfa/enroll")
        .header("Authorization", bearer(&token))
        .send()
        .await;
    resp.assert_status_is_ok();
    let secret = resp.json().await.value().deserialize::<Value>()["secret"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = cli
        .post("/api/v1/mfa/confirm")
        .header("Authorization", bearer(&token))
        .body_json(&json!({ "code": wrong_code(&secret) }))
        .send()
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    assert_eq!(status(&token).await["enabled"], false);

    let resp = cli
        .post("/api/v1/mfa/confirm")
        .header("Authorization", bearer(&token))
        .body_json(&json!({ "code": code_at(&secret, 0) }))
        .send()
        .await;
    resp.assert_status_is_ok();
    assert_eq!(
        status(&token).await,
        json!({ "enabled": true, "available": true, "recovery_codes_left": 10 })
    );

    let resp = cli
        .post("/api/v1/mfa/enroll")
        .header("Authorization", bearer(&token))
        .send()
        .await;
    resp.assert_status(StatusCode::CONFLICT);

    let enabled: Vec<bool> = audit_of(&username, since)
        .into_iter()
        .filter(|e| e.event_type == "mfa.enabled")
        .map(|e| e.success)
        .collect();
    assert_eq!(enabled, vec![false, true]);
}

#[tokio::test]
async fn confirming_without_enrolment_is_a_bad_request() {
    let (_, username) = create_member().await;
    let token = login(&username, PASSWORD).await;
    let resp = api(unique_ip())
        .post("/api/v1/mfa/confirm")
        .header("Authorization", bearer(&token))
        .body_json(&json!({ "code": "123456" }))
        .send()
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn sso_users_cannot_enrol() {
    let (id, username) = create_member().await;
    let token = login(&username, PASSWORD).await;
    let mut sso = UserModel::find(id).unwrap().unwrap();
    sso.sso_provider = Some("oidc".into());
    sso.sso_id = Some(format!("subject-{id}"));
    upsert_impl(DB_MANAGER.db(), sso).unwrap();

    assert_eq!(status(&token).await["available"], false);
    let resp = api(unique_ip())
        .post("/api/v1/mfa/enroll")
        .header("Authorization", bearer(&token))
        .send()
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn disabling_needs_a_valid_code() {
    mailboxd_core::audit::install();
    let since = chrono_millis();
    let user = enrolled_member().await;
    let cli = api(unique_ip());

    let resp = cli
        .post("/api/v1/mfa/disable")
        .header("Authorization", bearer(&user.token))
        .body_json(&json!({ "code": wrong_code(&user.secret) }))
        .send()
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    assert_eq!(status(&user.token).await["enabled"], true);

    let resp = cli
        .post("/api/v1/mfa/disable")
        .header("Authorization", bearer(&user.token))
        .body_json(&json!({ "code": code_at(&user.secret, 1) }))
        .send()
        .await;
    resp.assert_status_is_ok();
    assert_eq!(status(&user.token).await["enabled"], false);

    let disabled: Vec<bool> = audit_of(&user.username, since)
        .into_iter()
        .filter(|e| e.event_type == "mfa.disabled")
        .map(|e| e.success)
        .collect();
    assert_eq!(disabled, vec![false, true]);

    let first = password_login(&public(unique_ip()), &user.username).await;
    assert!(first["access_token"].is_string(), "no second step: {first}");
}

#[tokio::test]
async fn code_attempts_are_rate_limited_per_ip() {
    let (_, username) = create_member().await;
    let token = login(&username, PASSWORD).await;
    let cli = api(unique_ip());
    for _ in 0..10 {
        let resp = cli
            .post("/api/v1/mfa/disable")
            .header("Authorization", bearer(&token))
            .body_json(&json!({ "code": "123456" }))
            .send()
            .await;
        resp.assert_status(StatusCode::BAD_REQUEST);
    }
    for path in ["/api/v1/mfa/disable", "/api/v1/mfa/confirm"] {
        let resp = cli
            .post(path)
            .header("Authorization", bearer(&token))
            .body_json(&json!({ "code": "123456" }))
            .send()
            .await;
        resp.assert_status(StatusCode::TOO_MANY_REQUESTS);
    }
}

#[tokio::test]
async fn an_admin_can_reset_another_users_two_factor() {
    mailboxd_core::audit::install();
    let since = chrono_millis();
    let user = enrolled_member().await;
    let resp = api(unique_ip())
        .post(format!("/api/v1/mfa/admin-reset/{}", user.id))
        .header("Authorization", bearer(&admin_token().await))
        .send()
        .await;
    resp.assert_status_is_ok();
    assert_eq!(status(&user.token).await["enabled"], false);

    let reset = audit_of(&user.username, since)
        .into_iter()
        .find(|e| e.event_type == "mfa.reset")
        .expect("reset audited");
    assert_eq!(reset.actor, "admin");
    assert_eq!(reset.target.as_deref(), Some(user.username.as_str()));
}

#[tokio::test]
async fn the_user_list_shows_who_has_two_factor_on() {
    let user = enrolled_member().await;
    let (plain_id, _) = create_member().await;
    let resp = api(unique_ip())
        .get("/api/v1/list-users")
        .header("Authorization", bearer(&admin_token().await))
        .send()
        .await;
    resp.assert_status_is_ok();
    let users: Value = resp.json().await.value().deserialize();
    let totp_of = |id: u64| {
        users
            .as_array()
            .unwrap()
            .iter()
            .find(|u| u["id"] == id)
            .map(|u| u["totp_enabled"].clone())
    };
    assert_eq!(totp_of(user.id), Some(json!(true)));
    assert_eq!(totp_of(plain_id), Some(json!(false)));
    let body = users.to_string();
    assert!(
        !body.contains(&user.secret),
        "the secret never leaves the server"
    );
}

#[tokio::test]
async fn admin_reset_refuses_self_unknown_users_and_non_admins() {
    let user = enrolled_member().await;
    let admin = admin_token().await;
    let cli = api(unique_ip());

    let resp = cli
        .post(format!("/api/v1/mfa/admin-reset/{DEFAULT_ADMIN_USER_ID}"))
        .header("Authorization", bearer(&admin))
        .send()
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);

    let resp = cli
        .post("/api/v1/mfa/admin-reset/1")
        .header("Authorization", bearer(&admin))
        .send()
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);

    let (_, other) = create_member().await;
    let member = login(&other, PASSWORD).await;
    let resp = cli
        .post(format!("/api/v1/mfa/admin-reset/{}", user.id))
        .header("Authorization", bearer(&member))
        .send()
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    assert_eq!(status(&user.token).await["enabled"], true);
}
