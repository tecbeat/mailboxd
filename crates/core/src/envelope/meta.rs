use serde::{Deserialize, Serialize};

use crate::base64_decode;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MailboxdMetadata {
    pub account_email: Option<String>,
    pub mailbox_name: Option<String>,
    pub tags: Option<Vec<String>>,
}

pub fn parse_mailboxd_metadata(header_value: &str) -> Option<MailboxdMetadata> {
    let decoded = base64_decode!(header_value.trim());
    serde_json::from_slice(&decoded).ok()
}
