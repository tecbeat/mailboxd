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

//! Maintenance mode: when a startup check fails before the web server can
//! start, the process stays alive and answers every HTTP request with a 503
//! page instead of exiting.
//!
//! Exiting is the worst option under Docker: the container restarts in a
//! loop and `docker exec` cannot reach it, which is the practical way to run
//! `mailboxd-admin`. In maintenance mode `docker exec -it <container>
//! mailboxd-admin` works, the page says what to do, and `/api/status`
//! answers 503, so the healthcheck reports the container unhealthy.
//!
//! The page is public, so it only carries fixed instructions; the error
//! itself goes to the log.

use std::sync::Arc;
use std::time::Duration;

use mailboxd_core::{
    common::signal::{SignalManager, SIGNAL_MANAGER},
    context::Initialize,
    mailboxd_version,
    settings::cli::SETTINGS,
};
use poem::{
    listener::{Listener, TcpListener},
    Endpoint, Request, Response, Route, Server,
};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

/// How often the "still in maintenance mode" reminder is logged, so
/// `docker logs --tail` keeps showing the reason for the outage.
const REMINDER_INTERVAL: Duration = Duration::from_secs(600);

const WIKI_URL: &str = "https://git.teccave.de/tecbeat/mailboxd/-/wikis/home";

/// Why the server is in maintenance mode, and what the operator can do.
pub struct Maintenance {
    pub heading: &'static str,
    pub steps: &'static [&'static str],
}

pub const RESTORE_FAILED: Maintenance = Maintenance {
    heading: "Restoring a backup failed",
    steps: &[
        "A staged backup restore could not be applied on startup.",
        "Check the service logs (e.g. `docker logs <mailboxd-container>`) for the cause.",
        "Fix the underlying problem (disk space, permissions, a damaged backup), then restart the service.",
    ],
};

pub const MIGRATION_FAILED: Maintenance = Maintenance {
    heading: "Migration failed",
    steps: &[
        "Your data was created by an older version and could not be migrated automatically.",
        "Check the service logs (e.g. `docker logs <mailboxd-container>`) for the cause.",
        "To migrate interactively, run `docker exec -it <mailboxd-container> mailboxd-admin` (other installs: `./mailboxd-admin`) and choose the migration matching your old version.",
        "Migrations are non-destructive: legacy files are never modified.",
        "Afterwards, restart the service (e.g. `docker restart <mailboxd-container>`).",
    ],
};

pub const MIGRATION_REQUIRED: Maintenance = Maintenance {
    heading: "Migration required",
    steps: &[
        "Your data was created by an older version and must be migrated before use.",
        "Run `docker exec -it <mailboxd-container> mailboxd-admin` (other installs: `./mailboxd-admin`) and choose the migration matching your old version.",
        "Migrations are non-destructive: legacy files are never modified.",
        "Afterwards, restart the service (e.g. `docker restart <mailboxd-container>`).",
    ],
};

pub const LAYOUT_CHECK_FAILED: Maintenance = Maintenance {
    heading: "Startup check failed",
    steps: &[
        "Checking the data layout failed.",
        "Check the service logs (e.g. `docker logs <mailboxd-container>`) for the cause.",
        "Fix the underlying problem, then restart the service.",
    ],
};

pub const REINDEX_FAILED: Maintenance = Maintenance {
    heading: "Rebuilding the search index failed",
    steps: &[
        "The search index is in an older format and could not be rebuilt automatically. The live index was left untouched.",
        "Check the service logs (e.g. `docker logs <mailboxd-container>`) for the cause, e.g. too little free disk space.",
        "Fix the underlying problem, or rebuild the index interactively with `docker exec -it <mailboxd-container> mailboxd-admin` (other installs: `./mailboxd-admin`) and choose \"Rebuild Search Index\".",
        "Afterwards, restart the service (e.g. `docker restart <mailboxd-container>`).",
    ],
};

