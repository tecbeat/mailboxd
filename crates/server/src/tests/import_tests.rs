//
// Copyright (c) 2026 tecbeat
//
// Part of mailboxd, a fork of the Bichon email archiving project.
// Modifications by tecbeat, 2026.

use base64::{engine::general_purpose::URL_SAFE, Engine};
use poem::test::TestClient;
use serde_json::{json, Value};

use super::{admin_token, build_api_route, setup};

fn unique() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// A message with a unique Message-ID, dated `date` (RFC 2822).
fn raw_eml(date: &str) -> String {
    let unique = unique();
    format!(
        "From: sender@example.com\r\nTo: rcpt@example.com\r\nSubject: retention\r\n\
         Message-ID: <{unique}@import-api.test>\r\nDate: {date}\r\n\r\nbody\r\n"
    )
}

/// `raw_eml` in base64, as the batch import expects it.
fn eml(date: &str) -> String {
    URL_SAFE.encode(raw_eml(date))
}

const OLD: &str = "Mon, 1 Jan 2001 10:00:00 +0000";

/// Create an enabled no-sync account keeping 30 days of mail; returns its id.
async fn retained_account<T: poem::Endpoint>(cli: &TestClient<T>, auth: &str) -> u64 {
    let resp = cli
        .post("/api/v1/account")
        .header("Authorization", auth)
        .body_json(&json!({
            "email": format!("import-retention-{}@example.com", unique()),
            "enabled": true,
            "account_type": "NoSync",
            "use_dangerous": false,
            "retention_days": 30,
        }))
        .send()
        .await;
    resp.assert_status_is_ok();
    let account: Value = resp.json().await.value().deserialize();
    account["id"].as_u64().unwrap()
}

async fn delete_account<T: poem::Endpoint>(cli: &TestClient<T>, auth: &str, id: u64) {
    cli.delete(format!("/api/v1/account/{id}"))
        .header("Authorization", auth)
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn a_batch_import_reports_and_records_mail_outside_the_retention_window() {
    setup().await;
    let token = admin_token().await;
    let cli = TestClient::new(build_api_route()).default_header("X-Forwarded-For", "127.0.0.31");
    let auth = format!("Bearer {token}");
    let account_id = retained_account(&cli, &auth).await;

    let resp = cli
        .post("/api/v1/import")
        .header("Authorization", &auth)
        .body_json(&json!({
            "account_id": account_id,
            "mail_folder": "Imported",
            "emls": [eml(OLD)],
        }))
        .send()
        .await;
    resp.assert_status_is_ok();
    let result: Value = resp.json().await.value().deserialize();
    assert_eq!(
        (
            &result["success"],
            &result["outside_retention"],
            &result["failed"]
        ),
        (&json!(0), &json!(1), &json!(0))
    );

    let resp = cli
        .get("/api/v1/import-history")
        .header("Authorization", &auth)
        .send()
        .await;
    resp.assert_status_is_ok();
    let history: Vec<Value> = resp.json().await.value().deserialize();
    let entry = history
        .iter()
        .find(|entry| entry["account_id"].as_u64() == Some(account_id))
        .expect("the batch import is in the history");
    assert_eq!(entry["status"], "completed");
    assert_eq!(entry["outside_retention"], 1);

    delete_account(&cli, &auth, account_id).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_uploaded_file_reports_mail_outside_the_retention_window() {
    setup().await;
    let token = admin_token().await;
    let cli = TestClient::new(build_api_route()).default_header("X-Forwarded-For", "127.0.0.32");
    let auth = format!("Bearer {token}");
    let account_id = retained_account(&cli, &auth).await;

    let resp = cli
        .post("/api/v1/upload-import")
        .header("Authorization", &auth)
        .content_type("application/octet-stream")
        .query("account_id", &account_id)
        .query("mail_folder", &"Uploaded")
        .query("file_name", &"old.eml")
        .body(raw_eml(OLD))
        .send()
        .await;
    resp.assert_status_is_ok();
    let initial: Value = resp.json().await.value().deserialize();
    assert_eq!(initial["status"], "Pending");
    assert_eq!(initial["outside_retention"], 0);
    let import_id = initial["import_id"].as_str().unwrap().to_string();

    let mut progress = Value::Null;
    for _ in 0..100 {
        let resp = cli
            .get(format!("/api/v1/import-progress/{import_id}"))
            .header("Authorization", &auth)
            .send()
            .await;
        resp.assert_status_is_ok();
        progress = resp.json().await.value().deserialize();
        if progress["status"] != "Pending" && progress["status"] != "Processing" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert_eq!(progress["status"], "Completed", "{progress}");
    assert_eq!(
        (&progress["success"], &progress["outside_retention"]),
        (&json!(0), &json!(1))
    );

    delete_account(&cli, &auth, account_id).await;
}
