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

//! Prometheus metrics.
//!
//! Counters and histograms are updated live at the instrumented points.
//! Gauges that need index or directory scans are set by [`refresh`], which
//! the [`refresh::MetricsRefreshTask`] runs on an interval, so a scrape only
//! renders what is already there.
//!
//! Recording is always on and cheap; whether the numbers are exposed is the
//! server's decision (`MAILBOXD_METRICS_ENABLED`).

use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
    time::Duration,
};

use prometheus::{
    Encoder, Gauge, GaugeVec, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    IntGaugeVec, Opts, Registry, TextEncoder,
};

use crate::{
    account::{
        migration::AccountModel,
        state::{DownloadSession, DownloadStatus, TriggerType},
    },
    error::{code::ErrorCode, MailboxdResult},
    ext::event_bus::Event,
    mailboxd_version, raise_error,
    settings::cli::SETTINGS,
    utc_now,
};

pub mod refresh;

pub use refresh::refresh;

/// Content type of the text exposition format.
pub const CONTENT_TYPE: &str = prometheus::TEXT_FORMAT;

const ACCOUNT_LABELS: &[&str] = &["account_id", "account"];
const TRIGGERS: &[&str] = &["scheduled", "manual", "sync_full"];
const DOWNLOAD_RESULTS: &[&str] = &["success", "failed", "cancelled"];
const GAP_FILL_RESULTS: &[&str] = &["downloaded", "failed"];
const LOGIN_METHODS: &[&str] = &["password", "mfa", "oidc"];
const OUTCOMES: &[&str] = &["success", "failure"];

/// All metric handles, registered in one private registry.
pub(crate) struct Metrics {
    registry: Registry,

    pub(crate) http_requests: IntCounterVec,
    pub(crate) http_duration: HistogramVec,
    pub(crate) http_in_flight: IntGauge,

    pub(crate) accounts: IntGaugeVec,
    pub(crate) account_emails: IntGaugeVec,
    pub(crate) account_attachments: IntGaugeVec,
    pub(crate) account_size: IntGaugeVec,
    pub(crate) account_folders: IntGaugeVec,

    pub(crate) download_runs: IntCounterVec,
    pub(crate) download_duration: HistogramVec,
    pub(crate) download_messages: IntCounterVec,
    pub(crate) download_running: IntGaugeVec,
    pub(crate) download_last_success: GaugeVec,
    pub(crate) download_last_failure: GaugeVec,
    pub(crate) gap_fill_messages: IntCounterVec,
    pub(crate) oauth2_token_refreshed: GaugeVec,
    pub(crate) oauth2_refreshes: IntCounterVec,

    pub(crate) storage_bytes: IntGauge,
    pub(crate) index_bytes: IntGaugeVec,
    pub(crate) emails: IntGauge,
    pub(crate) attachments: IntGauge,
    pub(crate) disk_free: IntGaugeVec,
    pub(crate) dedup_removed: IntCounter,

    pub(crate) task_runs: IntCounterVec,
    pub(crate) task_duration: HistogramVec,
    pub(crate) task_last_success: GaugeVec,
    pub(crate) retention_deleted: IntCounter,
    pub(crate) backup_last_success: Gauge,
    pub(crate) backup_size: IntGauge,

    pub(crate) logins: IntCounterVec,
}

/// Register `metric` and hand it back. The names and labels are constants,
/// so a failure here is a programming error.
fn register<M: prometheus::core::Collector + Clone + 'static>(registry: &Registry, metric: M) -> M {
    registry
        .register(Box::new(metric.clone()))
        .expect("metric definitions are valid and unique");
    metric
}

fn opts(name: &str, help: &str) -> Opts {
    Opts::new(name, help)
}