/// Serve the maintenance page and block until SIGTERM/SIGINT.
///
/// Callers reach this before `initialize()`, so nothing here may touch
/// databases, TLS material or anything beyond the bind address. If the
/// HTTP server cannot start (e.g. the port is taken), the process stays
/// alive anyway and waits for a shutdown signal.
pub async fn serve_maintenance(maintenance: &Maintenance) {
    // `initialize()` normally installs the signal watcher; start it here so
    // `docker stop` shuts down gracefully instead of waiting for SIGKILL.
    if let Err(e) = SignalManager::initialize().await {
        error!("Failed to install signal handlers: {e:#?}");
    }

    let bind_ip = SETTINGS
        .mailboxd_bind_ip
        .clone()
        .unwrap_or("0.0.0.0".into());
    let port = SETTINGS.mailboxd_http_port as u16;
    let heading = maintenance.heading;

    warn!("Entering maintenance mode ({heading}): serving a 503 page on {bind_ip}:{port}");
    if SETTINGS.mailboxd_enable_rest_https {
        warn!(
            "HTTPS is enabled, but the maintenance page is served over plain HTTP: \
             TLS is only set up once the startup checks pass."
        );
    }

    run_maintenance(
        TcpListener::bind((bind_ip, port)),
        maintenance,
        REMINDER_INTERVAL,
        || SIGNAL_MANAGER.subscribe(),
    )
    .await;
}

/// Serves the page on `listener` until a shutdown signal arrives. If the
/// server fails, waits for the signal anyway.
async fn run_maintenance<L: Listener + 'static>(
    listener: L,
    maintenance: &Maintenance,
    reminder_interval: Duration,
    subscribe: impl Fn() -> broadcast::Receiver<()>,
) {
    let heading = maintenance.heading;
    let reminder = tokio::spawn(async move {
        let start = tokio::time::Instant::now() + reminder_interval;
        let mut ticker = tokio::time::interval_at(start, reminder_interval);
        loop {
            ticker.tick().await;
            warn!("Maintenance mode still active: {heading}");
        }
    });

    let mut shutdown_rx = subscribe();
    let server = Server::new(listener)
        .name("mailboxd maintenance")
        .idle_timeout(Duration::from_secs(60))
        .run_with_graceful_shutdown(
            maintenance_app(maintenance),
            async move {
                let _ = shutdown_rx.recv().await;
            },
            Some(Duration::from_secs(5)),
        );

    match server.await {
        Ok(()) => info!("Maintenance server stopped."),
        Err(e) => {
            error!("Maintenance HTTP server failed: {e:#?}");
            error!(
                "Staying alive so `docker exec -it <container> mailboxd-admin` keeps \
                 working; stop the process to exit."
            );
            let mut rx = subscribe();
            let _ = rx.recv().await;
        }
    }
    reminder.abort();
}

/// Answers every path, including `/api/status`, with the 503 page.
fn maintenance_app(maintenance: &Maintenance) -> Route {
    let page = MaintenancePage {
        html: Arc::new(render_page(maintenance)),
    };
    Route::new().at("/", page.clone()).at("/*", page)
}

#[derive(Clone)]
struct MaintenancePage {
    html: Arc<String>,
}

impl Endpoint for MaintenancePage {
    type Output = Response;

    async fn call(&self, _req: Request) -> poem::Result<Self::Output> {
        Ok(Response::builder()
            .status(http::StatusCode::SERVICE_UNAVAILABLE)
            .header("Retry-After", "300")
            .header("Cache-Control", "no-store")
            .content_type("text/html; charset=utf-8")
            .body(self.html.as_str().to_owned()))
    }
}

