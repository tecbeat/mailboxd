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

use crate::logger::LocalTimer;
use crate::settings::cli::SETTINGS;
use crate::settings::dir::DATA_DIR_MANAGER;
use std::sync::OnceLock;
use tracing::level_filters::LevelFilter;
use tracing::Level;
use tracing::Subscriber;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

pub static LOG_WORKER_GUARD: OnceLock<Vec<WorkerGuard>> = OnceLock::new();

pub fn setup_file_logger(level: Level) -> Result<(), tracing::dispatcher::SetGlobalDefaultError> {
    let with_ansi = SETTINGS.mailboxd_ansi_logs;

    let (server_nonb, server_guard) = server_log_writer();
    LOG_WORKER_GUARD.set(vec![server_guard]).unwrap();

    let registry = tracing_subscriber::registry().with(LevelFilter::from_level(level));

    // Set the combined subscriber as the global default
    tracing::subscriber::set_global_default(registry.with(server_layer(
        SETTINGS.mailboxd_json_logs,
        with_ansi,
        server_nonb,
    )))
}

/// The server log file layer: one JSON object per event when `json` is set,
/// human-readable lines otherwise.
fn server_layer<S, W>(json: bool, with_ansi: bool, writer: W) -> Box<dyn Layer<S> + Send + Sync>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    if json {
        fmt::layer()
            .json()
            .with_timer(LocalTimer)
            .with_ansi(with_ansi)
            .with_writer(writer)
            .boxed()
    } else {
        fmt::layer()
            .with_timer(LocalTimer)
            .with_ansi(with_ansi)
            .with_level(true)
            .with_writer(writer)
            .with_target(true)
            .boxed()
    }
}

fn server_log_writer() -> (NonBlocking, WorkerGuard) {
    let rolling = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("server")
        .max_log_files(SETTINGS.mailboxd_max_server_log_files)
        .build(DATA_DIR_MANAGER.log_dir.clone())
        .expect("failed to initialize rolling file appender");
    let (nb, wg) = tracing_appender::non_blocking(rolling);
    (nb, wg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logger::test_writer::CapturedLogs;

    fn log_through_server_layer(json: bool) -> Vec<String> {
        let logs = CapturedLogs::default();
        let subscriber = tracing_subscriber::registry()
            .with(LevelFilter::from_level(Level::INFO))
            .with(server_layer(json, false, logs.clone()));
        tracing::subscriber::with_default(subscriber, || {
            tracing::warn!(account = 7, "file log line");
            tracing::debug!("below the configured level");
        });
        logs.lines()
    }

    #[test]
    fn server_layer_writes_one_json_object_per_event_when_json_is_enabled() {
        let lines = log_through_server_layer(true);
        assert_eq!(lines.len(), 1, "debug event must be filtered: {lines:?}");
        let event: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(event["level"], "WARN");
        assert_eq!(event["target"], "mailboxd_core::logger::file::tests");
        assert_eq!(event["fields"]["message"], "file log line");
        assert_eq!(event["fields"]["account"], 7);
        assert!(event["timestamp"].is_string());
    }

    #[test]
    fn server_layer_writes_text_lines_when_json_is_disabled() {
        let lines = log_through_server_layer(false);
        assert_eq!(lines.len(), 1, "debug event must be filtered: {lines:?}");
        let line = &lines[0];
        assert!(serde_json::from_str::<serde_json::Value>(line).is_err());
        assert!(line.contains(" WARN "), "{line}");
        assert!(
            line.contains("mailboxd_core::logger::file::tests:"),
            "{line}"
        );
        assert!(line.ends_with("file log line account=7"), "{line}");
    }
}
