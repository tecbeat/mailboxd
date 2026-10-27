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

use crate::logger::file::setup_file_logger;
use crate::settings::cli::SETTINGS;
use chrono::Local;
use std::process;
use tracing::Level;
use tracing_log::LogTracer;
use tracing_subscriber::fmt::{format::Writer, time::FormatTime, MakeWriter};

mod file;
#[cfg(test)]
mod test_writer;

struct LocalTimer;

impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z"))
    }
}

pub fn initialize_logging() {
    let level = validate_log_level(&SETTINGS.mailboxd_log_level);
    if matches!(level, Level::DEBUG) || matches!(level, Level::TRACE) {
        LogTracer::init().unwrap();
    }
    if SETTINGS.mailboxd_log_to_file {
        setup_file_logger(level).unwrap();
    } else {
        setup_stdout_logger(level).unwrap();
    }
}

fn setup_stdout_logger(level: Level) -> Result<(), tracing::dispatcher::SetGlobalDefaultError> {
    tracing::subscriber::set_global_default(stdout_subscriber(
        level,
        SETTINGS.mailboxd_ansi_logs,
        SETTINGS.mailboxd_json_logs,
        std::io::stdout,
    ))
}

/// The stdout subscriber: one JSON object per event when `json` is set,
/// human-readable lines otherwise.
fn stdout_subscriber<W>(
    level: Level,
    with_ansi: bool,
    json: bool,
    writer: W,
) -> Box<dyn tracing::Subscriber + Send + Sync>
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    if json {
        Box::new(
            tracing_subscriber::fmt()
                .with_max_level(level)
                .with_ansi(with_ansi)
                .with_writer(writer)
                .json()
                .with_timer(LocalTimer)
                .finish(),
        )
    } else {
        let format = tracing_subscriber::fmt::format()
            .with_level(true)
            .with_target(true)
            .with_timer(LocalTimer);

        Box::new(
            tracing_subscriber::fmt()
                .with_max_level(level)
                .with_ansi(with_ansi)
                .with_writer(writer)
                .event_format(format)
                .finish(),
        )
    }
}

fn validate_log_level(value: &String) -> Level {
    match value.parse::<Level>() {
        Ok(level) => level,
        Err(_) => {
            eprintln!(
                "Invalid log level specified. Use one of: error, warn, info, debug, trace. 
                The log level you currently specified is 'mailboxd_log_level'='{}'",
                value
            );
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logger::test_writer::CapturedLogs;

    fn log_through_stdout_subscriber(json: bool) -> Vec<String> {
        let logs = CapturedLogs::default();
        let subscriber = stdout_subscriber(Level::INFO, false, json, logs.clone());
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(account = 7, "stdout log line");
            tracing::debug!("below the configured level");
        });
        logs.lines()
    }

    #[test]
    fn stdout_subscriber_writes_one_json_object_per_event_when_json_is_enabled() {
        let lines = log_through_stdout_subscriber(true);
        assert_eq!(lines.len(), 1, "debug event must be filtered: {lines:?}");
        let event: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(event["level"], "INFO");
        assert_eq!(event["target"], "mailboxd_core::logger::tests");
        assert_eq!(event["fields"]["message"], "stdout log line");
        assert_eq!(event["fields"]["account"], 7);
        assert!(event["timestamp"].is_string());
    }

    #[test]
    fn stdout_subscriber_writes_text_lines_when_json_is_disabled() {
        let lines = log_through_stdout_subscriber(false);
        assert_eq!(lines.len(), 1, "debug event must be filtered: {lines:?}");
        let line = &lines[0];
        assert!(serde_json::from_str::<serde_json::Value>(line).is_err());
        assert!(line.contains(" INFO "), "{line}");
        assert!(line.contains("mailboxd_core::logger::tests:"), "{line}");
        assert!(line.ends_with("stdout log line account=7"), "{line}");
    }
}