impl Metrics {
    fn new() -> Self {
        let r = Registry::new();

        let build_info = register(
            &r,
            IntGaugeVec::new(
                opts("mailboxd_build_info", "Build information, always 1"),
                &["version"],
            )
            .expect("valid metric"),
        );
        build_info.with_label_values(&[mailboxd_version!()]).set(1);

        let start_time = register(
            &r,
            Gauge::with_opts(opts(
                "mailboxd_start_time_seconds",
                "Start time of the process since the Unix epoch in seconds",
            ))
            .expect("valid metric"),
        );
        start_time.set(utc_now!() as f64 / 1000.0);

        #[cfg(target_os = "linux")]
        r.register(Box::new(
            prometheus::process_collector::ProcessCollector::for_self(),
        ))
        .expect("process collector registers once");

        let int_gauge_vec = |name: &str, help: &str, labels: &[&str]| {
            register(
                &r,
                IntGaugeVec::new(opts(name, help), labels).expect("valid metric"),
            )
        };
        let gauge_vec = |name: &str, help: &str, labels: &[&str]| {
            register(
                &r,
                GaugeVec::new(opts(name, help), labels).expect("valid metric"),
            )
        };
        let counter_vec = |name: &str, help: &str, labels: &[&str]| {
            register(
                &r,
                IntCounterVec::new(opts(name, help), labels).expect("valid metric"),
            )
        };
        let histogram_vec = |name: &str, help: &str, labels: &[&str], buckets: Vec<f64>| {
            register(
                &r,
                HistogramVec::new(HistogramOpts::new(name, help).buckets(buckets), labels)
                    .expect("valid metric"),
            )
        };
        let int_gauge = |name: &str, help: &str| {
            register(
                &r,
                IntGauge::with_opts(opts(name, help)).expect("valid metric"),
            )
        };
        let counter = |name: &str, help: &str| {
            register(
                &r,
                IntCounter::with_opts(opts(name, help)).expect("valid metric"),
            )
        };

        let long_buckets = vec![
            1.0, 5.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0, 3600.0, 7200.0, 21600.0,
        ];

        let metrics = Self {
            http_requests: counter_vec(
                "mailboxd_http_requests_total",
                "HTTP requests by method, route template and status code",
                &["method", "route", "status"],
            ),
            http_duration: histogram_vec(
                "mailboxd_http_request_duration_seconds",
                "HTTP request duration by method and route template",
                &["method", "route"],
                prometheus::DEFAULT_BUCKETS.to_vec(),
            ),
            http_in_flight: int_gauge(
                "mailboxd_http_requests_in_flight",
                "HTTP requests currently being served",
            ),

            accounts: int_gauge_vec(
                "mailboxd_accounts",
                "Accounts by type and enabled flag",
                &["type", "enabled"],
            ),
            account_emails: int_gauge_vec(
                "mailboxd_account_emails",
                "Archived emails of an account",
                ACCOUNT_LABELS,
            ),
            account_attachments: int_gauge_vec(
                "mailboxd_account_attachments",
                "Archived attachments of an account",
                ACCOUNT_LABELS,
            ),
            account_size: int_gauge_vec(
                "mailboxd_account_size_bytes",
                "Total size of the archived emails of an account",
                ACCOUNT_LABELS,
            ),
            account_folders: int_gauge_vec(
                "mailboxd_account_folders",
                "Known folders of an account",
                ACCOUNT_LABELS,
            ),

            download_runs: counter_vec(
                "mailboxd_account_download_runs_total",
                "Finished download sessions by trigger and status",
                &["account_id", "account", "trigger", "status"],
            ),
            download_duration: histogram_vec(
                "mailboxd_account_download_duration_seconds",
                "Duration of finished download sessions",
                ACCOUNT_LABELS,
                long_buckets.clone(),
            ),
            download_messages: counter_vec(
                "mailboxd_account_download_messages_total",
                "Messages stored by finished download sessions",
                ACCOUNT_LABELS,
            ),
            download_running: int_gauge_vec(
                "mailboxd_account_download_running",
                "1 while a download session of the account is running",
                ACCOUNT_LABELS,
            ),
            download_last_success: gauge_vec(
                "mailboxd_account_last_download_success_timestamp_seconds",
                "End time of the last successful download session",
                ACCOUNT_LABELS,
            ),
            download_last_failure: gauge_vec(
                "mailboxd_account_last_download_failure_timestamp_seconds",
                "End time of the last failed download session",
                ACCOUNT_LABELS,
            ),
            gap_fill_messages: counter_vec(
                "mailboxd_account_gap_fill_messages_total",
                "Messages handled by finished gap-fill runs by result",
                &["account_id", "account", "result"],
            ),
            oauth2_token_refreshed: gauge_vec(
                "mailboxd_account_oauth2_token_refreshed_timestamp_seconds",
                "Time the OAuth2 access token of the account was last stored",
                ACCOUNT_LABELS,
            ),
            oauth2_refreshes: counter_vec(
                "mailboxd_oauth2_token_refreshes_total",
                "OAuth2 access token refreshes by result",
                &["result"],
            ),

            storage_bytes: int_gauge(
                "mailboxd_storage_bytes",
                "Size of the blob storage directory",
            ),
            index_bytes: int_gauge_vec(
                "mailboxd_index_bytes",
                "Size of a search index directory",
                &["index"],
            ),
            emails: int_gauge("mailboxd_emails", "Archived emails of all accounts"),
            attachments: int_gauge(
                "mailboxd_attachments",
                "Archived attachments of all accounts",
            ),
            disk_free: int_gauge_vec(
                "mailboxd_disk_free_bytes",
                "Free space on the file system holding a data directory",
                &["dir"],
            ),
            dedup_removed: counter(
                "mailboxd_dedup_removed_total",
                "Duplicate emails removed from the index",
            ),

            task_runs: counter_vec(
                "mailboxd_task_runs_total",
                "Runs of the periodic background tasks by status",
                &["task", "status"],
            ),
            task_duration: histogram_vec(
                "mailboxd_task_duration_seconds",
                "Duration of the periodic background task runs",
                &["task"],
                long_buckets,
            ),
            task_last_success: gauge_vec(
                "mailboxd_task_last_success_timestamp_seconds",
                "End time of the last successful run of a periodic task",
                &["task"],
            ),
            retention_deleted: counter(
                "mailboxd_retention_deleted_messages_total",
                "Messages deleted by the retention sweep",
            ),
            backup_last_success: register(
                &r,
                Gauge::with_opts(opts(
                    "mailboxd_backup_last_success_timestamp_seconds",
                    "Modification time of the newest scheduled backup archive",
                ))
                .expect("valid metric"),
            ),
            backup_size: int_gauge(
                "mailboxd_backup_size_bytes",
                "Size of the newest scheduled backup archive",
            ),

            logins: counter_vec(
                "mailboxd_logins_total",
                "Login attempts by method and result",
                &["method", "result"],
            ),

            registry: r,
        };

        for method in LOGIN_METHODS {
            for result in OUTCOMES {
                metrics.logins.with_label_values(&[method, result]);
            }
        }
        for result in OUTCOMES {
            metrics.oauth2_refreshes.with_label_values(&[result]);
        }
        metrics
    }
}

