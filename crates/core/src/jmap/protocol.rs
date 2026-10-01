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

//! JMAP request/response envelope (RFC 8620 §3.2–3.4).
//!
//! A JMAP request is a set of `using` capability URIs plus an ordered list of
//! method calls `[name, arguments, callId]`. The response echoes the call ids
//! and returns `[name, result, callId]` triples. This module models those wire
//! shapes and the method-call builder, including result references (§3.7).

use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::jmap::error::{JmapMethodError, JmapMethodErrorObject};

/// A single method call `[name, arguments, callId]` (an "Invocation", §3.2).
#[derive(Clone, Debug, PartialEq)]
pub struct Invocation {
    pub name: String,
    pub arguments: serde_json::Value,
    pub call_id: String,
}

impl Invocation {
    pub fn new(
        name: impl Into<String>,
        arguments: serde_json::Value,
        call_id: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            arguments,
            call_id: call_id.into(),
        }
    }
}

impl Serialize for Invocation {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(3))?;
        seq.serialize_element(&self.name)?;
        seq.serialize_element(&self.arguments)?;
        seq.serialize_element(&self.call_id)?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Invocation {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (name, arguments, call_id): (String, serde_json::Value, String) =
            Deserialize::deserialize(deserializer)?;
        Ok(Invocation {
            name,
            arguments,
            call_id,
        })
    }
}

/// A JMAP request envelope (RFC 8620 §3.3).
#[derive(Clone, Debug, Serialize)]
pub struct Request {
    #[serde(rename = "using")]
    pub using: Vec<String>,
    #[serde(rename = "methodCalls")]
    pub method_calls: Vec<Invocation>,
    #[serde(rename = "createdIds", skip_serializing_if = "Option::is_none")]
    pub created_ids: Option<serde_json::Value>,
}

impl Request {
    pub fn new(using: Vec<String>, method_calls: Vec<Invocation>) -> Self {
        Self {
            using,
            method_calls,
            created_ids: None,
        }
    }
}

/// A JMAP response envelope (RFC 8620 §3.4).
#[derive(Clone, Debug, Deserialize)]
pub struct Response {
    #[serde(rename = "methodResponses")]
    pub method_responses: Vec<Invocation>,
    #[serde(rename = "sessionState", default)]
    pub session_state: String,
    #[serde(rename = "createdIds", default)]
    pub created_ids: Option<serde_json::Value>,
}

impl Response {
    /// Find the first method response with the given call id.
    pub fn response_for<'a>(&'a self, call_id: &str) -> Option<&'a Invocation> {
        self.method_responses.iter().find(|r| r.call_id == call_id)
    }

    /// Interpret a method response: if the server returned an `error`
    /// invocation (name == "error"), surface it as a typed [`JmapMethodError`];
    /// otherwise hand back the raw result arguments for the caller to parse.
    pub fn result_for(&self, call_id: &str) -> Result<&serde_json::Value, JmapMethodError> {
        match self.response_for(call_id) {
            Some(inv) if inv.name == "error" => {
                let obj: JmapMethodErrorObject =
                    serde_json::from_value(inv.arguments.clone()).unwrap_or_default();
                Err(JmapMethodError::from_type(&obj.error_type))
            }
            Some(inv) => Ok(&inv.arguments),
            None => Err(JmapMethodError::Other(format!(
                "no response for callId '{call_id}'"
            ))),
        }
    }
}

/// Build a result reference (`#name`) pointing at an earlier call's output
/// (RFC 8620 §3.7). Used to chain e.g. `Email/query` → `Email/get`.
///
/// Produces the `{ "resultOf": <callId>, "name": <method>, "path": <path> }`
/// object that a method argument like `#ids` expects.
pub fn result_reference(
    result_of: &str,
    method: &str,
    path: &str,
) -> serde_json::Value {
    serde_json::json!({
        "resultOf": result_of,
        "name": method,
        "path": path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn invocation_serializes_as_triple() {
        let inv = Invocation::new("Mailbox/get", json!({"accountId": "A1"}), "c0");
        let v = serde_json::to_value(&inv).unwrap();
        assert_eq!(v, json!(["Mailbox/get", {"accountId": "A1"}, "c0"]));
    }

    #[test]
    fn invocation_roundtrips() {
        let inv = Invocation::new("Email/get", json!({"ids": ["m1"]}), "c1");
        let s = serde_json::to_string(&inv).unwrap();
        let back: Invocation = serde_json::from_str(&s).unwrap();
        assert_eq!(inv, back);
    }

    #[test]
    fn request_envelope_shape() {
        let req = Request::new(
            vec!["urn:ietf:params:jmap:core".into()],
            vec![Invocation::new("Core/echo", json!({"hello": 1}), "c0")],
        );
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["using"][0], "urn:ietf:params:jmap:core");
        assert_eq!(v["methodCalls"][0][0], "Core/echo");
        assert!(v.get("createdIds").is_none());
    }

    #[test]
    fn response_result_lookup_and_error_mapping() {
        let raw = json!({
            "methodResponses": [
                ["Mailbox/get", {"accountId": "A1", "list": []}, "c0"],
                ["error", {"type": "cannotCalculateChanges"}, "c1"]
            ],
            "sessionState": "s1"
        });
        let resp: Response = serde_json::from_value(raw).unwrap();

        let ok = resp.result_for("c0").unwrap();
        assert_eq!(ok["accountId"], "A1");

        let err = resp.result_for("c1").unwrap_err();
        assert_eq!(err, JmapMethodError::CannotCalculateChanges);

        let missing = resp.result_for("nope").unwrap_err();
        assert!(matches!(missing, JmapMethodError::Other(_)));
    }

    #[test]
    fn result_reference_shape() {
        let r = result_reference("c0", "Email/query", "/ids");
        assert_eq!(r["resultOf"], "c0");
        assert_eq!(r["name"], "Email/query");
        assert_eq!(r["path"], "/ids");
    }
}
