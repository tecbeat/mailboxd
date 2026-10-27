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

use crate::error::code::ErrorCode;
use crate::error::MailboxdResult;
use crate::raise_error;
use crate::store::tantivy::attachment::ATTACHMENT_MANAGER;
use crate::store::tantivy::envelope::ENVELOPE_MANAGER;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct DeleteMessagesResult {
    /// Number of messages that existed and were deleted.
    pub deleted: u64,
}

/// Deletes messages keyed by account id. Fails with `ResourceNotFound` when
/// none of the messages exist, e.g. because a mailbox id was used as the key.
pub async fn delete_messages_impl(
    request: HashMap<u64, Vec<String>>,
) -> MailboxdResult<DeleteMessagesResult> {
    let deleted = ENVELOPE_MANAGER
        .delete_envelopes_multi_account(request.clone())
        .await?;
    if deleted == 0 {
        return Err(raise_error!(
            "None of the messages exist. The request must be keyed by account id.".into(),
            ErrorCode::ResourceNotFound
        ));
    }
    ATTACHMENT_MANAGER
        .delete_attachments_multi_account(request)
        .await?;
    Ok(DeleteMessagesResult { deleted })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unknown_ids_are_reported_as_not_found() {
        // Account id used by no other test.
        let request = HashMap::from([(61_332_001, vec!["no-such-envelope".to_string()])]);
        let err = delete_messages_impl(request)
            .await
            .expect_err("nothing matched, so nothing may be reported as deleted");
        assert_eq!(err.code(), ErrorCode::ResourceNotFound);
    }
}
