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

//! A minimal scriptable HTTP server for JMAP integration testing.
//!
//! Mirrors the IMAP `mock_server` pattern (raw tokio TCP, no extra deps). It
//! parses just enough HTTP/1.1 to read the request line, headers, and body, then
//! matches on method+path to return a canned JSON response. Routes are matched
//! by `(METHOD, path-substring)` in insertion order; the first match wins.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Clone)]
struct Route {
    method: String,
    path_contains: String,
    status: u16,
    body: String,
    /// If set, only match when the request body contains this substring
    /// (used to distinguish JMAP method calls on the same apiUrl).
    body_contains: Option<String>,
}

/// A captured request (for assertions in tests).
#[derive(Clone, Debug, Default)]
pub struct CapturedRequest {
    pub method: String,
    pub path: String,
    pub authorization: Option<String>,
    pub body: String,
}

pub struct MockJmapServer {
    routes: Vec<Route>,
}

impl Default for MockJmapServer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockJmapServer {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Add a route matching `method` + a path substring, returning `200` + `body`.
    pub fn route(
        mut self,
        method: &str,
        path_contains: &str,
        body: impl Into<String>,
    ) -> Self {
        self.routes.push(Route {
            method: method.to_string(),
            path_contains: path_contains.to_string(),
            status: 200,
            body: body.into(),
            body_contains: None,
        });
        self
    }

    /// Add a route that also requires the request body to contain `body_contains`.
    pub fn route_body(
        mut self,
        method: &str,
        path_contains: &str,
        body_contains: &str,
        body: impl Into<String>,
    ) -> Self {
        self.routes.push(Route {
            method: method.to_string(),
            path_contains: path_contains.to_string(),
            status: 200,
            body: body.into(),
            body_contains: Some(body_contains.to_string()),
        });
        self
    }

    /// Add a route returning a specific HTTP status.
    pub fn route_status(
        mut self,
        method: &str,
        path_contains: &str,
        status: u16,
        body: impl Into<String>,
    ) -> Self {
        self.routes.push(Route {
            method: method.to_string(),
            path_contains: path_contains.to_string(),
            status,
            body: body.into(),
            body_contains: None,
        });
        self
    }

    pub async fn start(self) -> MockJmapServerHandle {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let base_url = format!("http://{}", addr);
        // Substitute the `{BASE}` placeholder in every route body with the real
        // base URL, so a session resource can reference its own apiUrl/download/
        // upload endpoints without the caller knowing the random port in advance.
        let routes: Vec<Route> = self
            .routes
            .into_iter()
            .map(|mut r| {
                r.body = r.body.replace("{BASE}", &base_url);
                r
            })
            .collect();
        let routes = Arc::new(routes);
        let captured: Arc<tokio::sync::Mutex<Vec<CapturedRequest>>> =
            Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let captured_for_task = captured.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let routes = routes.clone();
                        let captured = captured_for_task.clone();
                        tokio::spawn(async move {
                            handle_connection(stream, routes, captured).await;
                        });
                    }
                    Err(_) => break,
                }
            }
        });

        MockJmapServerHandle {
            base_url,
            captured,
        }
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    routes: Arc<Vec<Route>>,
    captured: Arc<tokio::sync::Mutex<Vec<CapturedRequest>>>,
) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];

    // Read until we have headers (\r\n\r\n), then the full body per Content-Length.
    let (head_end, content_length, head_text) = loop {
        match stream.read(&mut tmp).await {
            Ok(0) => return,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
                    let head_text = String::from_utf8_lossy(&buf[..pos]).to_string();
                    let cl = parse_content_length(&head_text);
                    break (pos + 4, cl, head_text);
                }
            }
            Err(_) => return,
        }
    };

    // Ensure we've read the whole body.
    while buf.len() < head_end + content_length {
        match stream.read(&mut tmp).await {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(_) => break,
        }
    }

    let body = String::from_utf8_lossy(&buf[head_end..(head_end + content_length).min(buf.len())])
        .to_string();

    let mut lines = head_text.lines();
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();

    let mut headers: HashMap<String, String> = HashMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }

    captured.lock().await.push(CapturedRequest {
        method: method.clone(),
        path: path.clone(),
        authorization: headers.get("authorization").cloned(),
        body: body.clone(),
    });

    let route = routes.iter().find(|r| {
        r.method.eq_ignore_ascii_case(&method)
            && path.contains(&r.path_contains)
            && r
                .body_contains
                .as_ref()
                .map(|needle| body.contains(needle))
                .unwrap_or(true)
    });

    let (status, resp_body) = match route {
        Some(r) => (r.status, r.body.clone()),
        None => (404, "{\"error\":\"no mock route\"}".to_string()),
    };

    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Status",
    };

    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        resp_body.as_bytes().len(),
        resp_body
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
}

fn parse_content_length(head: &str) -> usize {
    for line in head.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                return v.trim().parse().unwrap_or(0);
            }
        }
    }
    0
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

pub struct MockJmapServerHandle {
    base_url: String,
    captured: Arc<tokio::sync::Mutex<Vec<CapturedRequest>>>,
}

impl MockJmapServerHandle {
    /// Base URL, e.g. `http://127.0.0.1:54321`.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The session resource URL this mock serves at `/jmap/session`.
    pub fn session_url(&self) -> String {
        format!("{}/jmap/session", self.base_url)
    }

    /// All requests captured so far.
    pub async fn captured(&self) -> Vec<CapturedRequest> {
        self.captured.lock().await.clone()
    }
}
