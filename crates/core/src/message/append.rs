use crate::{
    raise_error,
    {
        account::migration::AccountModel,
        archive::source::mail_source_for,
        envelope::extractor::reattach_eml_content,
        error::{code::ErrorCode, MailboxdResult},
    },
};
//use poem_openapi::Object;
use serde::{Deserialize, Serialize};

const MAX_RESTORE_COUNT: usize = 100;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct RestoreMessagesRequest {
    /// envelope IDs to restore (max 100)
    pub envelope_ids: Vec<String>,
}

pub async fn restore_emails(account_id: u64, envelope_ids: Vec<String>) -> MailboxdResult<()> {
    if envelope_ids.len() > MAX_RESTORE_COUNT {
        return Err(raise_error!(
            format!(
                "Too many messages to restore: {} (max {})",
                envelope_ids.len(),
                MAX_RESTORE_COUNT
            ),
            ErrorCode::InvalidParameter
        ));
    }

    let account = AccountModel::check_account_exists(account_id)?;
    // Restore is "read from archive → MailSource::append": IMAP via APPEND, JMAP
    // via Email/import (FA-17). `NoSync` accounts have no server to restore into.
    let source = mail_source_for(account.account_type).ok_or_else(|| {
        raise_error!(
            "Account type does not support restore".into(),
            ErrorCode::Incompatible
        )
    })?;

    let mut failed = Vec::new();
    let mut session = source.connect(account_id).await?;
    for envelope_id in envelope_ids {
        let result: MailboxdResult<()> = async {
            let (envelope, eml) = reattach_eml_content(account_id, envelope_id.clone())?;
            if let Some(mailbox_name) = envelope.mailbox_name {
                session.append(&mailbox_name, eml.as_ref()).await?;
            }

            Ok(())
        }
        .await;

        if let Err(err) = result {
            tracing::warn!(
                account_id = account_id,
                message_id = &envelope_id,
                error = ?err,
                "Failed to restore email"
            );
            failed.push(envelope_id);
        }
    }

    if !failed.is_empty() {
        tracing::info!(
            account_id = account_id,
            failed_count = failed.len(),
            failed_message_ids = ?failed,
            "Restore emails finished with partial failures"
        );
    }

    session.logout().await.ok();

    Ok(())
}
