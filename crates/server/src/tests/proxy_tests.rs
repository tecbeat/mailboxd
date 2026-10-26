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
struct Proxy {
    id: u64,
    url: String,
}

#[tokio::test]
async fn proxy_crud() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    // Create proxy
    let resp = cli
        .post("/api/v1/proxy")
        .header("Authorization", &format!("Bearer {}", token))
        .content_type("text/plain")
        .body("socks5://127.0.0.1:1080")
        .send()
        .await;
    resp.assert_status_is_ok();

    // List proxies
    let resp = cli
        .get("/api/v1/list-proxy")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
    let proxies: Vec<Proxy> = resp.json().await.value().deserialize();
    // Other tests share the database, so find the proxy created here.
    let proxy_id = proxies
        .iter()
        .find(|p| p.url == "socks5://127.0.0.1:1080")
        .expect("created proxy is listed")
        .id;

    // Get single proxy
    let resp = cli
        .get(&format!("/api/v1/proxy/{}", proxy_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();

    // Update proxy
    let resp = cli
        .post(&format!("/api/v1/proxy/{}", proxy_id))
        .header("Authorization", &format!("Bearer {}", token))
        .content_type("text/plain")
        .body("socks5://192.168.1.1:1080")
        .send()
        .await;
    resp.assert_status_is_ok();

    let resp = cli
        .get(format!("/api/v1/proxy/{}", proxy_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
    let proxy: Proxy = resp.json().await.value().deserialize();
    assert_eq!(proxy.url, "socks5://192.168.1.1:1080");

    // Delete proxy
    let resp = cli
        .delete(&format!("/api/v1/proxy/{}", proxy_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}