pub(crate) static METRICS: LazyLock<Metrics> = LazyLock::new(Metrics::new);

/// Email address per account id, as used in the `account` label. Lets the
/// series of an account be removed again with the exact label values they
/// were created with.
static ACCOUNT_EMAILS: LazyLock<RwLock<HashMap<u64, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Render all metrics in the Prometheus text exposition format.
pub fn render() -> MailboxdResult<String> {
    let mut buffer = Vec::new();
    TextEncoder::new()
        .encode(&METRICS.registry.gather(), &mut buffer)
        .map_err(|e| raise_error!(format!("encode metrics: {e}"), ErrorCode::InternalError))?;
    String::from_utf8(buffer)
        .map_err(|e| raise_error!(format!("encode metrics: {e}"), ErrorCode::InternalError))
}

/// The scrape password from `MAILBOXD_METRICS_PASSWORD` or, when that is
/// unset, the file named by `MAILBOXD_METRICS_PASSWORD_FILE`. `None` leaves
/// the endpoint open. A configured but unreadable or empty file is an error,
/// so a broken secret mount never silently opens the endpoint.
pub fn configured_password() -> MailboxdResult<Option<String>> {
    if let Some(password) = SETTINGS
        .mailboxd_metrics_password
        .as_ref()
        .filter(|p| !p.is_empty())
    {
        return Ok(Some(password.clone()));
    }
    let Some(file) = SETTINGS.mailboxd_metrics_password_file.as_ref() else {
        return Ok(None);
    };
    let password = std::fs::read_to_string(file)
        .map_err(|e| {
            raise_error!(
                format!("read MAILBOXD_METRICS_PASSWORD_FILE {file}: {e}"),
                ErrorCode::InternalError
            )
        })?
        .trim()
        .to_string();
    if password.is_empty() {
        return Err(raise_error!(
            format!("MAILBOXD_METRICS_PASSWORD_FILE {file} is empty"),
            ErrorCode::InternalError
        ));
    }
    Ok(Some(password))
}

