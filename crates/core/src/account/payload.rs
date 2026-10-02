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

use std::str::FromStr;

use crate::account::entity::{ImapConfig, JmapAuthType, JmapConfig};
use crate::account::migration::{
    AccountModel, AccountType, ArchiveRules, ExtractionRules, QuotaWindow,
};
use crate::account::since::{DateSince, RelativeDate};
use crate::error::code::ErrorCode;
use crate::error::MailboxdResult;
use crate::{raise_error, validate_email};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct AccountCreateRequest {
    #[cfg_attr(
        feature = "web-api",
        oai(validator(custom = "crate::common::validator::EmailValidator"))
    )]
    pub email: String,
    pub login_name: Option<String>,
    pub account_name: Option<String>,
    pub imap: Option<ImapConfig>,
    /// JMAP source configuration. Required when `account_type` is `JMAP`,
    /// must be absent otherwise.
    pub jmap: Option<JmapConfig>,
    pub enabled: bool,
    pub date_since: Option<DateSince>,
    pub date_before: Option<RelativeDate>,
    pub account_type: AccountType,
    #[cfg_attr(feature = "web-api", oai(validator(minimum(value = "1"))))]
    pub download_interval_min: Option<i64>,
    #[cfg_attr(
        feature = "web-api",
        oai(validator(minimum(value = "10"), maximum(value = "200")))
    )]
    pub download_batch_size: Option<u32>,
    pub max_email_size_bytes: Option<u64>,
    pub use_dangerous: bool,
    pub pgp_key: Option<String>,
    pub imap_quota_bytes: Option<u64>,
    pub imap_quota_window: Option<QuotaWindow>,
    pub auto_download_new_mailboxes: Option<bool>,
    pub download_schedule: Option<String>,
    /// Email archive filtering rules (Pro feature).
    /// `None` = archive everything (backward compatible).
    pub archive_rules: Option<ArchiveRules>,
    /// Attachment text extraction rules (Pro feature).
    /// `None` = extract everything (backward compatible).
    pub extraction_rules: Option<ExtractionRules>,
}

impl AccountCreateRequest {
    pub fn create_entity(self, user_id: u64) -> MailboxdResult<AccountModel> {
        if self.date_before.is_some() && self.date_since.is_some() {
            return Err(raise_error!(
                "date_before and date_since are mutually exclusive; specify only one time boundary"
                    .into(),
                ErrorCode::InvalidParameter
            ));
        }

        if self.imap_quota_bytes.is_some() ^ self.imap_quota_window.is_some() {
            return Err(raise_error!(
                "Quota bytes and quota window must be provided together or omitted together".into(),
                ErrorCode::InvalidParameter
            ));
        }

        if let Some(date_since) = self.date_since.as_ref() {
            date_since.validate()?;
        }

        if let Some(date_before) = self.date_before.as_ref() {
            date_before.validate_date()?;
        }

        // A config block must match the declared account_type: exactly the
        // right one present, the other absent.
        if !matches!(self.account_type, AccountType::IMAP) && self.imap.is_some() {
            return Err(raise_error!(
                "imap configuration is only allowed for IMAP account type".into(),
                ErrorCode::InvalidParameter
            ));
        }
        if !matches!(self.account_type, AccountType::JMAP) && self.jmap.is_some() {
            return Err(raise_error!(
                "jmap configuration is only allowed for JMAP account type".into(),
                ErrorCode::InvalidParameter
            ));
        }

        match self.account_type {
            AccountType::IMAP => {
                match &self.imap {
                    Some(imap) => Self::validate_request(imap, &self.email)?,
                    None => {
                        return Err(raise_error!(
                            "IMAP configuration is required for IMAP account type".into(),
                            ErrorCode::InvalidParameter
                        ))
                    }
                }
                if self.download_interval_min.is_none() && self.download_schedule.is_none() {
                    return Err(raise_error!(
                        "`sync_interval_min` or `download_schedule` is required for IMAP account type".into(),
                        ErrorCode::InvalidParameter
                    ));
                }
                if let Some(ref schedule) = self.download_schedule {
                    validate_cron_expression(schedule)?;
                }
            }
            AccountType::JMAP => {
                match &self.jmap {
                    Some(jmap) => Self::validate_jmap_request(jmap, &self.email)?,
                    None => {
                        return Err(raise_error!(
                            "JMAP configuration is required for JMAP account type".into(),
                            ErrorCode::InvalidParameter
                        ))
                    }
                }
                if self.download_interval_min.is_none() && self.download_schedule.is_none() {
                    return Err(raise_error!(
                        "`sync_interval_min` or `download_schedule` is required for JMAP account type".into(),
                        ErrorCode::InvalidParameter
                    ));
                }
                if let Some(ref schedule) = self.download_schedule {
                    validate_cron_expression(schedule)?;
                }
            }
            AccountType::NoSync => {}
        }
        if let Some(ref rules) = self.extraction_rules {
            rules.validate().map_err(|e| {
                raise_error!(
                    format!("extraction_rules: {}", e),
                    ErrorCode::InvalidParameter
                )
            })?;
        }
        if let Some(ref rules) = self.archive_rules {
            rules.validate().map_err(|e| {
                raise_error!(format!("archive_rules: {}", e), ErrorCode::InvalidParameter)
            })?;
        }
        Ok(AccountModel::new(user_id, self)?)
    }

