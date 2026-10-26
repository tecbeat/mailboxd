//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// Part of mailboxd, a fork of the Bichon email archiving project.
// Modifications by tecbeat, 2026.

use poem::test::TestClient;
use serde::{Deserialize, Serialize};

use super::{admin_token, build_api_route, setup};

fn api_client(route: impl poem::Endpoint) -> TestClient<impl poem::Endpoint> {
    TestClient::new(route).default_header("X-Forwarded-For", "127.0.0.1")
}

#[derive(Debug, Serialize)]
struct CreateUserPayload {
    username: String,
    email: String,
    password: String,
    global_roles: Vec<u64>,
    account_access_map: std::collections::BTreeMap<u64, u64>,
}

#[derive(Debug, Deserialize)]
struct UserView {
    id: u64,
    username: String,
    email: String,
}

#[derive(Debug, Serialize)]
struct UpdateUserPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

const ADMIN_ROLE_ID: u64 = 100_000_000_000_000;
const MEMBER_ROLE_ID: u64 = 100_200_000_000_000;

#[tokio::test]
async fn user_crud() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    // Create
    let create_payload = CreateUserPayload {
        username: "testuser1".into(),
        email: "testuser1@example.com".into(),
        password: "testpass123".into(),
        global_roles: vec![ADMIN_ROLE_ID],
        account_access_map: Default::default(),
    };
    let resp = cli
        .post("/api/v1/users")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&create_payload)
        .send()
        .await;
    resp.assert_status_is_ok();
    let user: UserView = resp.json().await.value().deserialize();
    assert_eq!(user.username, "testuser1");
    assert_eq!(user.email, "testuser1@example.com");
    let user_id = user.id;

    // List users
    let resp = cli
        .get("/api/v1/list-users")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();

    // Get current user
    let resp = cli
        .get("/api/v1/current-user")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();

    // Update
    let update = UpdateUserPayload {
        description: Some("Test description".into()),
    };
    let resp = cli
        .post(&format!("/api/v1/users/{}", user_id))
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&update)
        .send()
        .await;
    resp.assert_status_is_ok();

    // Delete
    let resp = cli
        .delete(&format!("/api/v1/users/{}", user_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}

#[tokio::test]
async fn create_user_with_invalid_data_fails() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    // Username too short (min 3)
    let payload = CreateUserPayload {
        username: "ab".into(),
        email: "valid@example.com".into(),
        password: "testpass123".into(),
        global_roles: vec![ADMIN_ROLE_ID],
        account_access_map: Default::default(),
    };
    let resp = cli
        .post("/api/v1/users")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&payload)
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "short username should fail");

    // Password too short (min 8)
    let payload = CreateUserPayload {
        username: "validuser".into(),
        email: "valid@example.com".into(),
        password: "short".into(),
        global_roles: vec![ADMIN_ROLE_ID],
        account_access_map: Default::default(),
    };
    let resp = cli
        .post("/api/v1/users")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&payload)
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "short password should fail");
}

#[tokio::test]
async fn cannot_delete_default_admin() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let resp = cli
        .delete("/api/v1/users/100000000000000")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "deleting admin should fail");
}

#[derive(Debug, Deserialize)]
struct Notifications {
    default_admin_password: bool,
}

async fn login(username: &str, password: &str) -> String {
    let route = poem::Route::new().at("/api/login", poem::post(crate::rest::public::login::login));
    let resp = TestClient::new(route)
        .post("/api/login")
        .body_json(&super::LoginPayload {
            username: username.into(),
            password: password.into(),
        })
        .send()
        .await;
    resp.assert_status_is_ok();
    let result: super::LoginResult = resp.json().await.value().deserialize();
    assert!(result.success, "login as {username} failed");
    result.access_token.unwrap()
}

/// The test admin keeps the initial password, so it is reminded; a regular
/// member must not learn that the admin password is the public default.
#[tokio::test]
async fn default_admin_password_is_only_reported_to_user_managers() {
    setup().await;
    let token = admin_token().await;
    let cli = api_client(build_api_route());
    let default_password = |token: String| {
        let cli = &cli;
        async move {
            let resp = cli
                .get("/api/v1/notifications")
                .header("Authorization", &format!("Bearer {}", token))
                .send()
                .await;
            resp.assert_status_is_ok();
            let notifications: Notifications = resp.json().await.value().deserialize();
            notifications.default_admin_password
        }
    };

    assert!(default_password(token.clone()).await);

    let resp = cli
        .post("/api/v1/users")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&CreateUserPayload {
            username: "member-notifications".into(),
            email: "member-notifications@example.com".into(),
            password: "member-pass-123".into(),
            global_roles: vec![MEMBER_ROLE_ID],
            account_access_map: Default::default(),
        })
        .send()
        .await;
    resp.assert_status_is_ok();
    let member: UserView = resp.json().await.value().deserialize();

    let member_token = login("member-notifications", "member-pass-123").await;
    assert!(!default_password(member_token).await);

    cli.delete(&format!("/api/v1/users/{}", member.id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .assert_status_is_ok();
}