fn render_page(maintenance: &Maintenance) -> String {
    let steps = maintenance
        .steps
        .iter()
        .map(|step| format!("<li>{}</li>", render_step(step)))
        .collect::<Vec<_>>()
        .join("\n      ");
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>mailboxd — {heading}</title>
<style>
  body {{ margin: 0; font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
         background: #f5f5f5; color: #24292f; }}
  main {{ max-width: 640px; margin: 8vh auto; padding: 32px 40px;
         background: #fff; border: 1px solid #d0d7de; border-radius: 8px; }}
  h1 {{ margin: 0 0 4px; font-size: 28px; }}
  h2 {{ margin: 0 0 16px; font-size: 18px; color: #b35900; }}
  ol {{ padding-left: 20px; line-height: 1.7; }}
  li {{ margin-bottom: 6px; }}
  code {{ background: #f0f2f4; padding: 1px 6px; border-radius: 4px; font-size: 0.92em; }}
  .footer {{ margin-top: 20px; font-size: 12px; color: #656d76; }}
  @media (prefers-color-scheme: dark) {{
    body {{ background: #161a1d; color: #e6edf3; }}
    main {{ background: #1f2428; border-color: #3d444d; }}
    code {{ background: #2d333b; }}
    .footer {{ color: #9198a1; }}
  }}
</style>
</head>
<body>
  <main>
    <h1>mailboxd</h1>
    <h2>{heading}</h2>
    <p>The mailboxd service is not running. Follow the steps below, then restart the service.</p>
    <ol>
      {steps}
    </ol>
    <p class="footer">Version {version} &middot; Documentation: <a href="{wiki}">{wiki}</a></p>
  </main>
</body>
</html>
"#,
        heading = escape_html(maintenance.heading),
        version = escape_html(mailboxd_version!()),
        wiki = WIKI_URL,
    )
}

/// Escapes a step and renders `backticked` parts as `<code>`.
fn render_step(step: &str) -> String {
    escape_html(step)
        .split('`')
        .enumerate()
        .map(|(i, part)| {
            if i % 2 == 1 {
                format!("<code>{part}</code>")
            } else {
                part.to_string()
            }
        })
        .collect()
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use poem::test::TestClient;

    #[test]
    fn steps_are_escaped_and_code_is_marked_up() {
        assert_eq!(
            render_step("Run `docker exec -it <c> mailboxd-admin` & restart"),
            "Run <code>docker exec -it &lt;c&gt; mailboxd-admin</code> &amp; restart"
        );
    }

    #[test]
    fn escape_covers_all_entities() {
        assert_eq!(
            escape_html(r#"<a href="x">&"</a> "#),
            r#"&lt;a href=&quot;x&quot;&gt;&amp;&quot;&lt;/a&gt; "#
        );
    }

    #[test]
    fn page_names_the_problem_and_every_step() {
        for maintenance in [
            RESTORE_FAILED,
            MIGRATION_FAILED,
            MIGRATION_REQUIRED,
            LAYOUT_CHECK_FAILED,
            REINDEX_FAILED,
        ] {
            let html = render_page(&maintenance);
            assert!(html.contains(&format!("<h2>{}</h2>", maintenance.heading)));
            for step in maintenance.steps {
                assert!(html.contains(&format!("<li>{}</li>", render_step(step))));
            }
        }
    }

    #[tokio::test]
    async fn every_path_answers_503_so_the_healthcheck_fails() {
        let client = TestClient::new(maintenance_app(&MIGRATION_REQUIRED));
        for path in ["/", "/api/status", "/api/v1/accounts", "/assets/index.js"] {
            let response = client.get(path).send().await;
            response.assert_status(http::StatusCode::SERVICE_UNAVAILABLE);
            response.assert_header("Retry-After", "300");
            response.assert_content_type("text/html; charset=utf-8");
            response.assert_text(render_page(&MIGRATION_REQUIRED)).await;
        }
    }

    /// A free local port. The listener is dropped again, so the port is only
    /// very likely still free when the server binds it.
    fn free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    async fn get_status_line(port: u16) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for _ in 0..100 {
            if let Ok(mut stream) = tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
                stream
                    .write_all(b"GET /api/status HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
                    .await
                    .unwrap();
                let mut response = String::new();
                stream.read_to_string(&mut response).await.unwrap();
                return response.lines().next().unwrap_or_default().to_string();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("maintenance server did not start on port {port}");
    }

    #[tokio::test]
    async fn serves_over_tcp_until_the_shutdown_signal() {
        let (signal, _) = broadcast::channel(1);
        let port = free_port();
        let subscribe = {
            let signal = signal.clone();
            move || signal.subscribe()
        };
        let server = tokio::spawn(async move {
            run_maintenance(
                TcpListener::bind(("127.0.0.1", port)),
                &REINDEX_FAILED,
                Duration::from_millis(10),
                subscribe,
            )
            .await
        });

        assert_eq!(
            get_status_line(port).await,
            "HTTP/1.1 503 Service Unavailable"
        );
        // Reminders keep ticking while the page is served.
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(!server.is_finished());

        signal.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .expect("server stops on the signal")
            .unwrap();
    }

    #[tokio::test]
    async fn stays_alive_when_the_port_is_taken() {
        let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = taken.local_addr().unwrap().port();
        let (signal, _) = broadcast::channel(1);
        let subscribe = {
            let signal = signal.clone();
            move || signal.subscribe()
        };
        let server = tokio::spawn(async move {
            run_maintenance(
                TcpListener::bind(("127.0.0.1", port)),
                &REINDEX_FAILED,
                REMINDER_INTERVAL,
                subscribe,
            )
            .await
        });

        // The bind fails at once, but the process must not exit.
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!server.is_finished());

        // Only the shutdown signal ends it. The fallback subscribes after
        // the failure, so keep signalling until it listens.
        let stopped = tokio::time::timeout(Duration::from_secs(10), async {
            while !server.is_finished() {
                let _ = signal.send(());
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
        assert!(stopped.is_ok(), "server stops on the signal");
        drop(taken);
    }
}