    fn validate_request(imap: &ImapConfig, email: &str) -> MailboxdResult<()> {
        imap.auth
            .validate()
            .map_err(|e| raise_error!(e.to_owned(), ErrorCode::InvalidParameter))?;
        validate_email!(email)?;
        Ok(())
    }

    fn validate_jmap_request(jmap: &JmapConfig, email: &str) -> MailboxdResult<()> {
        jmap.auth
            .validate()
            .map_err(|e| raise_error!(e.to_owned(), ErrorCode::InvalidParameter))?;
        // FA-2: a JMAP account is set up via an explicit session URL or via a
        // resolvable email address (autodiscovery). Require at least one.
        if jmap.session_url.is_none() && email.trim().is_empty() {
            return Err(raise_error!(
                "JMAP account requires either a session_url or an email address for autodiscovery"
                    .into(),
                ErrorCode::InvalidParameter
            ));
        }
        validate_email!(email)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct AccountUpdateRequest {
    pub email: Option<String>,
    /// Represents the account activation status.
    ///
    /// If this value is `false`, all account-related resources will be unavailable
    /// and any attempts to access them should return an error indicating the account
    /// is inactive.
    pub enabled: Option<bool>,
    pub account_name: Option<String>,
    /// IMAP server configuration
    pub imap: Option<ImapConfig>,
    /// JMAP source configuration. Only applied to JMAP accounts.
    pub jmap: Option<JmapConfig>,
    /// Controls initial synchronization time range
    ///
    /// When dealing with large mailboxes, this restricts scanning to:
    /// - Messages after specified starting point
    /// - Or within sliding window
    ///
    /// ### Use Cases
    /// - Event-driven systems (only sync recent actionable emails)
    /// - First-time sync optimization for large accounts
    /// - Reducing server load during resyncs
    pub date_since: Option<DateSince>,
    pub date_before: Option<RelativeDate>,
    pub clear_date_range: Option<bool>,
    /// Configuration for selective folder (mailbox/label) synchronization
    ///
    /// - For IMAP/SMTP accounts:
    ///   Stores the mailbox names, since IMAP mailboxes do not have stable IDs.
    ///   Synchronization is keyed by the folder name.
    ///
    /// - For Gmail API accounts:
    ///   A Gmail label is treated as a mailbox (model mapping).
    ///   Since label names can be easily changed, the stable `labelId` is recorded here
    ///   instead of the label name.
    ///
    /// Defaults to standard folders (`INBOX`, `Sent`) if empty.
    /// Modified folders will be automatically synced on the next update.
    pub sync_folders: Option<Vec<String>>,
    /// Incremental download interval (seconds)
    #[cfg_attr(feature = "web-api", oai(validator(minimum(value = "1"))))]
    pub download_interval_min: Option<i64>,
    #[cfg_attr(
        feature = "web-api",
        oai(validator(minimum(value = "10"), maximum(value = "200")))
    )]
    pub download_batch_size: Option<u32>,
    pub max_email_size_bytes: Option<u64>,
    pub use_dangerous: Option<bool>,

    pub pgp_key: Option<String>,
    pub imap_quota_bytes: Option<u64>,
    pub imap_quota_window: Option<QuotaWindow>,
    pub auto_download_new_mailboxes: Option<bool>,
    pub download_schedule: Option<String>,
    pub clear_download_schedule: Option<bool>,
    /// Email archive filtering rules (Pro feature).
    /// `None` = no change. Use `Some(ArchiveRules { .. })` to set.
    pub archive_rules: Option<ArchiveRules>,
    /// Attachment text extraction rules (Pro feature).
    /// `None` = no change. Use `Some(ExtractionRules { .. })` to set.
    pub extraction_rules: Option<ExtractionRules>,
}

