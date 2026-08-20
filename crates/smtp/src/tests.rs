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
use std::error::Error;

use lettre::address::Envelope;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{Message, SmtpTransport, Transport};

#[tokio::test]
#[ignore = "integration: requires a live mailboxd SMTP server on 127.0.0.1:2525"]
async fn test_smtp_archiving_flow() {
    let email = Message::builder()
        .from("tester@mailboxd.local".parse().unwrap())
        .to("archive@mailboxd.local".parse().unwrap())
        .subject("Integration Test")
        .body(String::from("Checking if mailboxd saves this!"))
        .unwrap();

    let envelope = Envelope::new(
        Some("sender@example.com".parse().unwrap()),
        vec!["placeholder@example.com".parse().unwrap()], // the email of a mailboxd account
    )
    .unwrap();
    let mailer = SmtpTransport::builder_dangerous("127.0.0.1")
        .port(2525)
        .tls(Tls::None)
        .build();

    let result = mailer.send_raw(&envelope, &email.formatted());
    assert!(
        result.is_ok(),
        "SMTP delivery should succeed, got: {:?}",
        result.err()
    );
}

#[test]
#[ignore = "integration: requires a live mailboxd SMTP server on 127.0.0.1:2525"]
fn test_mailboxd_smtp_logic() -> Result<(), Box<dyn Error>> {
    let smtp_host = "127.0.0.1";
    let smtp_port = 2525;

    println!("--- Testing STARTTLS Upgrade ---");

    let tls_parameters = TlsParameters::builder(smtp_host.to_string())
        .dangerous_accept_invalid_certs(true)
        .build()?;

    let mailer = SmtpTransport::starttls_relay(smtp_host)?
        .port(smtp_port)
        .tls(Tls::Required(tls_parameters))
        .authentication(vec![Mechanism::Login, Mechanism::Plain])
        .credentials(Credentials::new(
            "test_user".to_string(),
            "hP1Z4ZBs4IjdXtjbImFoX9kM".to_string(),
        ))
        .build();

    // If RCPT TO is not explicitly specified in the envelope, the addresses in the 'To' header
    // will be treated as envelope recipients. mailboxd enforces a single-recipient policy per
    // transaction; if multiple recipients are detected, it will reject with:
    // "452 4.5.3 Too many recipients, try again in a new transaction".
    let email = Message::builder()
        .from("sender@mailboxd.dev".parse()?)
        .to("placeholder@example.com".parse()?)
        .subject("TLS Test")
        .body(String::from("Hello mailboxd with TLS!"))?;

    let result = mailer.send(&email);
    assert!(
        result.is_ok(),
        "STARTTLS encryption or Auth failed: {:?}",
        result.err()
    );
    println!("SUCCESS: TLS upgrade and mail delivery worked.");
    Ok(())
}
