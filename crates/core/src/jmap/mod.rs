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

//! Low-level JMAP protocol client (RFC 8620 core + RFC 8621 mail).
//!
//! This module is the JMAP analogue of [`crate::imap`]: a self-contained,
//! well-tested client over the workspace `reqwest`. It knows nothing about the
//! account model or the sync engine — those are layered on top in later issues.
//!
//! Layout:
//! - [`error`]    — JMAP method/transport error types and mapping.
//! - [`session`]  — Session resource + server limits (RFC 8620 §2).
//! - [`protocol`] — request/response envelope + result references (§3).
//! - [`client`]   — reqwest transport, session fetch, batching, blobs.
//! - [`mail`]     — RFC 8621 Mailbox/Email methods.

pub mod client;
pub mod error;
pub mod mail;
pub mod protocol;
pub mod session;

#[cfg(test)]
pub mod mock_server;
#[cfg(test)]
mod tests;
