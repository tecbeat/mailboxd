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

//! The `/metrics` endpoint and the HTTP request metrics middleware.

use std::{sync::Arc, time::Instant};

use base64::{engine::general_purpose::STANDARD, Engine};
use http::{header, Method, StatusCode};
use mailboxd_core::{
    error::MailboxdResult,
    metrics::{self, InFlight},
    settings::cli::SETTINGS,
};
use poem::{Endpoint, IntoResponse, Middleware, Request, Response, Result};
use subtle::ConstantTimeEq;
use tracing::error;

/// Whether `/metrics` is served and the password it requires.
#[derive(Clone, Debug, Default)]
pub struct MetricsConfig {
    pub enabled: bool,
    /// `None` leaves the endpoint open.
    pub password: Option<String>,
}

impl MetricsConfig {
    pub fn from_settings() -> MailboxdResult<Self> {
        Ok(Self {
            enabled: SETTINGS.mailboxd_metrics_enabled,
            password: metrics::configured_password()?,
        })
    }
}

/// Serves the metrics, or `404` when they are disabled.
pub struct MetricsEndpoint {
    config: MetricsConfig,
}

impl MetricsEndpoint {
    pub fn new(config: MetricsConfig) -> Self {
        Self { config }
    }
}

impl Endpoint for MetricsEndpoint {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        if !self.config.enabled {
            return Ok(StatusCode::NOT_FOUND.into_response());
        }
        if let Some(password) = &self.config.password {
            if !authorized(&req, password) {
                return Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header(
                        header::WWW_AUTHENTICATE,
                        r#"Basic realm="mailboxd metrics""#,
                    )
                    .finish());
            }
        }
        match metrics::render() {
            Ok(text) => Ok(Response::builder()
                .content_type(metrics::CONTENT_TYPE)
                .body(text)),
            Err(e) => {
                error!("Failed to render metrics: {e:#?}");
                Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response())
            }
        }
    }
}

/// Accepts `Bearer <password>` and Basic auth with any username.
fn authorized(req: &Request, password: &str) -> bool {
    let Some(value) = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let (scheme, credentials) = value.split_once(' ').unwrap_or((value, ""));
    let credentials = credentials.trim();
    let supplied = if scheme.eq_ignore_ascii_case("bearer") {
        credentials.as_bytes().to_vec()
    } else if scheme.eq_ignore_ascii_case("basic") {
        let Ok(decoded) = STANDARD.decode(credentials) else {
            return false;
        };
        match decoded.iter().position(|&b| b == b':') {
            Some(colon) => decoded[colon + 1..].to_vec(),
            None => return false,
        }
    } else {
        return false;
    };
    supplied.ct_eq(password.as_bytes()).into()
}

/// Maps request paths to the route templates used as the `route` label, so
/// ids in URLs never become label values.
pub struct RouteTemplates {
    /// OpenAPI path templates split into segments, `None` for a parameter.
    api: Vec<(String, Vec<Option<String>>)>,
}

/// Routes outside the OpenAPI service, matched by prefix.
const PUBLIC_ROUTES: &[&str] = &[
    "/api/login",
    "/api/status",
    "/api/auth/mfa/verify",
    "/api/auth/oidc/login",
    "/api/auth/oidc/callback",
    "/api/auth/oidc/handoff",
    "/api/auth/oidc/config",
    "/oauth2/callback",
    "/metrics",
];

impl RouteTemplates {
    /// Build from the `paths` of an OpenAPI spec, where parameters are
    /// written as `{name}`.
    pub fn from_openapi_spec(spec: &str) -> Self {
        let paths: Vec<String> = serde_json::from_str::<serde_json::Value>(spec)
            .ok()
            .and_then(|spec| {
                spec.get("paths")
                    .and_then(|p| p.as_object())
                    .map(|paths| paths.keys().cloned().collect())
            })
            .unwrap_or_default();
        let api = paths
            .into_iter()
            .map(|template| {
                let segments = template
                    .trim_matches('/')
                    .split('/')
                    .map(|segment| {
                        (!(segment.starts_with('{') && segment.ends_with('}')))
                            .then(|| segment.to_string())
                    })
                    .collect();
                (template, segments)
            })
            .collect();
        Self { api }
    }

