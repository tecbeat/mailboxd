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

use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD, Engine};
use http::StatusCode;
use poem::test::{TestClient, TestResponse};
use serde::Deserialize;

use super::{admin_token, setup};
use crate::rest::{build_routes, metrics::MetricsConfig};

const PASSWORD: &str = "scrape-secret";

fn client(base_url: &str, metrics: MetricsConfig) -> TestClient<impl poem::Endpoint> {
    TestClient::new(build_routes(base_url, metrics)).default_header("X-Forwarded-For", "127.0.0.1")
}

fn open() -> MetricsConfig {
    MetricsConfig {
        enabled: true,
        password: None,
    }
}

fn protected() -> MetricsConfig {
    MetricsConfig {
        enabled: true,
        password: Some(PASSWORD.into()),
    }
}

async fn body(resp: TestResponse) -> String {
    resp.0.into_body().into_string().await.unwrap()
}

/// Names of the metric families announced by `# TYPE` lines.
fn families(text: &str) -> Vec<&str> {
    text.lines()
        .filter_map(|line| line.strip_prefix("# TYPE "))
        .filter_map(|rest| rest.split_whitespace().next())
        .collect()
}

#[tokio::test]
async fn metrics_are_not_served_when_disabled() {
    setup().await;
    let cli = client(
        "/",
        MetricsConfig {
            enabled: false,
            password: None,
        },
    );

    cli.get("/metrics")
        .send()
        .await
        .assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn open_endpoint_serves_the_text_format() {
    setup().await;
    let cli = client("/", open());

    // One request before the scrape so the HTTP families have a series.
    cli.get("/api/status").send().await.assert_status_is_ok();

    let resp = cli.get("/metrics").send().await;
    resp.assert_status_is_ok();
    resp.assert_header("content-type", "text/plain; version=0.0.4");
    let text = body(resp).await;
    let families = families(&text);
    for family in [
        "mailboxd_build_info",
        "mailboxd_start_time_seconds",
        "mailboxd_http_requests_total",
        "mailboxd_http_request_duration_seconds",
        "mailboxd_http_requests_in_flight",
        "mailboxd_logins_total",
        "mailboxd_retention_deleted_messages_total",
        "mailboxd_dedup_removed_total",
    ] {
        assert!(
            families.contains(&family),
            "missing family {family}:\n{text}"
        );
    }
}

#[tokio::test]
async fn http_metrics_use_route_templates() {
    setup().await;
    let token = admin_token().await;
    let cli = client("/", open());

    cli.get("/api/v1/account/4711")
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;

    let text = body(cli.get("/metrics").send().await).await;
    assert!(
        text.contains(r#"route="/api/v1/account/{account_id}""#),
        "expected the route template:\n{text}"
    );
    assert!(
        !text.contains("/api/v1/account/4711"),
        "raw path leaked:\n{text}"
    );
}

#[tokio::test]
async fn password_is_required_when_set() {
    setup().await;
    let cli = client("/", protected());

    let resp = cli.get("/metrics").send().await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
    assert!(resp.0.headers().contains_key("www-authenticate"));

    cli.get("/metrics")
        .header("Authorization", "Bearer wrong")
        .send()
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    let wrong_basic = STANDARD.encode("prometheus:wrong");
    cli.get("/metrics")
        .header("Authorization", format!("Basic {wrong_basic}"))
        .send()
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bearer_and_basic_auth_are_accepted() {
    setup().await;
    let cli = client("/", protected());

    cli.get("/metrics")
        .header("Authorization", format!("Bearer {PASSWORD}"))
        .send()
        .await
        .assert_status_is_ok();

    let basic = STANDARD.encode(format!("any-user:{PASSWORD}"));
    cli.get("/metrics")
        .header("Authorization", format!("Basic {basic}"))
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn endpoint_honours_the_base_url() {
    setup().await;
    let cli = client("/mailboxd", open());

    cli.get("/mailboxd/metrics")
        .send()
        .await
        .assert_status_is_ok();
}

#[derive(Debug, Deserialize)]
struct CreatedAccount {
    id: u64,
}

#[tokio::test]
async fn account_series_follow_the_account_lifecycle() {
    setup().await;
    let token = admin_token().await;
    let cli = client("/", open());
    let email = "metrics-lifecycle@example.com";

    let resp = cli
        .post("/api/v1/account")
        .header("Authorization", format!("Bearer {token}"))
        .body_json(&serde_json::json!({
            "email": email,
            "enabled": false,
            "account_type": "NoSync",
            "use_dangerous": false,
        }))
        .send()
        .await;
    resp.assert_status_is_ok();
    let account: CreatedAccount = resp.json().await.value().deserialize();
    let series = format!(
        r#"mailboxd_account_emails{{account="{email}",account_id="{}"}}"#,
        account.id
    );

    mailboxd_core::metrics::refresh().await.unwrap();
    let text = body(cli.get("/metrics").send().await).await;
    assert!(text.contains(&series), "missing {series}:\n{text}");

    cli.delete(format!("/api/v1/account/{}", account.id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .assert_status_is_ok();

    // Deletion finishes in a background task.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let text = body(cli.get("/metrics").send().await).await;
        if !text.contains(&format!(r#"account_id="{}""#, account.id)) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "account series still present:\n{text}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