impl AccountUpdateRequest {
    pub fn validate_update_request(&self, account: &AccountModel) -> MailboxdResult<()> {
        if self.date_before.is_some() && self.date_since.is_some() {
            return Err(raise_error!(
                "date_before and date_since are mutually exclusive; specify only one time boundary"
                    .into(),
                ErrorCode::InvalidParameter
            ));
        }

        if self.imap_quota_bytes.is_some() ^ self.imap_quota_window.is_some() {
            return Err(raise_error!(
                "Quota bytes and quota window must be provided together or omitted together".into(),
                ErrorCode::InvalidParameter
            ));
        }

        if self.clear_date_range == Some(true)
            && (self.date_since.is_some() || self.date_before.is_some())
        {
            return Err(raise_error!(
                "clear_date_range cannot be combined with date_since or date_before".into(),
                ErrorCode::InvalidParameter
            ));
        }

        if let Some(date_since) = self.date_since.as_ref() {
            date_since.validate()?;
        }

        if let Some(date_before) = self.date_before.as_ref() {
            date_before.validate_date()?;
        }

        if matches!(
            account.account_type,
            AccountType::IMAP | AccountType::JMAP
        ) {
            if let Some(mailboxes) = self.sync_folders.as_ref() {
                if mailboxes.is_empty() {
                    return Err(raise_error!(
                    "Invalid configuration: 'sync_folders' cannot be empty. \
                     If you are modifying the subscription list, please provide at least one mailbox to subscribe to.".into(), ErrorCode::InvalidParameter
                ));
                }
            }
            if self.clear_download_schedule == Some(true) && self.download_schedule.is_some() {
                return Err(raise_error!(
                    "clear_download_schedule cannot be combined with download_schedule".into(),
                    ErrorCode::InvalidParameter
                ));
            }
            if let Some(ref schedule) = self.download_schedule {
                validate_cron_expression(schedule)?;
            }
        }

        // If a jmap config block is supplied on update, validate its structure.
        // Unlike create, a blank secret is allowed on update: it means "keep the
        // currently stored secret" (mirroring how the IMAP update path does not
        // re-require the password). So we only enforce the Basic-auth username
        // here, not the secret.
        if matches!(account.account_type, AccountType::JMAP) {
            if let Some(ref jmap) = self.jmap {
                if matches!(jmap.auth.auth_type, JmapAuthType::Basic)
                    && jmap.auth.username.is_none()
                {
                    return Err(raise_error!(
                        "When auth_type is Basic, username must not be None.".into(),
                        ErrorCode::InvalidParameter
                    ));
                }
            }
        }
        if let Some(ref rules) = self.extraction_rules {
            rules.validate().map_err(|e| {
                raise_error!(
                    format!("extraction_rules: {}", e),
                    ErrorCode::InvalidParameter
                )
            })?;
        }
        if let Some(ref rules) = self.archive_rules {
            rules.validate().map_err(|e| {
                raise_error!(format!("archive_rules: {}", e), ErrorCode::InvalidParameter)
            })?;
        }
        Ok(())
    }
}

fn validate_cron_expression(expr: &str) -> MailboxdResult<()> {
    if expr.trim().is_empty() {
        return Err(raise_error!(
            "download_schedule must not be empty".into(),
            ErrorCode::InvalidParameter
        ));
    }
    cron::Schedule::from_str(expr).map_err(|e| {
        raise_error!(
            format!("Invalid cron expression '{}': {}", expr, e),
            ErrorCode::InvalidParameter
        )
    })?;
    Ok(())
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]

pub struct MinimalAccount {
    pub id: u64,
    pub email: String,
    pub name: Option<String>,
}

pub fn filter_accessible_accounts<'a>(
    all_accounts: &'a [MinimalAccount],
    allowed: &Vec<u64>,
) -> Vec<MinimalAccount> {
    all_accounts
        .iter()
        .filter(|acct| allowed.contains(&acct.id))
        .cloned()
        .collect()
}

#[cfg(test)]
mod test {
    use super::validate_cron_expression;

    #[test]
    fn valid_cron_expressions() {
        assert!(validate_cron_expression("0 0 0 * * *").is_ok()); // daily at midnight
        assert!(validate_cron_expression("0 */5 * * * *").is_ok()); // every 5 minutes
        assert!(validate_cron_expression("0 0 12 * * 1-5").is_ok()); // weekdays at noon
        assert!(validate_cron_expression("0 30 4 1 * *").is_ok()); // 1st of month at 04:30
        assert!(validate_cron_expression("0 0 * * * *").is_ok()); // every hour
    }

    #[test]
    fn invalid_cron_expression_too_few_fields() {
        assert!(validate_cron_expression("0 0 * *").is_err());
        assert!(validate_cron_expression("* * * * *").is_err()); // 5 fields, needs seconds
    }

    #[test]
    fn invalid_cron_expression_empty() {
        assert!(validate_cron_expression("").is_err());
        assert!(validate_cron_expression("   ").is_err());
    }

    #[test]
    fn invalid_cron_expression_garbage() {
        assert!(validate_cron_expression("not a cron").is_err());
    }
}