    /// The route label for a path relative to the base URL.
    pub fn route_of(&self, path: &str) -> &str {
        if path.starts_with("/api/v1/") {
            let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
            return self
                .api
                .iter()
                .filter(|(_, template)| template.len() == segments.len())
                .find(|(_, template)| {
                    template
                        .iter()
                        .zip(&segments)
                        .all(|(t, s)| t.as_deref().map_or(!s.is_empty(), |t| t == *s))
                })
                .map_or("unknown", |(template, _)| template.as_str());
        }
        if let Some(route) = PUBLIC_ROUTES.iter().find(|route| {
            path.strip_prefix(**route)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
        }) {
            return route;
        }
        if path.starts_with("/api-docs/") {
            return "/api-docs";
        }
        if path.starts_with("/api/") || path == "/api" {
            return "unknown";
        }
        if path.starts_with("/assets/") {
            return "/assets";
        }
        "web"
    }
}

/// Records request count, duration and in-flight requests.
pub struct HttpMetrics {
    routes: Arc<RouteTemplates>,
}

impl HttpMetrics {
    pub fn new(routes: RouteTemplates) -> Self {
        Self {
            routes: Arc::new(routes),
        }
    }
}

impl<E: Endpoint> Middleware<E> for HttpMetrics {
    type Output = HttpMetricsEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        HttpMetricsEndpoint {
            inner: ep,
            routes: self.routes.clone(),
        }
    }
}

pub struct HttpMetricsEndpoint<E> {
    inner: E,
    routes: Arc<RouteTemplates>,
}

/// Unknown methods are folded together to keep the label bounded.
fn method_label(method: &Method) -> &'static str {
    match *method {
        Method::GET => "GET",
        Method::POST => "POST",
        Method::PUT => "PUT",
        Method::DELETE => "DELETE",
        Method::PATCH => "PATCH",
        Method::HEAD => "HEAD",
        Method::OPTIONS => "OPTIONS",
        _ => "OTHER",
    }
}

impl<E: Endpoint> Endpoint for HttpMetricsEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let method = method_label(req.method());
        let route = self.routes.route_of(req.uri().path()).to_string();
        let _in_flight = InFlight::start();
        let started = Instant::now();
        let result = self.inner.call(req).await.map(IntoResponse::into_response);
        let status = match &result {
            Ok(resp) => resp.status(),
            Err(err) => err.status(),
        };
        metrics::record_http_request(method, &route, status.as_u16(), started.elapsed());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn templates() -> RouteTemplates {
        RouteTemplates::from_openapi_spec(
            r#"{"paths": {
                "/api/v1/account/{account_id}": {},
                "/api/v1/accounts": {},
                "/api/v1/message-content/{account_id}/{envelope_id}": {}
            }}"#,
        )
    }

    #[test]
    fn api_paths_map_to_their_template() {
        let t = templates();
        assert_eq!(
            t.route_of("/api/v1/account/42"),
            "/api/v1/account/{account_id}"
        );
        assert_eq!(t.route_of("/api/v1/accounts"), "/api/v1/accounts");
        assert_eq!(
            t.route_of("/api/v1/message-content/1/abc"),
            "/api/v1/message-content/{account_id}/{envelope_id}"
        );
        assert_eq!(t.route_of("/api/v1/nope/1"), "unknown");
    }

    #[test]
    fn other_paths_map_to_fixed_labels() {
        let t = templates();
        assert_eq!(t.route_of("/api/login"), "/api/login");
        assert_eq!(t.route_of("/metrics"), "/metrics");
        assert_eq!(t.route_of("/api-docs/swagger/index.html"), "/api-docs");
        assert_eq!(t.route_of("/api/whatever"), "unknown");
        assert_eq!(t.route_of("/assets/index-abc.js"), "/assets");
        assert_eq!(t.route_of("/settings/accounts/7"), "web");
    }
}
