//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// Part of mailboxd, a fork of the Bichon email archiving project.
// Modifications by tecbeat, 2026.

use poem::test::TestClient;
use serde::Deserialize;

use super::{admin_token, build_api_route, setup};

fn api_client(route: impl poem::Endpoint) -> TestClient<impl poem::Endpoint> {
    TestClient::new(route).default_header("X-Forwarded-For", "127.0.0.1")
}

#[derive(Debug, Deserialize)]
struct SystemConfig {
    mailboxd_root_dir: String,
    mailboxd_http_port: i32,
}

#[derive(Debug, Deserialize)]
struct DashboardStats {
    system_version: String,
}

#[tokio::test]
async fn get_system_configurations() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let resp = cli
        .get("/api/v1/system-configurations")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
    let config: SystemConfig = resp.json().await.value().deserialize();
    assert!(!config.mailboxd_root_dir.is_empty());
    assert!(config.mailboxd_http_port > 0);
}

#[tokio::test]
async fn get_dashboard_stats_returns_data() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let resp = cli
        .get("/api/v1/dashboard-stats")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;

    resp.assert_status_is_ok();
    let stats: DashboardStats = resp.json().await.value().deserialize();
    assert!(!stats.system_version.is_empty());
}

#[tokio::test]
async fn list_proxy_returns_array() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let resp = cli
        .get("/api/v1/list-proxy")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}

#[tokio::test]
async fn list_roles_returns_builtins() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let resp = cli
        .get("/api/v1/list-roles")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}

#[tokio::test]
async fn minimal_user_list_works() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let resp = cli
        .get("/api/v1/minimal-user-list")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}

#[tokio::test]
async fn list_account_roles_works() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let resp = cli
        .get("/api/v1/list-account-roles")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}