/// Label values `[account_id, email]` for an account, looked up once and
/// then cached. `None` when the account does not exist (any more).
fn account_labels(account_id: u64) -> Option<[String; 2]> {
    if let Some(email) = ACCOUNT_EMAILS
        .read()
        .expect("account email cache poisoned")
        .get(&account_id)
    {
        return Some([account_id.to_string(), email.clone()]);
    }
    let account = AccountModel::find(account_id).ok().flatten()?;
    Some(remember_account(account_id, &account.email))
}

/// Cache the email of an account. When it changed, the series under the old
/// label are dropped so they do not linger next to the new ones.
fn remember_account(account_id: u64, email: &str) -> [String; 2] {
    let previous = ACCOUNT_EMAILS
        .write()
        .expect("account email cache poisoned")
        .insert(account_id, email.to_string());
    if let Some(previous) = previous.filter(|previous| previous != email) {
        remove_series(&[account_id.to_string(), previous]);
    }
    [account_id.to_string(), email.to_string()]
}

/// Drop every series of an account, e.g. after it was deleted.
pub fn remove_account(account_id: u64) {
    let email = ACCOUNT_EMAILS
        .write()
        .expect("account email cache poisoned")
        .remove(&account_id);
    if let Some(email) = email {
        remove_series(&[account_id.to_string(), email]);
    }
}

/// Account ids that currently have series.
fn known_accounts() -> Vec<u64> {
    ACCOUNT_EMAILS
        .read()
        .expect("account email cache poisoned")
        .keys()
        .copied()
        .collect()
}

fn remove_series(labels: &[String; 2]) {
    let m = &*METRICS;
    let [id, email] = labels;
    let pair = [id.as_str(), email.as_str()];
    // A series that was never created is not an error worth reporting.
    for gauge in [
        &m.account_emails,
        &m.account_attachments,
        &m.account_size,
        &m.account_folders,
        &m.download_running,
    ] {
        let _ = gauge.remove_label_values(&pair);
    }
    for gauge in [
        &m.download_last_success,
        &m.download_last_failure,
        &m.oauth2_token_refreshed,
    ] {
        let _ = gauge.remove_label_values(&pair);
    }
    let _ = m.download_duration.remove_label_values(&pair);
    let _ = m.download_messages.remove_label_values(&pair);
    for trigger in TRIGGERS {
        for status in DOWNLOAD_RESULTS {
            let _ = m.download_runs.remove_label_values(&[
                id.as_str(),
                email.as_str(),
                *trigger,
                *status,
            ]);
        }
    }
    for result in GAP_FILL_RESULTS {
        let _ = m
            .gap_fill_messages
            .remove_label_values(&[id.as_str(), email.as_str(), *result]);
    }
}

fn trigger_label(trigger: &TriggerType) -> &'static str {
    match trigger {
        TriggerType::Scheduled => "scheduled",
        TriggerType::Manual => "manual",
        TriggerType::SyncFull => "sync_full",
    }
}

fn millis_to_seconds(millis: i64) -> f64 {
    millis as f64 / 1000.0
}

/// One HTTP request has been answered.
pub fn record_http_request(method: &str, route: &str, status: u16, duration: Duration) {
    let m = &*METRICS;
    m.http_requests
        .with_label_values(&[method, route, &status.to_string()])
        .inc();
    m.http_duration
        .with_label_values(&[method, route])
        .observe(duration.as_secs_f64());
}

/// Counts a request as in flight until the guard is dropped.
pub struct InFlight(());

impl InFlight {
    pub fn start() -> Self {
        METRICS.http_in_flight.inc();
        Self(())
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        METRICS.http_in_flight.dec();
    }
}

/// An OIDC login finished. Password and MFA logins are counted through
/// their audit events, see [`observe_event`].
pub fn record_oidc_login(success: bool) {
    record_login("oidc", success);
}

fn record_login(method: &str, success: bool) {
    let result = if success { "success" } else { "failure" };
    METRICS.logins.with_label_values(&[method, result]).inc();
}

