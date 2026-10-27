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
struct CreateOAuth2Payload {
    client_id: String,
    client_secret: String,
    auth_url: String,
    token_url: String,
    redirect_uri: String,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct OAuth2Page {
    items: Vec<OAuth2Config>,
}

#[derive(Debug, Deserialize)]
struct OAuth2Config {
    id: u64,
    client_id: String,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct UpdateOAuth2Payload {
    enabled: Option<bool>,
}

#[tokio::test]
async fn oauth2_crud() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    // Create
    let create = CreateOAuth2Payload {
        client_id: "test-client-id".into(),
        client_secret: "test-client-secret".into(),
        auth_url: "https://provider.example.com/auth".into(),
        token_url: "https://provider.example.com/token".into(),
        redirect_uri: "http://localhost/callback".into(),
        enabled: false,
    };
    let resp = cli
        .post("/api/v1/oauth2")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&create)
        .send()
        .await;
    resp.assert_status_is_ok();

    // List
    let resp = cli
        .get("/api/v1/oauth2-list")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
    let list: OAuth2Page = resp.json().await.value().deserialize();
    let created = list
        .items
        .iter()
        .find(|c| c.client_id == "test-client-id")
        .expect("created OAuth2 config is listed");
    assert!(!created.enabled);
    let id = created.id;

    // Get by ID
    let resp = cli
        .get(&format!("/api/v1/oauth2/{}", id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();

    // Update
    let update = UpdateOAuth2Payload {
        enabled: Some(true),
    };
    let resp = cli
        .post(&format!("/api/v1/oauth2/{}", id))
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&update)
        .send()
        .await;
    resp.assert_status_is_ok();

    let resp = cli
        .get(format!("/api/v1/oauth2/{}", id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
    let config: OAuth2Config = resp.json().await.value().deserialize();
    assert!(config.enabled, "update must enable the config");

    // Delete
    let resp = cli
        .delete(&format!("/api/v1/oauth2/{}", id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}
