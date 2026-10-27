//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project.
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

//! `POST /api/v1/repair-message` and `POST /api/v1/delete-messages` against a
//! stored message.
//!
//! Runs as its own test binary: the index writer is a background task bound
//! to the runtime that first touches it, so storing a message and calling
//! the API must share one runtime. The writer only commits on a timer, a
//! size threshold, or shutdown; the test triggers the shutdown commit once
//! the message is stored. Deletions commit directly and still work
//! afterwards.

use std::collections::HashSet;
use std::time::Duration;

use mailboxd_core::{
    account::migration::{Account, AccountType},
    archive::imap::mailbox::MailBox,
    common::signal::SIGNAL_MANAGER,
    context::Initialize,
    database::{manager::DB_MANAGER, upsert_impl},
    envelope::extractor::{extract_envelope_from_eml, ExtractOutcome},
    message::search::{search_messages_impl, EmailSearchFilter, EmailSearchRequest},
    store::tantivy::envelope::ENVELOPE_MANAGER,
    users::manager::UserManager,
};
use mailboxd_server::{
    common::{auth::ApiGuard, error::ErrorCapture},
    rest::{api::create_openapi_service, public::login::login},
};
use poem::{test::TestClient, EndpointExt, Route};

/// Ids used by no other test.
const ACCOUNT_ID: u64 = 63_100_000_000_002;
const MAILBOX_ID: u64 = 63_100_000_000_003;

const EML: &[u8] = b"From: sender@example.com\r\n\
To: archive@example.com\r\n\
Subject: Delete via API\r\n\
Message-ID: <delete-via-api@example.com>\r\n\
Date: Mon, 1 Jan 2024 10:00:00 +0000\r\n\
\r\n\
Body\r\n";

fn stored_envelope_ids() -> Vec<String> {
    search_messages_impl(
        Some(HashSet::from([ACCOUNT_ID])),
        EmailSearchRequest {
            filter: EmailSearchFilter::default(),
            page: 1,
            page_size: 10,
            ..Default::default()
        },
    )
    .unwrap()
    .items
    .into_iter()
    .map(|envelope| envelope.id)
    .collect()
}

async fn store_message() -> String {
    // Leftovers of an aborted earlier run would make the message a duplicate.
    ENVELOPE_MANAGER
        .delete_account_envelopes(ACCOUNT_ID)
        .await
        .unwrap();
    upsert_impl(
        DB_MANAGER.db(),
        Account {
            id: ACCOUNT_ID,
            email: "archive@example.com".into(),
            account_type: AccountType::NoSync,
            enabled: true,
            ..Default::default()
        },
    )
    .unwrap();
    MailBox::batch_upsert(&[MailBox {
        id: MAILBOX_ID,
        account_id: ACCOUNT_ID,
        name: "INBOX".into(),
        ..Default::default()
    }])
    .unwrap();
    let outcome = extract_envelope_from_eml(EML, ACCOUNT_ID, MAILBOX_ID)
        .await
        .unwrap();
    assert!(matches!(outcome, ExtractOutcome::Imported));

    // Let the index writer pick the document up, then force its commit.
    tokio::time::sleep(Duration::from_millis(500)).await;
    SIGNAL_MANAGER.trigger_shutdown();
    ENVELOPE_MANAGER.shutdown().await;
    for _ in 0..100 {
        if let [id] = stored_envelope_ids().as_slice() {
            return id.clone();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the stored message never became searchable");
}

async fn admin_token() -> String {
    let cli = TestClient::new(Route::new().at("/api/login", poem::post(login)));
    let resp = cli
        .post("/api/login")
        .body_json(&serde_json::json!({ "username": "admin", "password": "admin@mailboxd" }))
        .send()
        .await;
    resp.assert_status_is_ok();
    let body: serde_json::Value = resp.json().await.value().deserialize();
    body["access_token"]
        .as_str()
        .expect("admin login returns a token")
        .to_string()
}

#[test]
fn delete_messages_reports_the_number_of_deleted_messages() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        UserManager::initialize().await.unwrap();
        let envelope_id = store_message().await;
        let token = admin_token().await;

        let api = Route::new()
            .nest_no_strip("/api/v1", create_openapi_service())
            .with(ApiGuard)
            .with(ErrorCapture);
        let cli = TestClient::new(api).default_header("X-Forwarded-For", "127.0.0.1");
        let auth = format!("Bearer {token}");

        // An intact message has nothing to repair; such repairs are not
        // recorded in the repair history.
        let resp = cli
            .post(format!("/api/v1/repair-message/{ACCOUNT_ID}/{envelope_id}"))
            .header("Authorization", &auth)
            .send()
            .await;
        resp.assert_status_is_ok();
        let report: serde_json::Value = resp.json().await.value().deserialize();
        assert_eq!(
            (&report["status"], &report["envelope_id"]),
            (
                &serde_json::json!("complete"),
                &serde_json::json!(envelope_id)
            )
        );
        let resp = cli
            .get("/api/v1/repair-reports")
            .header("Authorization", &auth)
            .query("account_id", &ACCOUNT_ID)
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.assert_json(serde_json::json!([])).await;

        let delete = || {
            cli.post("/api/v1/delete-messages")
                .header("Authorization", format!("Bearer {token}"))
                .body_json(&serde_json::json!({ ACCOUNT_ID.to_string(): [envelope_id] }))
        };

        let resp = delete().send().await;
        resp.assert_status_is_ok();
        resp.assert_json(serde_json::json!({ "deleted": 1 })).await;
        assert!(stored_envelope_ids().is_empty());

        // The message is gone, so a repeated request deletes nothing.
        let resp = delete().send().await;
        resp.assert_status(http::StatusCode::NOT_FOUND);
    });
}