/// Derive metrics from an emitted audit event.
pub fn observe_event(event: &Event) {
    match event {
        Event::UserLoggedIn { success, .. } => record_login("password", *success),
        Event::MfaLogin { success, .. } => record_login("mfa", *success),
        Event::RetentionPurged { deleted, .. } => METRICS.retention_deleted.inc_by(*deleted),
        _ => {}
    }
}

/// A download session of `account_id` started.
pub fn record_download_started(account_id: u64) {
    if let Some(labels) = account_labels(account_id) {
        METRICS.download_running.with_label_values(&labels).set(1);
    }
}

/// A download session of `account_id` ended with a final status.
pub fn record_download_finished(account_id: u64, session: &DownloadSession) {
    let status = match session.status {
        DownloadStatus::Running => return,
        DownloadStatus::Success => "success",
        DownloadStatus::Failed => "failed",
        DownloadStatus::Cancelled => "cancelled",
    };
    let Some(labels) = account_labels(account_id) else {
        return;
    };
    let [id, email] = &labels;
    let m = &*METRICS;
    m.download_runs
        .with_label_values(&[
            id.as_str(),
            email.as_str(),
            trigger_label(&session.trigger),
            status,
        ])
        .inc();
    m.download_running.with_label_values(&labels).set(0);
    m.download_messages.with_label_values(&labels).inc_by(
        session
            .folder_details
            .values()
            .map(|folder| folder.current)
            .sum(),
    );
    if let Some(end) = session.end_time {
        m.download_duration
            .with_label_values(&labels)
            .observe(millis_to_seconds((end - session.start_time).max(0)));
        match session.status {
            DownloadStatus::Success => m
                .download_last_success
                .with_label_values(&labels)
                .set(millis_to_seconds(end)),
            DownloadStatus::Failed => m
                .download_last_failure
                .with_label_values(&labels)
                .set(millis_to_seconds(end)),
            _ => {}
        }
    }
}

/// A gap-fill run of `account_id` finished.
pub fn record_gap_fill_finished(account_id: u64, downloaded: u64, failed: u64) {
    let Some([id, email]) = account_labels(account_id) else {
        return;
    };
    let m = &*METRICS;
    m.gap_fill_messages
        .with_label_values(&[id.as_str(), email.as_str(), "downloaded"])
        .inc_by(downloaded);
    m.gap_fill_messages
        .with_label_values(&[id.as_str(), email.as_str(), "failed"])
        .inc_by(failed);
}

/// An OAuth2 access token refresh finished.
pub fn record_oauth2_refresh(success: bool) {
    let result = if success { "success" } else { "failure" };
    METRICS.oauth2_refreshes.with_label_values(&[result]).inc();
}

/// The dedup pass removed `removed` duplicate emails.
pub fn record_dedup_removed(removed: u64) {
    METRICS.dedup_removed.inc_by(removed);
}

/// Create the series of a tracked periodic task so it shows up before its
/// first run.
pub fn register_task(task: &str) {
    for status in OUTCOMES {
        METRICS.task_runs.with_label_values(&[task, *status]);
    }
}

/// A run of the periodic task `task` finished.
pub fn record_task_run(task: &str, success: bool, duration: Duration) {
    let m = &*METRICS;
    let status = if success { "success" } else { "failure" };
    m.task_runs.with_label_values(&[task, status]).inc();
    m.task_duration
        .with_label_values(&[task])
        .observe(duration.as_secs_f64());
    if success {
        m.task_last_success
            .with_label_values(&[task])
            .set(millis_to_seconds(utc_now!()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_metric_definitions_register() {
        let text = render().unwrap();
        assert!(text.contains("mailboxd_build_info{version="));
        assert!(text.contains(r#"mailboxd_logins_total{method="oidc",result="failure"} 0"#));
    }

    #[test]
    fn email_change_drops_the_old_series() {
        let id = 9_000_001;
        let old = remember_account(id, "old@example.com");
        METRICS.account_emails.with_label_values(&old).set(3);

        remember_account(id, "new@example.com");

        let text = render().unwrap();
        assert!(!text.contains("old@example.com"), "{text}");
        remove_account(id);
    }
}
